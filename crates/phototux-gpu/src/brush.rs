//! GPU circular dab stamps onto a layer texture.

use bytemuck::{Pod, Zeroable};
use phototux_engine::BrushParams;

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

/// Reusable stamp pipeline for dabbing into layer textures.
pub struct BrushStamper {
    pipeline_paint: wgpu::RenderPipeline,
    pipeline_erase: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    uniform_buf: wgpu::Buffer,
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

        let uniform_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stamp-ubo"),
            size: std::mem::size_of::<StampUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline_paint: make_pipe(paint_blend, "stamp-paint"),
            pipeline_erase: make_pipe(erase_blend, "stamp-erase"),
            bind_layout,
            uniform_buf,
            width: width.max(1),
            height: height.max(1),
        }
    }

    pub fn stamp(
        &self,
        ctx: &GpuContext,
        target: &wgpu::Texture,
        x: f32,
        y: f32,
        radius_px: f32,
        params: BrushParams,
        pressure: f32,
    ) {
        let w = self.width as f32;
        let h = self.height as f32;
        let max_dim = w.max(h);
        let cx = x / w;
        let cy = y / h;
        let radius_uv = (radius_px * pressure.clamp(0.05, 1.0)) / max_dim;
        let u = StampUniforms {
            center_x: cx,
            center_y: cy,
            radius_uv,
            hardness: params.hardness.clamp(0.0, 1.0),
            color: params.color,
            eraser: u32::from(params.eraser),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        // Fix fragment shader: center is vec2, we packed as center_x/y - shader expects u.center
        // Our struct matches with center as first two floats = vec2 in std140 if aligned.
        // WGSL uniform struct: center: vec2, radius, hardness, color vec4, eraser u32 — need matching layout.

        ctx.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u));

        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stamp-bg"),
            layout: &self.bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform_buf.as_entire_binding(),
            }],
        });

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("stamp-enc"),
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
            pass.set_pipeline(if params.eraser {
                &self.pipeline_erase
            } else {
                &self.pipeline_paint
            });
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
}
