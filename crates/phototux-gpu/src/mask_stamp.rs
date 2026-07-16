//! Grayscale dab stamps onto R8 layer masks.

use bytemuck::{Pod, Zeroable};
use phototux_engine::{BrushParams, Dab};

use crate::GpuContext;
use crate::brush::StampRequest;

const MASK_STAMP_WGSL: &str = r#"
struct Uniforms {
    center: vec2<f32>,
    radius: f32,
    hardness: f32,
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
    let dist = distance(in.uv, u.center);
    let inner = u.radius * clamp(u.hardness, 0.0, 0.99);
    let a = 1.0 - smoothstep(inner, u.radius, dist);
    if (a <= 0.001) {
        discard;
    }
    // Paint → white, erase → black; alpha drives blend strength.
    let v = select(1.0, 0.0, u.eraser != 0u);
    return vec4<f32>(v, 0.0, 0.0, a);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaskStampUniforms {
    center_x: f32,
    center_y: f32,
    radius_uv: f32,
    hardness: f32,
    eraser: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// Stamp grayscale dabs into an R8 mask texture.
pub struct MaskStamper {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    uniform_bufs: Vec<wgpu::Buffer>,
    width: u32,
    height: u32,
}

impl MaskStamper {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        let bind_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("mask-stamp-bgl"),
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
                label: Some("mask-stamp-wgsl"),
                source: wgpu::ShaderSource::Wgsl(MASK_STAMP_WGSL.into()),
            });
        let pl = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("mask-stamp-pl"),
                bind_group_layouts: &[Some(&bind_layout)],
                immediate_size: 0,
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mask-stamp-pipe"),
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
                        format: wgpu::TextureFormat::R8Unorm,
                        blend: Some(wgpu::BlendState {
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
                        }),
                        write_mask: wgpu::ColorWrites::RED,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        Self {
            pipeline,
            bind_layout,
            uniform_bufs: Vec::new(),
            width: width.max(1),
            height: height.max(1),
        }
    }

    fn ensure_uniform_slots(&mut self, ctx: &GpuContext, count: usize) {
        while self.uniform_bufs.len() < count {
            let buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mask-stamp-ubo"),
                size: std::mem::size_of::<MaskStampUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.uniform_bufs.push(buf);
        }
    }

    fn uniforms_for(&self, req: StampRequest) -> MaskStampUniforms {
        let w = self.width as f32;
        let h = self.height as f32;
        let max_dim = w.max(h);
        MaskStampUniforms {
            center_x: req.x / w,
            center_y: req.y / h,
            radius_uv: (req.radius_px * req.pressure.clamp(0.05, 1.0)) / max_dim,
            hardness: req.params.hardness.clamp(0.0, 1.0),
            eraser: u32::from(req.params.eraser),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }

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
                label: Some("mask-stamp-batch-enc"),
            });
        for (i, req) in requests.iter().copied().enumerate() {
            let u = self.uniforms_for(req);
            let ubo = &self.uniform_bufs[i];
            ctx.queue.write_buffer(ubo, 0, bytemuck::bytes_of(&u));
            let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mask-stamp-bg"),
                layout: &self.bind_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                }],
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("mask-stamp-pass"),
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
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
}

impl StampRequest {
    pub fn from_dab_mask(dab: Dab, params: BrushParams) -> Self {
        Self::from_dab(dab, params)
    }
}
