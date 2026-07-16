//! GPU circular dab stamps onto a layer texture.

use bytemuck::{Pod, Zeroable};
use phototux_engine::{BrushParams, Dab};

use crate::GpuContext;

const STAMP_WGSL: &str = r#"
struct Uniforms {
    center: vec2<f32>,
    radius: f32,
    hardness: f32,
    color: vec4<f32>,
    eraser: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // center/radius are in UV space (0..1); hardness softens the edge.
    let dist = distance(in.uv, u.center);
    let inner = u.radius * clamp(u.hardness, 0.0, 0.99);
    let a = 1.0 - smoothstep(inner, u.radius, dist);
    if (a <= 0.001) {
        discard;
    }
    if (u.eraser != 0u) {
        return vec4<f32>(0.0, 0.0, 0.0, a);
    }
    return vec4<f32>(u.color.rgb, u.color.a * a);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct StampUniforms {
    center_x: f32,
    center_y: f32,
    radius_uv: f32,
    hardness: f32,
    color: [f32; 4],
    eraser: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// Packed stamp parameters for a single dab (avoids `too_many_arguments`).
#[derive(Debug, Clone, Copy)]
pub struct StampRequest {
    pub x: f32,
    pub y: f32,
    pub radius_px: f32,
    pub pressure: f32,
    pub params: BrushParams,
}

impl StampRequest {
    pub fn from_dab(dab: Dab, params: BrushParams) -> Self {
        Self {
            x: dab.x,
            y: dab.y,
            radius_px: dab.radius,
            pressure: dab.pressure,
            params,
        }
    }
}

/// Reusable stamp pipeline for dabbing into layer textures.
pub struct BrushStamper {
    pipeline_paint: wgpu::RenderPipeline,
    pipeline_erase: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    /// One uniform buffer slot per dab in a batch; resized as needed.
    uniform_bufs: Vec<wgpu::Buffer>,
    width: u32,
    height: u32,
}

impl BrushStamper {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        let bind_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("stamp-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("stamp-wgsl"),
                source: wgpu::ShaderSource::Wgsl(STAMP_WGSL.into()),
            });

        let pl = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("stamp-pl"),
                bind_group_layouts: &[Some(&bind_layout)],
                immediate_size: 0,
            });

        let make_pipe = |blend: wgpu::BlendState, label: &str| {
            ctx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pl),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: Some(blend),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                })
        };

        // Premultiplied-ish src-over for paint
        let paint_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        // Eraser reduces destination alpha
        let erase_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        Self {
            pipeline_paint: make_pipe(paint_blend, "stamp-paint"),
            pipeline_erase: make_pipe(erase_blend, "stamp-erase"),
            bind_layout,
            uniform_bufs: Vec::new(),
            width: width.max(1),
            height: height.max(1),
        }
    }

    fn ensure_uniform_slots(&mut self, ctx: &GpuContext, count: usize) {
        while self.uniform_bufs.len() < count {
            let buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("stamp-ubo"),
                size: std::mem::size_of::<StampUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.uniform_bufs.push(buf);
        }
    }

    fn uniforms_for(&self, req: StampRequest) -> StampUniforms {
        let w = self.width as f32;
        let h = self.height as f32;
        let max_dim = w.max(h);
        let cx = req.x / w;
        let cy = req.y / h;
        let radius_uv = (req.radius_px * req.pressure.clamp(0.05, 1.0)) / max_dim;
        StampUniforms {
            center_x: cx,
            center_y: cy,
            radius_uv,
            hardness: req.params.hardness.clamp(0.0, 1.0),
            color: req.params.color,
            eraser: u32::from(req.params.eraser),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }

    /// Stamp one dab (convenience wrapper around [`Self::stamp_batch`]).
    pub fn stamp(&mut self, ctx: &GpuContext, target: &wgpu::Texture, request: StampRequest) {
        self.stamp_batch(ctx, target, &[request]);
    }

    /// Stamp many dabs in a single GPU submission.
    pub fn stamp_batch(
        &mut self,
        ctx: &GpuContext,
        target: &wgpu::Texture,
        requests: &[StampRequest],
    ) {
        if requests.is_empty() {
            return;
        }
        self.ensure_uniform_slots(ctx, requests.len());
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("stamp-batch-enc"),
            });

        for (i, req) in requests.iter().copied().enumerate() {
            let u = self.uniforms_for(req);
            let ubo = &self.uniform_bufs[i];
            ctx.queue.write_buffer(ubo, 0, bytemuck::bytes_of(&u));
            let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("stamp-bg"),
                layout: &self.bind_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                }],
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("stamp-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(if req.params.eraser {
                    &self.pipeline_erase
                } else {
                    &self.pipeline_paint
                });
                pass.set_bind_group(0, &bind, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phototux_engine::Dab;

    #[test]
    fn stamp_request_from_dab_preserves_fields() {
        let params = BrushParams {
            size: 12.0,
            hardness: 0.8,
            color: [1.0, 0.0, 0.0, 1.0],
            eraser: false,
        };
        let dab = Dab {
            x: 10.0,
            y: 20.0,
            radius: 8.0,
            pressure: 0.5,
        };
        let req = StampRequest::from_dab(dab, params);
        assert_eq!(req.x, 10.0);
        assert_eq!(req.y, 20.0);
        assert_eq!(req.radius_px, 8.0);
        assert_eq!(req.pressure, 0.5);
        assert!(!req.params.eraser);
    }

    #[test]
    fn stamp_batch_empty_is_noop() {
        let ctx = crate::GpuContext::new().expect("gpu");
        let mut stamper = BrushStamper::new(&ctx, 64, 64);
        let tex = ctx.create_cleared_texture(64, 64, [0.0, 0.0, 0.0, 0.0]);
        stamper.stamp_batch(&ctx, &tex, &[]);
    }
}
