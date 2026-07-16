//! Separable Gaussian blur for nondestructive layer effects.

use bytemuck::{Pod, Zeroable};

use crate::GpuContext;

const BLUR_WGSL: &str = r#"
struct BlurUniforms {
    direction: vec2<f32>,
    radius: f32,
    _pad: f32,
};

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var<uniform> u: BlurUniforms;

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
    uniform_buf: wgpu::Buffer,
    temp_a: wgpu::Texture,
    temp_b: wgpu::Texture,
}

impl SeparableBlur {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
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
                source: wgpu::ShaderSource::Wgsl(BLUR_WGSL.into()),
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
                        format: wgpu::TextureFormat::Rgba8Unorm,
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
        let uniform_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur-ubo"),
            size: std::mem::size_of::<BlurUniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let temp_a = make_rt(ctx, width, height, "blur-temp-a");
        let temp_b = make_rt(ctx, width, height, "blur-temp-b");
        Self {
            pipeline,
            bind_layout,
            sampler,
            uniform_buf,
            temp_a,
            temp_b,
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
        let uniforms = BlurUniformsGpu {
            direction,
            radius,
            _pad: 0.0,
        };
        ctx.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));
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
                    resource: self.uniform_buf.as_entire_binding(),
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

fn make_rt(ctx: &GpuContext, w: u32, h: u32, label: &str) -> wgpu::Texture {
    ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
