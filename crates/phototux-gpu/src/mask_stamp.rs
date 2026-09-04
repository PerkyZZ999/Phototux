//! Grayscale dab stamps onto R8 layer masks.

use crate::pass::{FULLSCREEN_VS, plan_dab_batch};
use bytemuck::{Pod, Zeroable};
use phototux_engine::{BrushParams, Dab};

use crate::GpuContext;
use crate::brush::StampRequest;

/// Fragment stage only; the shared vertex stage is prepended at build time.
const MASK_STAMP_WGSL_FS: &str = r#"
struct Uniforms {
    center: vec2<f32>,
    radius: f32,
    hardness: f32,
    eraser: u32,
    use_selection: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
// The document's selection channel, R8; a 1x1 white stand-in when nothing is
// selected. See the comment in `brush.rs`.
@group(0) @binding(1) var sel_tex: texture_2d<f32>;
@group(0) @binding(2) var sel_samp: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dist = distance(in.uv, u.center);
    let inner = u.radius * clamp(u.hardness, 0.0, 0.99);
    var a = 1.0 - smoothstep(inner, u.radius, dist);
    // A selection bounds painting on a mask as it bounds painting on pixels.
    if (u.use_selection != 0u) {
        a = a * textureSampleLevel(sel_tex, sel_samp, in.uv, 0.0).r;
    }
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
    /// Whether the bound mask is a real selection rather than the 1x1 stand-in.
    use_selection: u32,
    _pad1: u32,
    _pad2: u32,
}

/// Stamp grayscale dabs into an R8 mask texture.
pub struct MaskStamper {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    uniform_bufs: Vec<wgpu::Buffer>,
    /// A 1x1 fully-selected mask, bound when the document has no selection.
    unselected_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
}

impl MaskStamper {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        let bind_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("mask-stamp-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mask-stamp-wgsl"),
                source: wgpu::ShaderSource::Wgsl(
                    format!("{FULLSCREEN_VS}{MASK_STAMP_WGSL_FS}").into(),
                ),
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
        let unselected = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mask-stamp-no-selection"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &unselected,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255_u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        Self {
            pipeline,
            bind_layout,
            uniform_bufs: Vec::new(),
            unselected_view: unselected.create_view(&wgpu::TextureViewDescriptor::default()),
            sampler: ctx.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("mask-stamp-sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
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

    fn uniforms_for(&self, req: StampRequest, use_selection: bool) -> MaskStampUniforms {
        let w = self.width as f32;
        let h = self.height as f32;
        let max_dim = w.max(h);
        MaskStampUniforms {
            center_x: req.x / w,
            center_y: req.y / h,
            // Radius already includes size-pressure from StrokeBuilder.
            radius_uv: req.radius_px / max_dim,
            hardness: req.params.hardness.clamp(0.0, 1.0),
            // A mask carries coverage, not colour, so the retouch modes have
            // nothing to rework there: painting and erasing are the only two.
            eraser: u32::from(req.params.mode == phototux_engine::DabMode::Erase),
            use_selection: u32::from(use_selection),
            _pad1: 0,
            _pad2: 0,
        }
    }

    /// Stamp many dabs into an R8 mask.
    ///
    /// `selection` is the document's selection channel when one is active, and
    /// `None` when nothing is selected — see [`crate::brush::BrushStamper::stamp_batch`].
    pub fn stamp_batch(
        &mut self,
        ctx: &GpuContext,
        target: &wgpu::Texture,
        requests: &[StampRequest],
        selection: Option<&wgpu::TextureView>,
    ) {
        if requests.is_empty() {
            return;
        }
        let drawable = plan_dab_batch(requests, self.width, self.height);
        if drawable.is_empty() {
            return;
        }

        self.ensure_uniform_slots(ctx, drawable.len());
        let binds: Vec<wgpu::BindGroup> = drawable
            .iter()
            .map(|dab| {
                let uniforms = self.uniforms_for(dab.request, selection.is_some());
                let ubo = &self.uniform_bufs[dab.slot];
                ctx.queue
                    .write_buffer(ubo, 0, bytemuck::bytes_of(&uniforms));
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("mask-stamp-bg"),
                    layout: &self.bind_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: ubo.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                selection.unwrap_or(&self.unselected_view),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                })
            })
            .collect();

        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mask-stamp-batch-enc"),
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
            for (dab, bind) in drawable.iter().zip(binds.iter()) {
                pass.set_scissor_rect(
                    dab.scissor.x,
                    dab.scissor.y,
                    dab.scissor.width,
                    dab.scissor.height,
                );
                pass.set_bind_group(0, bind, &[]);
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
