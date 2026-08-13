//! GPU circular dab stamps onto a layer texture.

use bytemuck::{Pod, Zeroable};
use phototux_engine::{BrushParams, BrushTextureKind, Dab};

use crate::GpuContext;

const STAMP_WGSL: &str = r#"
struct Uniforms {
    center: vec2<f32>,
    radius: f32,
    hardness: f32,
    color: vec4<f32>,
    eraser: u32,
    texture_kind: u32,
    texture_strength: f32,
    _pad0: f32,
};

fn tip_noise(p: vec2<f32>) -> f32 {
    let n = sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453;
    return fract(n);
}

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
    var a = 1.0 - smoothstep(inner, u.radius, dist);
    if (u.texture_kind != 0u && u.texture_strength > 0.001) {
        let n = tip_noise(in.uv * 1024.0);
        a = a * (1.0 - u.texture_strength + u.texture_strength * n);
    }
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
    texture_kind: u32,
    texture_strength: f32,
    _pad0: f32,
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
        // Radius already includes size-pressure from StrokeBuilder; do not scale again.
        let radius_uv = req.radius_px / max_dim;
        let mut color = req.params.color;
        color[3] = req.params.stamp_alpha(req.pressure);
        let texture_kind = match req.params.texture {
            BrushTextureKind::None => 0u32,
            BrushTextureKind::Noise => 1u32,
        };
        StampUniforms {
            center_x: cx,
            center_y: cy,
            radius_uv,
            hardness: req.params.hardness.clamp(0.0, 1.0),
            color,
            eraser: u32::from(req.params.eraser),
            texture_kind,
            texture_strength: req.params.texture_strength.clamp(0.0, 1.0),
            _pad0: 0.0,
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
        // Only dabs that touch the target get a draw; a scissor rect must lie
        // inside the attachment and may not be empty.
        let drawable: Vec<(usize, StampRequest, ScissorRect)> = requests
            .iter()
            .copied()
            .filter_map(|req| {
                dab_scissor(req.x, req.y, req.radius_px, self.width, self.height)
                    .map(|rect| (req, rect))
            })
            .enumerate()
            .map(|(index, (req, rect))| (index, req, rect))
            .collect();
        if drawable.is_empty() {
            return;
        }

        // Uniform writes and bind groups first: `write_buffer` is ordered ahead
        // of the encoder's commands at submit, and the bind groups must outlive
        // the pass that references them.
        self.ensure_uniform_slots(ctx, drawable.len());
        let binds: Vec<wgpu::BindGroup> = drawable
            .iter()
            .map(|&(index, req, _)| {
                let uniforms = self.uniforms_for(req);
                let ubo = &self.uniform_bufs[index];
                ctx.queue
                    .write_buffer(ubo, 0, bytemuck::bytes_of(&uniforms));
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("stamp-bg"),
                    layout: &self.bind_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: ubo.as_entire_binding(),
                    }],
                })
            })
            .collect();

        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("stamp-batch-enc"),
            });
        {
            // One pass for the whole batch. Draws within a pass blend in
            // submission order, so this is identical to the pass-per-dab
            // version it replaces — without a pipeline drain and a full
            // attachment load/store between every dab.
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
            let mut bound_eraser: Option<bool> = None;
            for (&(_, req, rect), bind) in drawable.iter().zip(binds.iter()) {
                // The draw is a full-screen triangle whose fragments outside the
                // dab are discarded, so without a scissor a 20 px dab on a 4K
                // layer rasterizes every pixel of the layer to keep ~1250.
                pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
                if bound_eraser != Some(req.params.eraser) {
                    pass.set_pipeline(if req.params.eraser {
                        &self.pipeline_erase
                    } else {
                        &self.pipeline_paint
                    });
                    bound_eraser = Some(req.params.eraser);
                }
                pass.set_bind_group(0, bind, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
}

/// Integer scissor rect in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScissorRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Pixel bounds a dab can touch, clamped to the target.
///
/// Returns `None` when the dab lies entirely outside, which must not produce a
/// draw: an empty scissor rect is invalid.
pub(crate) fn dab_scissor(
    x: f32,
    y: f32,
    radius_px: f32,
    width: u32,
    height: u32,
) -> Option<ScissorRect> {
    if width == 0 || height == 0 || !x.is_finite() || !y.is_finite() || !radius_px.is_finite() {
        return None;
    }
    // One texel of slack for the edge falloff and for centre-vs-corner rounding.
    let reach = radius_px.max(0.0) + 1.0;
    let min_x = (x - reach).floor().max(0.0) as u32;
    let min_y = (y - reach).floor().max(0.0) as u32;
    let max_x = (x + reach).ceil().max(0.0) as u32;
    let max_y = (y + reach).ceil().max(0.0) as u32;
    if min_x >= width || min_y >= height {
        return None;
    }
    let max_x = max_x.min(width);
    let max_y = max_y.min(height);
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some(ScissorRect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    })
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
            ..BrushParams::default()
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

    #[test]
    fn scissor_covers_the_dab_and_clamps_to_the_target() {
        // Fully inside: the rect brackets the dab with a texel of slack.
        let inside = dab_scissor(50.0, 50.0, 10.0, 256, 256).expect("inside");
        assert!(inside.x <= 39 && inside.y <= 39, "{inside:?} clips the dab");
        assert!(
            inside.x + inside.width >= 61 && inside.y + inside.height >= 61,
            "{inside:?} clips the dab"
        );

        // Straddling an edge: clamped, still non-empty, still covers what is on-target.
        let corner = dab_scissor(2.0, 2.0, 10.0, 256, 256).expect("corner");
        assert_eq!((corner.x, corner.y), (0, 0));
        assert!(corner.width >= 13 && corner.height >= 13, "{corner:?}");

        let far = dab_scissor(250.0, 250.0, 20.0, 256, 256).expect("far corner");
        assert_eq!(far.x + far.width, 256);
        assert_eq!(far.y + far.height, 256);

        // Entirely outside must not draw: an empty scissor rect is invalid.
        assert_eq!(dab_scissor(-40.0, 50.0, 10.0, 256, 256), None);
        assert_eq!(dab_scissor(50.0, 400.0, 10.0, 256, 256), None);
        assert_eq!(dab_scissor(f32::NAN, 50.0, 10.0, 256, 256), None);
        assert_eq!(dab_scissor(50.0, 50.0, 10.0, 0, 256), None);
    }
}

/// Device-backed checks that the scissor bounds work rather than clip the dab.
///
/// The batch is stamped into one scissored render pass, so a rect that is too
/// small silently truncates a stroke. Nothing in the rest of the suite reads
/// pixels back after a stamp, so without this the optimisation is unguarded.
#[cfg(all(test, feature = "gpu-tests"))]
mod gpu_tests {
    use super::*;
    use crate::{GpuContext, LayerCompositeEngine};
    use phototux_engine::{DocumentGraph, DocumentSize};

    const W: u32 = 128;
    const H: u32 = 128;

    fn painted_alpha(requests: &[StampRequest]) -> Vec<u8> {
        let ctx = GpuContext::new().expect("gpu");
        let size = DocumentSize::new(W, H);
        let graph = DocumentGraph::new(size);
        let mut engine = LayerCompositeEngine::new(&ctx, size);
        engine
            .sync_layers_from_graph(&ctx, graph.layers())
            .expect("sync");
        let id = graph.layers()[0].id;
        engine
            .write_layer_rgba(&ctx, id, &vec![0u8; (W * H * 4) as usize])
            .expect("clear layer");

        let mut stamper = BrushStamper::new(&ctx, W, H);
        let target = engine.layer_texture(id).expect("layer texture").clone();
        stamper.stamp_batch(&ctx, &target, requests);

        let rgba = engine.read_layer_rgba(&ctx, id).expect("readback");
        rgba.chunks_exact(4).map(|px| px[3]).collect()
    }

    fn request_at(x: f32, y: f32, radius: f32) -> StampRequest {
        StampRequest {
            x,
            y,
            radius_px: radius,
            pressure: 1.0,
            params: BrushParams {
                size: radius * 2.0,
                hardness: 0.9,
                color: [1.0, 1.0, 1.0, 1.0],
                ..BrushParams::default()
            },
        }
    }

    fn alpha_at(alpha: &[u8], x: u32, y: u32) -> u8 {
        alpha[(y * W + x) as usize]
    }

    #[test]
    fn a_stamped_dab_covers_its_whole_radius() {
        let alpha = painted_alpha(&[request_at(64.0, 64.0, 12.0)]);
        assert!(alpha_at(&alpha, 64, 64) > 200, "centre not painted");
        // Near the rim, inside the radius: this is what a too-tight scissor eats.
        for (x, y) in [(54_u32, 64_u32), (74, 64), (64, 54), (64, 74)] {
            assert!(
                alpha_at(&alpha, x, y) > 0,
                "({x},{y}) inside the radius was not painted — scissor clipped it"
            );
        }
        // Well outside stays untouched.
        assert_eq!(alpha_at(&alpha, 5, 5), 0);
        assert_eq!(alpha_at(&alpha, 120, 120), 0);
    }

    #[test]
    fn a_batch_paints_every_dab_not_just_the_last() {
        // One pass for the batch: a scissor left set from a previous dab, or a
        // pass that only honours the final draw, shows up here.
        let dabs: Vec<StampRequest> = (0..5)
            .map(|i| request_at(20.0 + i as f32 * 20.0, 64.0, 6.0))
            .collect();
        let alpha = painted_alpha(&dabs);
        for i in 0..5 {
            let x = 20 + i * 20;
            assert!(
                alpha_at(&alpha, x, 64) > 200,
                "dab {i} at x={x} missing from the batch"
            );
        }
    }

    #[test]
    fn a_dab_straddling_the_edge_still_paints_what_is_on_canvas() {
        let alpha = painted_alpha(&[request_at(2.0, 64.0, 10.0)]);
        assert!(
            alpha_at(&alpha, 0, 64) > 0,
            "clamped edge dab did not paint"
        );
        assert!(alpha_at(&alpha, 6, 64) > 0, "clamped edge dab truncated");
    }

    #[test]
    fn a_dab_entirely_off_canvas_paints_nothing_and_does_not_fail() {
        let alpha = painted_alpha(&[request_at(-50.0, 64.0, 10.0)]);
        assert!(alpha.iter().all(|&a| a == 0), "off-canvas dab painted");
    }
}
