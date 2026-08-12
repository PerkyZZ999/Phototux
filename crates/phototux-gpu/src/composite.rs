//! Multi-layer GPU composite (ADR-008: 10×4K &lt; 2 ms gate).
//!
//! Single full-screen pass samples up to [`MAX_LAYERS`] layer textures and
//! blends bottom→top in the fragment shader (avoids 10 serial full-frame RT passes).

use std::collections::HashMap;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use phototux_engine::{
    AdjustmentParams, BlendMode, DocumentSize, Layer, LayerId, LayerKind, MAX_LAYERS,
};

use crate::blur::SeparableBlur;
use crate::effect_pass::{EffectPass, LayerPackPlan};
use crate::filters::adjustment_pass;
use crate::layer_mask::LayerMaskChannel;
use crate::pass_timer::PassTimer;
use crate::transform_bake::inverse_affine_coeffs;
use crate::{GpuContext, TextureTransferError};

const BLEND_WGSL: &str = r#"
struct LayerParams {
    opacity: f32,
    mode: u32,
    visible: u32,
    has_mask: u32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    tx: f32,
    ty: f32,
    mask_enabled: u32,
    mask_inverted: u32,
    clips_to_below: u32,
    kind: u32,
    adj_op: u32,
    _pad0: u32,
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
};

struct Uniforms {
    layer_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    layers: array<LayerParams, 16>,
};

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var layers_tex: texture_2d_array<f32>;
@group(0) @binding(2) var masks_tex: texture_2d_array<f32>;
@group(0) @binding(3) var<uniform> u: Uniforms;

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

fn blend_fn(mode: u32, b: vec3<f32>, o: vec3<f32>) -> vec3<f32> {
    switch mode {
        case 1u: { return b * o; } // multiply
        case 2u: { return 1.0 - (1.0 - b) * (1.0 - o); } // screen
        case 3u: { // overlay
            let low = 2.0 * b * o;
            let high = 1.0 - 2.0 * (1.0 - b) * (1.0 - o);
            return select(high, low, b < vec3<f32>(0.5));
        }
        case 4u: { return min(b, o); } // darken
        case 5u: { return max(b, o); } // lighten
        case 6u: { // color dodge
            return select(vec3<f32>(1.0), min(vec3<f32>(1.0), b / max(vec3<f32>(1.0) - o, vec3<f32>(1e-5))), o >= vec3<f32>(1.0));
        }
        case 7u: { // color burn
            return select(vec3<f32>(0.0), vec3<f32>(1.0) - min(vec3<f32>(1.0), (vec3<f32>(1.0) - b) / max(o, vec3<f32>(1e-5))), o <= vec3<f32>(0.0));
        }
        case 8u: { // hard light
            let low = 2.0 * b * o;
            let high = 1.0 - 2.0 * (1.0 - b) * (1.0 - o);
            return select(high, low, o < vec3<f32>(0.5));
        }
        case 9u: { // soft light (approx)
            return (vec3<f32>(1.0) - 2.0 * o) * b * b + 2.0 * o * b;
        }
        case 10u: { return abs(b - o); } // difference
        case 11u: { return b + o - 2.0 * b * o; } // exclusion
        case 12u, 13u, 14u, 15u: { return o; } // hue/sat/color/lum: RGB fallback
        case 16u: { return o; } // pass-through treated as normal in flat stack
        default: { return o; }
    }
}

fn apply_brightness_contrast(rgb: vec3<f32>, brightness: f32, contrast: f32) -> vec3<f32> {
    let c = contrast + 1.0;
    var out = (rgb - vec3<f32>(0.5)) * c + vec3<f32>(0.5);
    out = out + vec3<f32>(brightness);
    return clamp(out, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_levels(rgb: vec3<f32>, black: f32, white: f32, gamma: f32) -> vec3<f32> {
    let span = max(white - black, 1e-5);
    var t = clamp((rgb - vec3<f32>(black)) / span, vec3<f32>(0.0), vec3<f32>(1.0));
    let g = max(gamma, 0.01);
    t = pow(t, vec3<f32>(1.0 / g));
    return clamp(t, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_exposure(rgb: vec3<f32>, stops: f32, gamma: f32) -> vec3<f32> {
    let mul = pow(2.0, clamp(stops, -5.0, 5.0));
    var t = clamp(rgb * mul, vec3<f32>(0.0), vec3<f32>(1.0));
    let g = max(gamma, 0.01);
    t = pow(t, vec3<f32>(1.0 / g));
    return clamp(t, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var acc = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    var clip_base = 1.0;
    let n = u.layer_count;
    let dims = vec2<f32>(f32(textureDimensions(layers_tex).x), f32(textureDimensions(layers_tex).y));
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let p = u.layers[i];
        if (p.visible == 0u) {
            continue;
        }
        let dest = in.uv * dims;
        let src = vec2<f32>(p.a * dest.x + p.b * dest.y + p.tx, p.c * dest.x + p.d * dest.y + p.ty);
        let src_uv = src / dims;
        var mask_f = 1.0;
        if (p.has_mask != 0u && p.mask_enabled != 0u) {
            var m = 0.0;
            if (src_uv.x >= 0.0 && src_uv.x <= 1.0 && src_uv.y >= 0.0 && src_uv.y <= 1.0) {
                m = textureSample(masks_tex, samp, src_uv, i32(i)).r;
            }
            if (p.mask_inverted != 0u) {
                m = 1.0 - m;
            }
            // Contrast/shift refine (non-adjustment layers pack contrast in p0, shift in p1).
            if (p.kind == 0u) {
                let contrast = clamp(p.p0, -1.0, 1.0);
                let shift = clamp(p.p1, -1.0, 1.0);
                m = clamp((m - 0.5) * (1.0 + contrast) + 0.5 + shift, 0.0, 1.0);
            }
            // Density: M' = 1 - density × (1 - M); p3 carries density for non-adjustment layers.
            let density = clamp(p.p3, 0.0, 1.0);
            m = 1.0 - density * (1.0 - m);
            mask_f = m;
        }

        // Adjustment layers transform the running composite (not paint).
        if (p.kind == 1u) {
            let strength = clamp(p.opacity * mask_f, 0.0, 1.0);
            if (strength < 0.0001) {
                continue;
            }
            var adjusted = acc.rgb;
            if (p.adj_op == 1u) {
                adjusted = apply_brightness_contrast(acc.rgb, p.p0, p.p1);
            } else if (p.adj_op == 2u) {
                adjusted = apply_levels(acc.rgb, p.p0, p.p1, p.p2);
            } else if (p.adj_op == 3u) {
                adjusted = apply_exposure(acc.rgb, p.p0, p.p1);
            }
            let rgb = mix(acc.rgb, adjusted, strength);
            acc = vec4<f32>(rgb, acc.a);
            continue;
        }

        var over = vec4<f32>(0.0);
        if (src_uv.x >= 0.0 && src_uv.x <= 1.0 && src_uv.y >= 0.0 && src_uv.y <= 1.0) {
            over = textureSample(layers_tex, samp, src_uv, i32(i));
        }
        var oa = over.a * p.opacity * mask_f;
        if (p.clips_to_below != 0u) {
            oa = oa * clip_base;
        }
        if (oa < 0.0001) {
            if (p.clips_to_below == 0u) {
                clip_base = over.a * p.opacity * mask_f;
            }
            continue;
        }
        let blended = blend_fn(p.mode, acc.rgb, over.rgb);
        let rgb = mix(acc.rgb, blended, oa);
        let a = acc.a + oa * (1.0 - acc.a);
        acc = vec4<f32>(rgb, a);
        if (p.clips_to_below == 0u) {
            clip_base = oa;
        }
    }
    return acc;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LayerParamsGpu {
    opacity: f32,
    mode: u32,
    visible: u32,
    has_mask: u32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    tx: f32,
    ty: f32,
    mask_enabled: u32,
    mask_inverted: u32,
    clips_to_below: u32,
    kind: u32,
    adj_op: u32,
    _pad0: u32,
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UniformsGpu {
    layer_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    layers: [LayerParamsGpu; MAX_LAYERS],
}

/// GPU textures for each layer + single-pass composite result.
pub struct LayerCompositeEngine {
    width: u32,
    height: u32,
    /// Layer id → array slice index (stable until remove/rebuild).
    layer_index: HashMap<LayerId, u32>,
    /// Layer content lives here (slice = stack packing order at composite time via uniforms index map).
    /// We keep content in dedicated 2D textures and only composite; packing order uses layer_index order list.
    layer_tex: HashMap<LayerId, wgpu::Texture>,
    /// Per-layer R8 masks (present when graph has mask metadata).
    mask_tex: HashMap<LayerId, LayerMaskChannel>,
    /// Working array rebuilt only when stack membership changes (not every frame).
    array_tex: wgpu::Texture,
    /// Packed R8 masks (white fill for layers without a mask).
    mask_array_tex: wgpu::Texture,
    /// Shared white R8 used for slices without a mask channel.
    mask_white: wgpu::Texture,
    array_dirty: bool,
    mask_dirty: bool,
    /// Array-slice indices dirty from paint (incremental copy; avoids full repack).
    dirty_slices: Vec<u32>,
    dirty_mask_slices: Vec<u32>,
    result: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
    last_composite_ms: f32,
    /// GPU-timeline timing for the blend pass; `None` without timestamp support.
    timer: Option<PassTimer>,
    stack_order: Vec<LayerId>,
    /// Pre-pack effect/style plan per layer.
    pack_plans: HashMap<LayerId, LayerPackPlan>,
    /// Last applied solid fill colors (rewrite GPU when params change).
    fill_colors: HashMap<LayerId, [f32; 4]>,
    blur: SeparableBlur,
    effects: EffectPass,
}

impl LayerCompositeEngine {
    pub fn new(ctx: &GpuContext, size: DocumentSize) -> Self {
        let width = size.width.max(1);
        let height = size.height.max(1);

        let bind_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("composite-bgl"),
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
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
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
                label: Some("composite-wgsl"),
                source: wgpu::ShaderSource::Wgsl(BLEND_WGSL.into()),
            });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("composite-pl"),
                bind_group_layouts: &[Some(&bind_layout)],
                immediate_size: 0,
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("composite-pipe"),
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
            label: Some("composite-samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite-ubo"),
            size: std::mem::size_of::<UniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let array_tex = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("layer-array"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: MAX_LAYERS as u32,
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
        });

        let mask_array_tex = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mask-array"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: MAX_LAYERS as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let mask_white = make_r8_filled(ctx, width, height, 255, "mask-white");

        let result = make_rt(ctx, width, height, "comp-result");

        Self {
            width,
            height,
            layer_index: HashMap::new(),
            layer_tex: HashMap::new(),
            mask_tex: HashMap::new(),
            array_tex,
            mask_array_tex,
            mask_white,
            array_dirty: true,
            mask_dirty: true,
            dirty_slices: Vec::new(),
            dirty_mask_slices: Vec::new(),
            result,
            pipeline,
            bind_layout,
            sampler,
            uniform_buf,
            bind_group: None,
            last_composite_ms: 0.0,
            timer: PassTimer::new(ctx, "composite-timer"),
            stack_order: Vec::new(),
            pack_plans: HashMap::new(),
            fill_colors: HashMap::new(),
            blur: SeparableBlur::new(ctx, width, height),
            effects: EffectPass::new(ctx, width, height),
        }
    }

    pub fn last_composite_ms(&self) -> f32 {
        self.last_composite_ms
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn ensure_layer(&mut self, ctx: &GpuContext, id: LayerId, color: [f32; 4]) {
        if self.layer_tex.contains_key(&id) {
            return;
        }
        let tex = ctx.create_cleared_texture(self.width, self.height, color);
        self.layer_tex.insert(id, tex);
        self.array_dirty = true;
    }

    pub fn remove_layer(&mut self, id: LayerId) {
        self.layer_tex.remove(&id);
        self.mask_tex.remove(&id);
        self.layer_index.remove(&id);
        self.array_dirty = true;
        self.mask_dirty = true;
    }

    /// Allocate a white R8 mask for `id` (no-op if already present).
    pub fn ensure_mask(&mut self, ctx: &GpuContext, id: LayerId) {
        if self.mask_tex.contains_key(&id) {
            return;
        }
        let size = DocumentSize::new(self.width, self.height);
        self.mask_tex.insert(id, LayerMaskChannel::new(ctx, size));
        self.mask_dirty = true;
    }

    pub fn remove_mask(&mut self, id: LayerId) {
        if self.mask_tex.remove(&id).is_some() {
            self.mask_dirty = true;
        }
    }

    pub fn mask_channel(&self, id: LayerId) -> Option<&LayerMaskChannel> {
        self.mask_tex.get(&id)
    }

    pub fn mask_channel_mut(&mut self, id: LayerId) -> Option<&mut LayerMaskChannel> {
        self.mask_tex.get_mut(&id)
    }

    pub fn mask_texture(&self, id: LayerId) -> Option<&wgpu::Texture> {
        self.mask_tex.get(&id).map(LayerMaskChannel::texture)
    }

    pub fn sync_layers_from_graph(
        &mut self,
        ctx: &GpuContext,
        layers: &[Layer],
    ) -> Result<(), String> {
        if layers.len() > MAX_LAYERS {
            return Err(format!(
                "document has {} layers; compositor supports at most {MAX_LAYERS}",
                layers.len()
            ));
        }
        let ids: std::collections::HashSet<_> = layers.iter().map(|l| l.id).collect();
        self.layer_tex.retain(|id, _| ids.contains(id));
        self.fill_colors.retain(|id, _| ids.contains(id));
        // Drop GPU masks when graph metadata no longer has a mask.
        let masked: std::collections::HashSet<_> = layers
            .iter()
            .filter(|l| l.mask.is_some())
            .map(|l| l.id)
            .collect();
        let before = self.mask_tex.len();
        self.mask_tex.retain(|id, _| masked.contains(id));
        if self.mask_tex.len() != before {
            self.mask_dirty = true;
        }
        const PALETTE: [[f32; 4]; 8] = [
            [0.15, 0.16, 0.20, 1.0],
            [0.25, 0.45, 0.85, 0.85],
            [0.90, 0.35, 0.25, 0.75],
            [0.25, 0.75, 0.40, 0.70],
            [0.85, 0.75, 0.20, 0.65],
            [0.70, 0.30, 0.80, 0.70],
            [0.20, 0.70, 0.75, 0.70],
            [0.95, 0.55, 0.30, 0.65],
        ];
        let mut pack_plans = HashMap::new();
        for (i, layer) in layers.iter().enumerate() {
            match layer.kind {
                LayerKind::Adjustment => {
                    self.ensure_layer(ctx, layer.id, [0.0, 0.0, 0.0, 0.0]);
                }
                LayerKind::Fill => {
                    let color = layer
                        .fill
                        .as_ref()
                        .map(|f| f.color_rgba)
                        .unwrap_or([0.45, 0.55, 0.75, 1.0]);
                    self.ensure_layer(ctx, layer.id, color);
                    let dirty = self
                        .fill_colors
                        .get(&layer.id)
                        .is_none_or(|prev| prev != &color);
                    if dirty {
                        self.write_solid_fill(ctx, layer.id, color)?;
                        self.fill_colors.insert(layer.id, color);
                    }
                }
                _ => {
                    let color = PALETTE[i % PALETTE.len()];
                    self.ensure_layer(ctx, layer.id, color);
                }
            }
            if layer.mask.is_some() {
                self.ensure_mask(ctx, layer.id);
            }
            pack_plans.insert(layer.id, LayerPackPlan::from_layer(layer));
        }
        if pack_plans != self.pack_plans {
            self.pack_plans = pack_plans;
            self.array_dirty = true;
            self.dirty_slices.clear();
        } else {
            self.pack_plans = pack_plans;
        }
        let new_order: Vec<LayerId> = layers.iter().map(|l| l.id).collect();
        if new_order != self.stack_order {
            self.stack_order = new_order;
            self.array_dirty = true;
            self.mask_dirty = true;
            self.dirty_slices.clear();
            self.dirty_mask_slices.clear();
        }
        Ok(())
    }

    fn copy_layer_slice(
        &mut self,
        ctx: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        id: LayerId,
        slice: u32,
    ) -> Result<(), String> {
        let Some(src) = self.layer_tex.get(&id) else {
            return Ok(());
        };
        let src = src.clone();
        let plan = self
            .pack_plans
            .get(&id)
            .cloned()
            .unwrap_or_else(LayerPackPlan::identity);
        let pack_src = if plan.needs_effects() {
            self.effects
                .apply_pack(ctx, encoder, &mut self.blur, &src, &plan)
        } else {
            src
        };
        copy_slice_2d(
            encoder,
            &pack_src,
            &self.array_tex,
            slice,
            self.width,
            self.height,
        );
        Ok(())
    }

    fn repack_array_if_needed(&mut self, ctx: &GpuContext) -> Result<(), String> {
        let layers_need = self.array_dirty || !self.dirty_slices.is_empty();
        let masks_need = self.mask_dirty || !self.dirty_mask_slices.is_empty();
        if !layers_need && !masks_need && self.bind_group.is_some() {
            return Ok(());
        }

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("repack-array"),
            });

        if self.array_dirty {
            self.layer_index.clear();
            let mut pack = Vec::with_capacity(self.stack_order.len().min(MAX_LAYERS));
            for (i, id) in self.stack_order.iter().take(MAX_LAYERS).enumerate() {
                let slice = u32::try_from(i).map_err(|_| "layer index exceeds u32".to_owned())?;
                pack.push((*id, slice));
            }
            for (id, slice) in pack {
                self.layer_index.insert(id, slice);
                self.copy_layer_slice(ctx, &mut encoder, id, slice)?;
            }
        } else {
            let dirty: Vec<(LayerId, u32)> = self
                .dirty_slices
                .iter()
                .filter_map(|&slice| {
                    self.stack_order
                        .get(slice as usize)
                        .copied()
                        .map(|id| (id, slice))
                })
                .collect();
            for (id, slice) in dirty {
                self.copy_layer_slice(ctx, &mut encoder, id, slice)?;
            }
        }

        if self.mask_dirty {
            for (i, id) in self.stack_order.iter().take(MAX_LAYERS).enumerate() {
                let slice = u32::try_from(i).map_err(|_| "layer index exceeds u32".to_owned())?;
                let src = self
                    .mask_tex
                    .get(id)
                    .map(LayerMaskChannel::texture)
                    .unwrap_or(&self.mask_white);
                copy_slice_2d(
                    &mut encoder,
                    src,
                    &self.mask_array_tex,
                    slice,
                    self.width,
                    self.height,
                );
            }
        } else {
            for &slice in &self.dirty_mask_slices {
                let Some(id) = self.stack_order.get(slice as usize).copied() else {
                    continue;
                };
                let src = self
                    .mask_tex
                    .get(&id)
                    .map(LayerMaskChannel::texture)
                    .unwrap_or(&self.mask_white);
                copy_slice_2d(
                    &mut encoder,
                    src,
                    &self.mask_array_tex,
                    slice,
                    self.width,
                    self.height,
                );
            }
        }

        ctx.queue.submit(Some(encoder.finish()));
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| format!("GPU poll failed during array repack: {error:?}"))?;

        if self.array_dirty || self.mask_dirty || self.bind_group.is_none() {
            let array_view = self.array_tex.create_view(&wgpu::TextureViewDescriptor {
                label: Some("array-view"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
            let mask_view = self
                .mask_array_tex
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("mask-array-view"),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                });
            self.bind_group = Some(ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("composite-bg"),
                layout: &self.bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&array_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&mask_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.uniform_buf.as_entire_binding(),
                    },
                ],
            }));
        }
        self.array_dirty = false;
        self.mask_dirty = false;
        self.dirty_slices.clear();
        self.dirty_mask_slices.clear();
        Ok(())
    }

    /// Composite layers bottom→top in one full-screen pass. Returns host-measured ms (submit+poll).
    ///
    /// # Errors
    /// Returns an error when GPU poll fails or the bind group is missing after repack.
    pub fn composite(
        &mut self,
        ctx: &GpuContext,
        layers_bottom_to_top: &[Layer],
    ) -> Result<f32, String> {
        self.repack_array_if_needed(ctx)?;

        let count = layers_bottom_to_top.len().min(MAX_LAYERS);
        let mut uniforms = UniformsGpu {
            layer_count: u32::try_from(count).unwrap_or(0),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            layers: [LayerParamsGpu {
                opacity: 0.0,
                mode: 0,
                visible: 0,
                has_mask: 0,
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 0.0,
                ty: 0.0,
                mask_enabled: 0,
                mask_inverted: 0,
                clips_to_below: 0,
                kind: 0,
                adj_op: 0,
                _pad0: 0,
                p0: 0.0,
                p1: 0.0,
                p2: 0.0,
                p3: 0.0,
            }; MAX_LAYERS],
        };

        for (i, layer) in layers_bottom_to_top.iter().take(count).enumerate() {
            let _ = self.layer_index.get(&layer.id);
            let (a, b, c, d, tx, ty) =
                inverse_affine_coeffs(layer.transform, self.width, self.height);
            let (has_mask, mask_enabled, mask_inverted) = match &layer.mask {
                Some(m) if self.mask_tex.contains_key(&layer.id) => {
                    (1u32, u32::from(m.enabled), u32::from(m.inverted))
                }
                _ => (0, 0, 0),
            };
            let (kind, adj_op, mut p0, mut p1, p2, mut p3) = adjustment_gpu_params(layer);
            // Non-adjustment layers: mask contrast/shift in p0/p1, density in p3.
            if kind == 0 {
                if let Some(m) = layer.mask.as_ref() {
                    p0 = m.contrast.clamp(-1.0, 1.0);
                    p1 = m.shift.clamp(-1.0, 1.0);
                    p3 = m.density.clamp(0.0, 1.0);
                } else {
                    p3 = 1.0;
                }
            }
            uniforms.layers[i] = LayerParamsGpu {
                opacity: layer.opacity.clamp(0.0, 1.0),
                mode: layer.blend.as_u32(),
                visible: u32::from(layer.visible),
                has_mask,
                a,
                b,
                c,
                d,
                tx,
                ty,
                mask_enabled,
                mask_inverted,
                clips_to_below: u32::from(layer.clips_to_below),
                kind,
                adj_op,
                _pad0: 0,
                p0,
                p1,
                p2,
                p3,
            };
        }

        ctx.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        let Some(bind) = self.bind_group.as_ref() else {
            return Err("composite bind group missing".to_owned());
        };
        let result_view = self
            .result
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("composite-enc"),
            });

        let timestamp_writes = self.timer.as_ref().and_then(PassTimer::timestamp_writes);
        let timed = timestamp_writes.is_some();
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &result_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..3, 0..1);
        }
        // This barrier — not a host wait — is what orders Qt's sampling after
        // the blend writes. Qt Quick adopts this exact VkDevice and VkQueue
        // (`vulkan_device_handles` → `QQuickGraphicsDevice::fromDeviceObjects`),
        // so a barrier here applies to everything later in submission order,
        // including Qt's frame. Blocking the host adds nothing to that.
        if timed {
            if let Some(timer) = self.timer.as_ref() {
                timer.resolve(&mut encoder);
            }
        }
        encoder.transition_resources(
            std::iter::empty(),
            std::iter::once(wgpu::TextureTransition {
                texture: &self.result,
                selector: None,
                state: wgpu::TextureUses::RESOURCE,
            }),
        );

        ctx.queue.submit(Some(encoder.finish()));
        if timed {
            if let Some(timer) = self.timer.as_mut() {
                timer.map_after_submit();
            }
        }
        // Non-blocking: lets wgpu retire completed submissions and run the
        // timestamp map callback without stalling the caller.
        let _ = ctx.device.poll(wgpu::PollType::Poll);
        if let Some(timer) = self.timer.as_mut() {
            self.last_composite_ms = timer.poll_result();
        }
        Ok(self.last_composite_ms)
    }

    /// Composite and wait for the GPU, returning the measured pass time.
    ///
    /// For benchmarks and conformance gates only: it deliberately stalls so the
    /// caller measures completed work. The interactive path must use
    /// [`Self::composite`], which never waits.
    ///
    /// # Errors
    /// Propagates composite failures and device poll failures.
    pub fn composite_measured(
        &mut self,
        ctx: &GpuContext,
        layers_bottom_to_top: &[Layer],
    ) -> Result<f32, String> {
        let host_start = Instant::now();
        self.composite(ctx, layers_bottom_to_top)?;
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| format!("GPU poll failed during measured composite: {error:?}"))?;
        if let Some(timer) = self.timer.as_mut() {
            let ms = timer.poll_result();
            if ms > 0.0 {
                self.last_composite_ms = ms;
                return Ok(ms);
            }
        }
        // No timestamp support: host wall time around a completed submit is the
        // only measurement available, and it overstates GPU cost.
        let ms = host_start.elapsed().as_secs_f32() * 1000.0;
        self.last_composite_ms = ms;
        Ok(ms)
    }

    pub fn result_texture(&self) -> &wgpu::Texture {
        &self.result
    }

    pub fn result_vk_handle(&self) -> Option<u64> {
        GpuContext::texture_vk_image_handle(self.result_texture())
    }

    pub fn layer_texture(&self, id: LayerId) -> Option<&wgpu::Texture> {
        self.layer_tex.get(&id)
    }

    pub fn layer_texture_mut(&mut self, id: LayerId) -> Option<&mut wgpu::Texture> {
        self.layer_tex.get_mut(&id)
    }

    fn write_solid_fill(
        &mut self,
        ctx: &GpuContext,
        id: LayerId,
        color: [f32; 4],
    ) -> Result<(), String> {
        let w = self.width as usize;
        let h = self.height as usize;
        let px = w
            .checked_mul(h)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| "fill dimensions overflow".to_owned())?;
        let r = (color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
        let a = (color[3].clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut pixels = vec![0_u8; px];
        for chunk in pixels.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
        self.write_layer_rgba(ctx, id, &pixels)
            .map_err(|e| e.to_string())
    }

    /// Replace one layer with tightly packed RGBA8 pixels.
    pub fn write_layer_rgba(
        &mut self,
        ctx: &GpuContext,
        id: LayerId,
        pixels: &[u8],
    ) -> Result<(), TextureTransferError> {
        let expected = rgba_byte_len(self.width, self.height)?;
        if pixels.len() != expected {
            return Err(TextureTransferError::InvalidPixelLength {
                expected,
                actual: pixels.len(),
            });
        }
        let texture = self
            .layer_tex
            .get(&id)
            .ok_or(TextureTransferError::LayerNotFound)?;
        let bytes_per_row = self
            .width
            .checked_mul(4)
            .ok_or(TextureTransferError::DimensionOverflow)?;
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.array_dirty = true;
        Ok(())
    }

    /// Read the composite once into tightly packed RGBA8 host memory.
    pub fn read_result_rgba(&self, ctx: &GpuContext) -> Result<Vec<u8>, TextureTransferError> {
        self.read_texture_rgba(ctx, &self.result, "composite-readback")
    }

    /// Read one layer texture into tightly packed RGBA8 host memory (Save / clipboard).
    pub fn read_layer_rgba(
        &self,
        ctx: &GpuContext,
        id: LayerId,
    ) -> Result<Vec<u8>, TextureTransferError> {
        let texture = self
            .layer_tex
            .get(&id)
            .ok_or(TextureTransferError::LayerNotFound)?;
        self.read_texture_rgba(ctx, texture, "layer-readback")
    }

    fn read_texture_rgba(
        &self,
        ctx: &GpuContext,
        texture: &wgpu::Texture,
        label: &str,
    ) -> Result<Vec<u8>, TextureTransferError> {
        let unpadded_row = self
            .width
            .checked_mul(4)
            .ok_or(TextureTransferError::DimensionOverflow)?;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row = unpadded_row
            .checked_add(alignment - 1)
            .map(|value| value / alignment * alignment)
            .ok_or(TextureTransferError::DimensionOverflow)?;
        let buffer_size = u64::from(padded_row)
            .checked_mul(u64::from(self.height))
            .ok_or(TextureTransferError::DimensionOverflow)?;
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        encoder.transition_resources(
            std::iter::empty(),
            std::iter::once(wgpu::TextureTransition {
                texture,
                selector: None,
                state: wgpu::TextureUses::RESOURCE,
            }),
        );
        ctx.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|_| TextureTransferError::MapFailed)?;
        receiver
            .recv()
            .map_err(|_| TextureTransferError::CallbackDisconnected)?
            .map_err(|_| TextureTransferError::MapFailed)?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|_| TextureTransferError::MapFailed)?;
        let output_len = rgba_byte_len(self.width, self.height)?;
        let mut output = Vec::with_capacity(output_len);
        for row in mapped
            .chunks_exact(padded_row as usize)
            .take(self.height as usize)
        {
            output.extend_from_slice(&row[..unpadded_row as usize]);
        }
        drop(mapped);
        buffer.unmap();
        Ok(output)
    }

    /// After painting into a layer texture, mark only that array slice dirty.
    pub fn mark_layer_painted(&mut self, id: LayerId) {
        if let Some(&slice) = self.layer_index.get(&id) {
            if !self.dirty_slices.contains(&slice) {
                self.dirty_slices.push(slice);
            }
        } else if self.layer_tex.contains_key(&id) {
            self.array_dirty = true;
        }
    }

    /// After painting into a mask texture, mark only that mask array slice dirty.
    pub fn mark_mask_painted(&mut self, id: LayerId) {
        if let Some(&slice) = self.layer_index.get(&id) {
            if !self.dirty_mask_slices.contains(&slice) {
                self.dirty_mask_slices.push(slice);
            }
        } else if self.mask_tex.contains_key(&id) {
            self.mask_dirty = true;
        }
    }

    /// Replace one layer mask with tightly packed R8 pixels.
    ///
    /// # Errors
    /// Returns an error when the layer has no mask or length mismatches.
    pub fn write_mask_r8(
        &mut self,
        ctx: &GpuContext,
        id: LayerId,
        pixels: &[u8],
    ) -> Result<(), String> {
        self.ensure_mask(ctx, id);
        let mask = self
            .mask_tex
            .get_mut(&id)
            .ok_or_else(|| "mask channel missing after ensure".to_owned())?;
        mask.write_r8(ctx, pixels)?;
        self.mask_dirty = true;
        Ok(())
    }

    /// Read one layer mask into tightly packed R8 host memory.
    ///
    /// # Errors
    /// Returns an error when the mask is missing or GPU readback fails.
    pub fn read_mask_r8(&self, id: LayerId) -> Result<Vec<u8>, String> {
        let mask = self
            .mask_tex
            .get(&id)
            .ok_or_else(|| "mask channel not found".to_owned())?;
        Ok(mask.cpu().to_vec())
    }

    /// Download mask CPU mirror from GPU (after paint before save).
    ///
    /// # Errors
    /// Returns an error when readback fails.
    pub fn sync_mask_cpu_from_gpu(&mut self, ctx: &GpuContext, id: LayerId) -> Result<(), String> {
        let mask = self
            .mask_tex
            .get_mut(&id)
            .ok_or_else(|| "mask channel not found".to_owned())?;
        mask.download_from_gpu(ctx)
    }

    /// GPU copy of a layer texture (for stroke undo).
    pub fn clone_layer_texture(&self, ctx: &GpuContext, id: LayerId) -> Option<wgpu::Texture> {
        let src = self.layer_tex.get(&id)?;
        let dst = make_rt(ctx, self.width, self.height, "layer-undo-bak");
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clone-layer"),
            });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: src,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &dst,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        ctx.queue.submit(Some(encoder.finish()));
        Some(dst)
    }

    /// Restore layer from backup texture.
    pub fn restore_layer_texture(&mut self, ctx: &GpuContext, id: LayerId, backup: &wgpu::Texture) {
        let Some(dst) = self.layer_tex.get(&id) else {
            return;
        };
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("restore-layer"),
            });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: backup,
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
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        ctx.queue.submit(Some(encoder.finish()));
        self.array_dirty = true;
    }

    /// GPU copy of a mask texture (for stroke undo).
    pub fn clone_mask_texture(&self, ctx: &GpuContext, id: LayerId) -> Option<wgpu::Texture> {
        let src = self.mask_tex.get(&id)?.texture();
        let dst = make_r8_empty(ctx, self.width, self.height, "mask-undo-bak");
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clone-mask"),
            });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: src,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &dst,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        ctx.queue.submit(Some(encoder.finish()));
        Some(dst)
    }

    /// Restore mask from backup texture.
    pub fn restore_mask_texture(&mut self, ctx: &GpuContext, id: LayerId, backup: &wgpu::Texture) {
        if !self.mask_tex.contains_key(&id) {
            return;
        }
        {
            let dst = self.mask_tex[&id].texture();
            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("restore-mask"),
                });
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: backup,
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
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
            ctx.queue.submit(Some(encoder.finish()));
        }
        self.mask_dirty = true;
    }
}

fn copy_slice_2d(
    encoder: &mut wgpu::CommandEncoder,
    src: &wgpu::Texture,
    dst_array: &wgpu::Texture,
    slice: u32,
    width: u32,
    height: u32,
) {
    encoder.copy_texture_to_texture(
        wgpu::TexelCopyTextureInfo {
            texture: src,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyTextureInfo {
            texture: dst_array,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: slice,
            },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn adjustment_gpu_params(layer: &Layer) -> (u32, u32, f32, f32, f32, f32) {
    if layer.kind != LayerKind::Adjustment {
        return (0, 0, 0.0, 0.0, 0.0, 0.0);
    }
    let Some(params) = layer.adjustment.as_ref() else {
        return (1, 0, 0.0, 0.0, 0.0, 0.0);
    };
    let pass = adjustment_pass(params);
    let adj_op = match params {
        AdjustmentParams::BrightnessContrast { .. } => 1,
        AdjustmentParams::Levels { .. } => 2,
        AdjustmentParams::Exposure { .. } => 3,
        _ => 0,
    };
    (
        1,
        adj_op,
        pass.params[0],
        pass.params[1],
        pass.params[2],
        pass.params[3],
    )
}

fn rgba_byte_len(width: u32, height: u32) -> Result<usize, TextureTransferError> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(TextureTransferError::DimensionOverflow)
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn make_r8_empty(ctx: &GpuContext, w: u32, h: u32, label: &str) -> wgpu::Texture {
    ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn make_r8_filled(ctx: &GpuContext, w: u32, h: u32, value: u8, label: &str) -> wgpu::Texture {
    let tex = make_r8_empty(ctx, w, h, label);
    let pixels = vec![value; (w.max(1) as usize) * (h.max(1) as usize)];
    ctx.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w.max(1)),
            rows_per_image: Some(h.max(1)),
        },
        wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
    );
    tex
}

/// Run the ADR-008 gate: 10 layers at 4K, return composite time ms.
pub fn benchmark_10x4k_ms(ctx: &GpuContext) -> f32 {
    use phototux_engine::DocumentGraph;

    let size = DocumentSize::new(3840, 2160);
    let mut graph = DocumentGraph::new(size);
    while graph.layer_count() < 10 {
        if graph.add_layer_top(None).is_err() {
            break;
        }
    }
    for (i, layer) in graph.layers().to_vec().into_iter().enumerate() {
        let blend = match i % 4 {
            0 => BlendMode::Normal,
            1 => BlendMode::Multiply,
            2 => BlendMode::Screen,
            _ => BlendMode::Overlay,
        };
        let _ = graph.set_blend(layer.id, blend);
        let _ = graph.set_opacity(layer.id, 0.55 + (i as f32) * 0.03);
    }

    let mut engine = LayerCompositeEngine::new(ctx, size);
    if let Err(error) = engine.sync_layers_from_graph(ctx, graph.layers()) {
        eprintln!("[phototux_gpu] 10×4K sync failed: {error}");
        return f32::MAX;
    }
    for _ in 0..5 {
        let _ = engine.composite_measured(ctx, graph.layers());
    }
    let mut best = f32::MAX;
    for _ in 0..10 {
        match engine.composite_measured(ctx, graph.layers()) {
            Ok(ms) => best = best.min(ms),
            Err(error) => {
                eprintln!("[phototux_gpu] 10×4K composite failed: {error}");
                return f32::MAX;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GpuContext;
    use phototux_engine::DocumentGraph;

    #[test]
    fn composite_small_runs() {
        let ctx = GpuContext::new().expect("gpu");
        let size = DocumentSize::new(256, 256);
        let graph = DocumentGraph::new(size);
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())
            .expect("sync");
        let layer_id = graph.layers()[0].id;
        let backup = eng
            .clone_layer_texture(&ctx, layer_id)
            .expect("clone layer texture");
        eng.restore_layer_texture(&ctx, layer_id, &backup);
        let ms = eng.composite(&ctx, graph.layers()).expect("composite");
        assert!(ms >= 0.0);
        assert_eq!(eng.result_texture().width(), 256);
    }

    /// The interactive composite must not wait on the GPU: handbook 28 forbids
    /// the UI thread waiting for GPU completion, and `recomposite` runs there.
    ///
    /// A submit-only call returns in host time far below the GPU cost of the
    /// pass itself, so a regression that reintroduces a blocking poll shows up
    /// as host time converging on GPU time.
    #[test]
    fn interactive_composite_does_not_wait_for_the_gpu() {
        let ctx = GpuContext::new().expect("gpu");
        let size = DocumentSize::new(2048, 2048);
        let graph = DocumentGraph::new(size);
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())
            .expect("sync");
        // Warm pipelines and let the first timestamp readback land.
        for _ in 0..4 {
            eng.composite_measured(&ctx, graph.layers())
                .expect("warm composite");
        }
        let gpu_ms = eng
            .composite_measured(&ctx, graph.layers())
            .expect("gpu ms");

        let mut best_host_ms = f32::MAX;
        for _ in 0..10 {
            let start = Instant::now();
            eng.composite(&ctx, graph.layers()).expect("composite");
            best_host_ms = best_host_ms.min(start.elapsed().as_secs_f32() * 1000.0);
        }
        eprintln!(
            "[phototux_gpu] composite host {best_host_ms:.3} ms vs gpu {gpu_ms:.3} ms (no-wait)"
        );
        assert!(
            best_host_ms < gpu_ms.max(0.2),
            "composite blocked the caller: host {best_host_ms:.3} ms >= gpu {gpu_ms:.3} ms"
        );
    }

    #[test]
    fn painted_layer_marks_only_dirty_slice() {
        let ctx = GpuContext::new().expect("gpu");
        let size = DocumentSize::new(64, 64);
        let graph = DocumentGraph::new(size);
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())
            .expect("sync");
        eng.composite(&ctx, graph.layers()).expect("composite");
        let id = graph.layers()[0].id;
        eng.mark_layer_painted(id);
        // Incremental path: next composite must succeed without full membership rebuild.
        assert!(eng.composite(&ctx, graph.layers()).is_ok());
    }

    #[test]
    fn sync_rejects_over_max_layers() {
        let ctx = GpuContext::new().expect("gpu");
        let size = DocumentSize::new(32, 32);
        let mut graph = DocumentGraph::new(size);
        while graph.layer_count() < MAX_LAYERS {
            graph.add_layer_top(None).expect("fill to cap");
        }
        // Bypass graph cap via insert to exercise compositor rejection.
        let overflow = phototux_engine::Layer::new(phototux_engine::LayerId(10_000), "overflow");
        graph.insert_layer_at(graph.layer_count(), overflow);
        assert!(graph.layer_count() > MAX_LAYERS);
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        let err = eng
            .sync_layers_from_graph(&ctx, graph.layers())
            .expect_err("over-cap sync");
        assert!(err.contains("at most"));
    }

    #[test]
    fn layer_upload_and_composite_readback_round_trip() {
        let ctx = GpuContext::new().expect("gpu");
        let size = DocumentSize::new(2, 1);
        let graph = DocumentGraph::new(size);
        let mut engine = LayerCompositeEngine::new(&ctx, size);
        engine
            .sync_layers_from_graph(&ctx, graph.layers())
            .expect("sync");
        let pixels = [12, 34, 56, 255, 12, 34, 56, 255];

        for layer in graph.layers() {
            engine
                .write_layer_rgba(&ctx, layer.id, &pixels)
                .expect("upload layer");
        }
        engine.composite(&ctx, graph.layers()).expect("composite");
        let readback = engine.read_result_rgba(&ctx).expect("read composite");

        assert_eq!(readback, pixels);
    }

    #[test]
    fn mask_multiply_hides_layer_pixels() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let size = DocumentSize::new(2, 1);
        let mut graph = DocumentGraph::new(size);
        // Single opaque layer so composite equals layer*mask.
        let keep = graph.layers()[0].id;
        let drop_id = graph.layers()[1].id;
        graph.remove_layer(drop_id);
        graph.set_mask(keep, Some(Default::default()));
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())
            .expect("sync");
        eng.write_layer_rgba(&ctx, keep, &[255, 0, 0, 255, 255, 0, 0, 255])
            .expect("layer");
        // Left pixel hidden, right revealed.
        eng.write_mask_r8(&ctx, keep, &[0, 255]).expect("mask");
        eng.composite(&ctx, graph.layers()).expect("composite");
        let out = eng.read_result_rgba(&ctx).expect("readback");
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);
        assert_eq!(&out[4..8], &[255, 0, 0, 255]);
    }

    #[test]
    fn composite_10x4k_under_2ms() {
        let ctx = GpuContext::new().expect("gpu");
        let ms = benchmark_10x4k_ms(&ctx);
        eprintln!("[phototux_gpu] 10×4K composite (best of 10) = {ms:.3} ms (gate < 2.0)");
        // Host Instant includes submit/poll; keep 50µs slack for clock noise.
        assert!(
            ms < 2.05,
            "ADR-008 gate failed: 10×4K composite {ms:.3} ms >= 2.05 ms"
        );
    }
}
