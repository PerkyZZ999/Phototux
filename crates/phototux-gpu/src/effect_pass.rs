//! Fullscreen GPU passes for wave-2 filters and layer styles (Phase 4 close-out).

use bytemuck::{Pod, Zeroable};

use crate::GpuContext;
use crate::blur::SeparableBlur;

const EFFECT_WGSL: &str = r#"
struct EffectUniforms {
    mode: u32,
    p0: f32,
    p1: f32,
    p2: f32,
    color: vec4<f32>,
    offset: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var under_tex: texture_2d<f32>;
@group(0) @binding(3) var<uniform> u: EffectUniforms;

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

fn over(dst: vec4<f32>, src: vec4<f32>) -> vec4<f32> {
    let a = src.a + dst.a * (1.0 - src.a);
    if (a < 1e-5) {
        return vec4<f32>(0.0);
    }
    let rgb = (src.rgb * src.a + dst.rgb * dst.a * (1.0 - src.a)) / a;
    return vec4<f32>(rgb, a);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(f32(textureDimensions(src_tex).x), f32(textureDimensions(src_tex).y));
    let src = textureSample(src_tex, samp, in.uv);
    // 0 = copy
    if (u.mode == 0u) {
        return src;
    }
    // 1 = motion blur (directional box)
    if (u.mode == 1u) {
        let dist = clamp(u.p0, 0.0, 64.0);
        let angle = u.p1 * 0.017453292519943295;
        let dir = vec2<f32>(cos(angle), sin(angle));
        let steps = i32(clamp(ceil(dist), 1.0, 32.0));
        var sum = vec4<f32>(0.0);
        var wsum = 0.0;
        for (var i: i32 = -steps; i <= steps; i = i + 1) {
            let t = f32(i) / f32(steps);
            let uv = in.uv + dir * t * dist / dims;
            sum = sum + textureSample(src_tex, samp, uv);
            wsum = wsum + 1.0;
        }
        return sum / max(wsum, 1.0);
    }
    // 2 = emboss
    if (u.mode == 2u) {
        let strength = clamp(u.p0, 0.0, 4.0);
        let angle = u.p1 * 0.017453292519943295;
        let light = vec2<f32>(cos(angle), sin(angle));
        let px = 1.0 / dims;
        let c00 = textureSample(src_tex, samp, in.uv + vec2<f32>(-px.x, -px.y));
        let c10 = textureSample(src_tex, samp, in.uv + vec2<f32>(0.0, -px.y));
        let c20 = textureSample(src_tex, samp, in.uv + vec2<f32>(px.x, -px.y));
        let c01 = textureSample(src_tex, samp, in.uv + vec2<f32>(-px.x, 0.0));
        let c21 = textureSample(src_tex, samp, in.uv + vec2<f32>(px.x, 0.0));
        let c02 = textureSample(src_tex, samp, in.uv + vec2<f32>(-px.x, px.y));
        let c12 = textureSample(src_tex, samp, in.uv + vec2<f32>(0.0, px.y));
        let c22 = textureSample(src_tex, samp, in.uv + vec2<f32>(px.x, px.y));
        let lum = vec3<f32>(0.299, 0.587, 0.114);
        let tl = dot(c00.rgb, lum);
        let t = dot(c10.rgb, lum);
        let tr = dot(c20.rgb, lum);
        let l = dot(c01.rgb, lum);
        let r = dot(c21.rgb, lum);
        let bl = dot(c02.rgb, lum);
        let b = dot(c12.rgb, lum);
        let br = dot(c22.rgb, lum);
        let gx = -tl - 2.0 * l - bl + tr + 2.0 * r + br;
        let gy = -tl - 2.0 * t - tr + bl + 2.0 * b + br;
        let lit = clamp(0.5 + (gx * light.x + gy * light.y) * strength, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(lit), src.a);
    }
    // 3 = tinted shadow from src alpha at -offset (src = blurred alpha layer)
    if (u.mode == 3u) {
        let uv = in.uv - u.offset / dims;
        let a = textureSample(src_tex, samp, uv).a * clamp(u.p0, 0.0, 1.0);
        return vec4<f32>(u.color.rgb, a * u.color.a);
    }
    // 4 = stroke dilate of alpha, tinted
    if (u.mode == 4u) {
        let radius = i32(clamp(ceil(u.p0), 1.0, 16.0));
        var best = 0.0;
        for (var oy: i32 = -radius; oy <= radius; oy = oy + 1) {
            for (var ox: i32 = -radius; ox <= radius; ox = ox + 1) {
                if (ox * ox + oy * oy > radius * radius) {
                    continue;
                }
                let uv = in.uv + vec2<f32>(f32(ox), f32(oy)) / dims;
                best = max(best, textureSample(src_tex, samp, uv).a);
            }
        }
        // Keep only the ring (dilated minus original).
        let ring = clamp(best - src.a, 0.0, 1.0) * clamp(u.p1, 0.0, 1.0);
        return vec4<f32>(u.color.rgb, ring * u.color.a);
    }
    // 5 = under over src (under_tex below, src on top)
    if (u.mode == 5u) {
        let under = textureSample(under_tex, samp, in.uv);
        return over(under, src);
    }
    // 6 = src over under
    if (u.mode == 6u) {
        let under = textureSample(under_tex, samp, in.uv);
        return over(src, under);
    }
    // 7 = Laplacian sharpen (matches cpu_sharpen_rgba)
    if (u.mode == 7u) {
        let amount = clamp(u.p0, 0.0, 4.0);
        let px = 1.0 / dims;
        let c = src.rgb;
        let l = textureSample(src_tex, samp, in.uv + vec2<f32>(-px.x, 0.0)).rgb;
        let r = textureSample(src_tex, samp, in.uv + vec2<f32>(px.x, 0.0)).rgb;
        let t = textureSample(src_tex, samp, in.uv + vec2<f32>(0.0, -px.y)).rgb;
        let b = textureSample(src_tex, samp, in.uv + vec2<f32>(0.0, px.y)).rgb;
        let lap = 4.0 * c - l - r - t - b;
        return vec4<f32>(clamp(c + amount * lap, vec3<f32>(0.0), vec3<f32>(1.0)), src.a);
    }
    // 8 = color overlay (lerp rgb toward color by opacity × alpha)
    if (u.mode == 8u) {
        let t = clamp(u.p0, 0.0, 1.0) * src.a;
        return vec4<f32>(mix(src.rgb, u.color.rgb, t), src.a);
    }
    return src;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EffectUniformsGpu {
    mode: u32,
    p0: f32,
    p1: f32,
    p2: f32,
    color: [f32; 4],
    offset: [f32; 2],
    _pad: [f32; 2],
}

/// Scratch-chain effect applicator used while packing layer array slices.
pub struct EffectPass {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    scratch_a: wgpu::Texture,
    scratch_b: wgpu::Texture,
    scratch_c: wgpu::Texture,
    black: wgpu::Texture,
}

impl EffectPass {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let bind_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("effect-bgl"),
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
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
                label: Some("effect-wgsl"),
                source: wgpu::ShaderSource::Wgsl(EFFECT_WGSL.into()),
            });
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("effect-pl"),
                bind_group_layouts: &[Some(&bind_layout)],
                immediate_size: 0,
            });
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("effect-pipe"),
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
            label: Some("effect-samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniform_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect-ubo"),
            size: std::mem::size_of::<EffectUniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            bind_layout,
            sampler,
            uniform_buf,
            scratch_a: make_rt(ctx, width, height, "effect-a"),
            scratch_b: make_rt(ctx, width, height, "effect-b"),
            scratch_c: make_rt(ctx, width, height, "effect-c"),
            black: ctx.create_cleared_texture(width, height, [0.0, 0.0, 0.0, 0.0]),
        }
    }

    fn run(
        &self,
        ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::Texture,
        under: &wgpu::Texture,
        dst: &wgpu::Texture,
        uniforms: EffectUniformsGpu,
    ) {
        ctx.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));
        let src_view = src.create_view(&wgpu::TextureViewDescriptor::default());
        let under_view = under.create_view(&wgpu::TextureViewDescriptor::default());
        let dst_view = dst.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("effect-bg"),
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
                    resource: wgpu::BindingResource::TextureView(&under_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.uniform_buf.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("effect-pass"),
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

    /// Apply enabled filters then styles; returns texture to pack into the array.
    pub fn apply_pack(
        &mut self,
        ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        blur: &mut SeparableBlur,
        src: &wgpu::Texture,
        plan: &LayerPackPlan,
    ) -> wgpu::Texture {
        // Current working texture alternates A/B.
        let mut use_a = true;
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: src,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.scratch_a,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: src.width(),
                height: src.height(),
                depth_or_array_layers: 1,
            },
        );

        if plan.gaussian > 0.01 {
            let cur = if use_a {
                &self.scratch_a
            } else {
                &self.scratch_b
            };
            let blurred = blur.blur(ctx, encoder, cur, plan.gaussian).clone();
            let dst = if use_a {
                &self.scratch_b
            } else {
                &self.scratch_a
            };
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &blurred,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: dst,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: src.width(),
                    height: src.height(),
                    depth_or_array_layers: 1,
                },
            );
            use_a = !use_a;
        }

        if let Some((distance, angle)) = plan.motion {
            let (from, to) = if use_a {
                (&self.scratch_a, &self.scratch_b)
            } else {
                (&self.scratch_b, &self.scratch_a)
            };
            self.run(
                ctx,
                encoder,
                from,
                &self.black,
                to,
                EffectUniformsGpu {
                    mode: 1,
                    p0: distance,
                    p1: angle,
                    p2: 0.0,
                    color: [0.0; 4],
                    offset: [0.0; 2],
                    _pad: [0.0; 2],
                },
            );
            use_a = !use_a;
        }

        if let Some((strength, angle)) = plan.emboss {
            let (from, to) = if use_a {
                (&self.scratch_a, &self.scratch_b)
            } else {
                (&self.scratch_b, &self.scratch_a)
            };
            self.run(
                ctx,
                encoder,
                from,
                &self.black,
                to,
                EffectUniformsGpu {
                    mode: 2,
                    p0: strength,
                    p1: angle,
                    p2: 0.0,
                    color: [0.0; 4],
                    offset: [0.0; 2],
                    _pad: [0.0; 2],
                },
            );
            use_a = !use_a;
        }

        if let Some(amount) = plan.sharpen {
            let (from, to) = if use_a {
                (&self.scratch_a, &self.scratch_b)
            } else {
                (&self.scratch_b, &self.scratch_a)
            };
            self.run(
                ctx,
                encoder,
                from,
                &self.black,
                to,
                EffectUniformsGpu {
                    mode: 7,
                    p0: amount,
                    p1: 0.0,
                    p2: 0.0,
                    color: [0.0; 4],
                    offset: [0.0; 2],
                    _pad: [0.0; 2],
                },
            );
            use_a = !use_a;
        }

        let content = if use_a {
            self.scratch_a.clone()
        } else {
            self.scratch_b.clone()
        };

        if let Some(shadow) = plan.drop_shadow {
            let blur_r = shadow.blur.max(0.5);
            let blurred = blur.blur(ctx, encoder, &content, blur_r).clone();
            // Tint offset shadow into scratch_c
            self.run(
                ctx,
                encoder,
                &blurred,
                &self.black,
                &self.scratch_c,
                EffectUniformsGpu {
                    mode: 3,
                    p0: shadow.opacity,
                    p1: 0.0,
                    p2: 0.0,
                    color: shadow.color_rgba,
                    offset: [shadow.offset_x, shadow.offset_y],
                    _pad: [0.0; 2],
                },
            );
            // content over shadow → scratch_a
            self.run(
                ctx,
                encoder,
                &content,
                &self.scratch_c,
                &self.scratch_a,
                EffectUniformsGpu {
                    mode: 5,
                    p0: 0.0,
                    p1: 0.0,
                    p2: 0.0,
                    color: [0.0; 4],
                    offset: [0.0; 2],
                    _pad: [0.0; 2],
                },
            );
            use_a = true;
        }

        if let Some(overlay) = plan.color_overlay {
            let src = if use_a {
                self.scratch_a.clone()
            } else {
                self.scratch_b.clone()
            };
            let dst = if use_a {
                &self.scratch_b
            } else {
                &self.scratch_a
            };
            self.run(
                ctx,
                encoder,
                &src,
                &self.black,
                dst,
                EffectUniformsGpu {
                    mode: 8,
                    p0: overlay.opacity,
                    p1: 0.0,
                    p2: 0.0,
                    color: overlay.color_rgba,
                    offset: [0.0; 2],
                    _pad: [0.0; 2],
                },
            );
            use_a = !use_a;
        }

        if let Some(stroke) = plan.stroke {
            let base = if use_a {
                self.scratch_a.clone()
            } else {
                self.scratch_b.clone()
            };
            // Ring into scratch_c
            self.run(
                ctx,
                encoder,
                &base,
                &self.black,
                &self.scratch_c,
                EffectUniformsGpu {
                    mode: 4,
                    p0: stroke.width,
                    p1: stroke.opacity,
                    p2: 0.0,
                    color: stroke.color_rgba,
                    offset: [0.0; 2],
                    _pad: [0.0; 2],
                },
            );
            // stroke over base → opposite scratch
            let dst = if use_a {
                &self.scratch_b
            } else {
                &self.scratch_a
            };
            // Stroke ring (src) over base (under).
            self.run(
                ctx,
                encoder,
                &self.scratch_c,
                &base,
                dst,
                EffectUniformsGpu {
                    mode: 5,
                    p0: 0.0,
                    p1: 0.0,
                    p2: 0.0,
                    color: [0.0; 4],
                    offset: [0.0; 2],
                    _pad: [0.0; 2],
                },
            );
            use_a = !use_a;
        }

        if use_a {
            self.scratch_a.clone()
        } else {
            self.scratch_b.clone()
        }
    }
}

/// Pre-pack plan derived from layer effects + styles.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerPackPlan {
    pub gaussian: f32,
    pub motion: Option<(f32, f32)>,
    pub emboss: Option<(f32, f32)>,
    /// Sharpen amount when present (`FilterParams::Sharpen`).
    pub sharpen: Option<f32>,
    pub drop_shadow: Option<ShadowPlan>,
    pub color_overlay: Option<ColorOverlayPlan>,
    pub stroke: Option<StrokePlan>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorOverlayPlan {
    pub opacity: f32,
    pub color_rgba: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowPlan {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub opacity: f32,
    pub color_rgba: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokePlan {
    pub width: f32,
    pub opacity: f32,
    pub color_rgba: [f32; 4],
}

impl LayerPackPlan {
    pub fn identity() -> Self {
        Self {
            gaussian: 0.0,
            motion: None,
            emboss: None,
            sharpen: None,
            drop_shadow: None,
            color_overlay: None,
            stroke: None,
        }
    }

    pub fn needs_effects(&self) -> bool {
        self.gaussian > 0.01
            || self.motion.is_some()
            || self.emboss.is_some()
            || self.sharpen.is_some_and(|a| a > 0.001)
            || self.drop_shadow.is_some()
            || self.color_overlay.is_some()
            || self.stroke.is_some()
    }

    pub fn from_layer(layer: &phototux_engine::Layer) -> Self {
        use phototux_engine::{FilterParams, LayerStyle};
        let mut plan = Self::identity();
        for effect in &layer.effects {
            if !effect.enabled {
                continue;
            }
            match effect.params {
                FilterParams::GaussianBlur { radius } if radius > 0.01 => {
                    plan.gaussian = plan.gaussian.max(radius);
                }
                FilterParams::MotionBlur {
                    distance,
                    angle_deg,
                } if distance > 0.01 => {
                    plan.motion = Some((distance, angle_deg));
                }
                FilterParams::Emboss {
                    strength,
                    angle_deg,
                } if strength > 0.01 => {
                    plan.emboss = Some((strength, angle_deg));
                }
                FilterParams::Sharpen { amount } if amount > 0.001 => {
                    plan.sharpen = Some(plan.sharpen.map_or(amount, |a| a.max(amount)));
                }
                _ => {}
            }
        }
        for style in &layer.styles {
            match style {
                LayerStyle::DropShadow {
                    enabled: true,
                    offset_x,
                    offset_y,
                    blur,
                    opacity,
                    color_rgba,
                } => {
                    plan.drop_shadow = Some(ShadowPlan {
                        offset_x: *offset_x,
                        offset_y: *offset_y,
                        blur: *blur,
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                LayerStyle::Stroke {
                    enabled: true,
                    width,
                    opacity,
                    color_rgba,
                } => {
                    plan.stroke = Some(StrokePlan {
                        width: *width,
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                LayerStyle::OuterGlow {
                    enabled: true,
                    radius,
                    opacity,
                    color_rgba,
                } if plan.drop_shadow.is_none() => {
                    plan.drop_shadow = Some(ShadowPlan {
                        offset_x: 0.0,
                        offset_y: 0.0,
                        blur: *radius,
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                LayerStyle::ColorOverlay {
                    enabled: true,
                    opacity,
                    color_rgba,
                } => {
                    plan.color_overlay = Some(ColorOverlayPlan {
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                _ => {}
            }
        }
        plan
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
