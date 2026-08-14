//! Separable Gaussian blur for nondestructive layer effects.
//!
//! The fragment stage is format-agnostic — it samples `texture_2d<f32>` and
//! writes `vec4<f32>`, so an R8 source reads as `(r, 0, 0, 1)` and an R8 target
//! keeps only the red channel. Only the pipeline's colour target and the two
//! scratch textures pinned this to RGBA, which is why layer masks (stored R8)
//! could not be blurred and mask feathering shipped disabled. They are a
//! parameter now rather than a second shader.

use crate::pass::{FULLSCREEN_VS, make_render_target};
use bytemuck::{Pod, Zeroable};

use crate::GpuContext;

/// Fragment stage only; the shared vertex stage is prepended at build time.
const BLUR_WGSL_FS: &str = r#"
struct BlurUniforms {
    direction: vec2<f32>,
    radius: f32,
    _pad: f32,
};

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var<uniform> u: BlurUniforms;


@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(f32(textureDimensions(src_tex).x), f32(textureDimensions(src_tex).y));
    let radius = clamp(u.radius, 0.0, 64.0);
    if (radius < 0.01) {
        return textureSample(src_tex, samp, in.uv);
    }
    let sigma = max(radius * 0.5, 0.5);
    let two_sigma2 = 2.0 * sigma * sigma;
    let support = i32(ceil(radius));
    var sum = vec4<f32>(0.0);
    var wsum = 0.0;
    for (var i: i32 = -support; i <= support; i = i + 1) {
        let offset = u.direction * f32(i) / dims;
        let w = exp(-(f32(i) * f32(i)) / two_sigma2);
        sum = sum + textureSample(src_tex, samp, in.uv + offset) * w;
        wsum = wsum + w;
    }
    return sum / max(wsum, 1e-5);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BlurUniformsGpu {
    direction: [f32; 2],
    radius: f32,
    _pad: f32,
}

/// GPU resources for horizontal/vertical Gaussian passes.
pub struct SeparableBlur {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    temp_a: wgpu::Texture,
    temp_b: wgpu::Texture,
    format: wgpu::TextureFormat,
}

impl SeparableBlur {
    /// RGBA blur, for layer effects.
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        Self::with_format(ctx, width, height, wgpu::TextureFormat::Rgba8Unorm)
    }

    /// Single-channel blur, for coverage such as layer masks.
    pub fn new_r8(ctx: &GpuContext, width: u32, height: u32) -> Self {
        Self::with_format(ctx, width, height, wgpu::TextureFormat::R8Unorm)
    }

    /// Blur writing `format`, which must match the textures passed to
    /// [`Self::blur`] — a render pass rejects an attachment whose format
    /// differs from the pipeline's.
    pub fn with_format(
        ctx: &GpuContext,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let bind_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blur-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("blur-wgsl"),
                source: wgpu::ShaderSource::Wgsl(format!("{FULLSCREEN_VS}{BLUR_WGSL_FS}").into()),
            });
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blur-pl"),
                bind_group_layouts: &[Some(&bind_layout)],
                immediate_size: 0,
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blur-pipe"),
                layout: Some(&pipeline_layout),
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
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blur-samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let temp_a = make_render_target(ctx, width, height, format, "blur-temp-a");
        let temp_b = make_render_target(ctx, width, height, format, "blur-temp-b");
        Self {
            pipeline,
            bind_layout,
            sampler,
            temp_a,
            temp_b,
            format,
        }
    }

    /// Blur `src` into `self.temp_b` (horizontal then vertical). Returns the result texture.
    pub fn blur(
        &mut self,
        ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::Texture,
        radius: f32,
    ) -> &wgpu::Texture {
        let radius = radius.clamp(0.0, 64.0);
        debug_assert_eq!(
            src.format(),
            self.format,
            "blur source format must match the pipeline's colour target"
        );
        let temp_a = self.temp_a.clone();
        let temp_b = self.temp_b.clone();
        self.pass(ctx, encoder, src, &temp_a, [1.0, 0.0], radius);
        self.pass(ctx, encoder, &temp_a, &temp_b, [0.0, 1.0], radius);
        &self.temp_b
    }

    fn pass(
        &self,
        ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::Texture,
        dst: &wgpu::Texture,
        direction: [f32; 2],
        radius: f32,
    ) {
        // One buffer per pass, with its contents mapped in at creation.
        //
        // A single shared buffer written through `Queue::write_buffer` cannot
        // work here: those writes are staged and flushed when the encoder is
        // submitted, so every pass recorded into one encoder reads whichever
        // value was written last. Both halves of this separable blur therefore
        // ran with the second direction — the blur has only ever blurred along
        // one axis — and two `blur` calls in a frame shared one radius.
        let uniforms = BlurUniformsGpu {
            direction,
            radius,
            _pad: 0.0,
        };
        let uniform_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur-ubo"),
            size: std::mem::size_of::<BlurUniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        if let Ok(mut mapped) = uniform_buf.slice(..).get_mapped_range_mut() {
            mapped.copy_from_slice(bytemuck::bytes_of(&uniforms));
        }
        uniform_buf.unmap();
        let src_view = src.create_view(&wgpu::TextureViewDescriptor::default());
        let dst_view = dst.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur-bg"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blur-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dst_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The R8 instance exists so layer masks can be feathered. A pipeline whose
    /// colour target disagreed with the texture it renders into is rejected at
    /// pass creation, so constructing one is the check that the format actually
    /// threads through.
    #[test]
    fn an_r8_blur_builds_with_a_single_channel_target() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let blur = SeparableBlur::new_r8(&ctx, 8, 8);
        assert_eq!(blur.format, wgpu::TextureFormat::R8Unorm);
    }

    #[test]
    fn the_default_blur_is_still_rgba() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        assert_eq!(
            SeparableBlur::new(&ctx, 8, 8).format,
            wgpu::TextureFormat::Rgba8Unorm
        );
    }

    /// A zero-size document must not produce a zero-extent texture, which wgpu
    /// rejects outright.
    #[test]
    fn a_degenerate_size_is_clamped_up() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let _ = SeparableBlur::new_r8(&ctx, 0, 0);
    }
}
