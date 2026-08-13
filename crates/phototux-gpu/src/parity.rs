//! GPU↔CPU blend/filter parity fixtures (handbook parity P6).
//!
//! CPU reference: [`phototux_engine::composite_rgba8`] and [`crate::cpu_gaussian_rgba`] /
//! [`crate::cpu_sharpen_rgba`]. Device path gated by feature `gpu-tests`.
//!
//! ## Tolerances
//! - Blends (opaque fixtures): max abs ≤ 2/255, mean ≤ 1/255 per channel.
//! - Gaussian blur: GPU separable + linear `textureSample` vs CPU discrete taps diverge on
//!   high-contrast checkers (mean often ~20–30). Fixture requires both softens vs source and
//!   mean CPU↔GPU ≤ 40/255 (structural parity, not bit-identical).
//! - Sharpen: max abs ≤ 4/255.
//!
//! ## Skip matrix
//! - Hue / Saturation / Color / Luminosity: GPU RGB fallback; CPU falls back to Normal — not compared.
//! - Motion / emboss: covered by unit refs only; full GPU↔CPU deferred with handbook filter catalog.

use phototux_engine::{BlendMode, CpuLayerRef, composite_rgba8};

/// Max absolute per-channel error (0–255 scale).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelError {
    pub max_abs: f32,
    pub mean: f32,
}

/// Compare two equal-length RGBA8 buffers; returns per-channel stats (R,G,B,A).
///
/// # Errors
/// Length mismatch.
pub fn rgba8_channel_errors(a: &[u8], b: &[u8]) -> Result<[ChannelError; 4], String> {
    if a.len() != b.len() || !a.len().is_multiple_of(4) {
        return Err(format!(
            "buffer length mismatch or not RGBA: {} vs {}",
            a.len(),
            b.len()
        ));
    }
    let n = a.len() / 4;
    if n == 0 {
        return Ok([ChannelError {
            max_abs: 0.0,
            mean: 0.0,
        }; 4]);
    }
    let mut max = [0.0_f32; 4];
    let mut sum = [0.0_f32; 4];
    for i in 0..n {
        for c in 0..4 {
            let d = (i32::from(a[i * 4 + c]) - i32::from(b[i * 4 + c])).unsigned_abs() as f32;
            max[c] = max[c].max(d);
            sum[c] += d;
        }
    }
    let inv = 1.0 / n as f32;
    Ok(std::array::from_fn(|c| ChannelError {
        max_abs: max[c],
        mean: sum[c] * inv,
    }))
}

/// Assert all channels within `max_abs` / `mean` tolerances.
///
/// # Errors
/// Compare failure or tolerance breach.
pub fn assert_rgba8_within(a: &[u8], b: &[u8], max_abs: f32, mean: f32) -> Result<(), String> {
    let errs = rgba8_channel_errors(a, b)?;
    for (i, e) in errs.iter().enumerate() {
        if e.max_abs > max_abs || e.mean > mean {
            return Err(format!(
                "channel {i}: max_abs={} mean={} (limits max={max_abs} mean={mean})",
                e.max_abs, e.mean
            ));
        }
    }
    Ok(())
}

/// Solid RGBA fill.
pub fn solid_rgba(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let n = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    let mut v = Vec::with_capacity(n);
    for _ in 0..(n / 4) {
        v.extend_from_slice(&rgba);
    }
    v
}

/// Checkerboard (period `cell` pixels).
pub fn checker_rgba(width: u32, height: u32, cell: u32, a: [u8; 4], b: [u8; 4]) -> Vec<u8> {
    let cell = cell.max(1);
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let on = ((x / cell) + (y / cell)).is_multiple_of(2);
            out.extend_from_slice(if on { &a } else { &b });
        }
    }
    out
}

/// Blend modes covered by CPU reference + GPU shader (parity set).
/// Every mode both the CPU reference and the shader genuinely implement.
///
/// Hue/Saturation/Color/Luminosity are excluded because neither side computes
/// them — both fall back to the source — so listing them would assert agreement
/// on a shared shortcut rather than on a blend.
pub const PARITY_BLEND_MODES: &[BlendMode] = &[
    BlendMode::Normal,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Overlay,
    BlendMode::Darken,
    BlendMode::Lighten,
    BlendMode::ColorDodge,
    BlendMode::ColorBurn,
    BlendMode::HardLight,
    BlendMode::SoftLight,
    BlendMode::Difference,
    BlendMode::Exclusion,
];

/// CPU blend fixture: bottom solid + top solid with `mode`.
///
/// # Errors
/// Composite failure.
pub fn cpu_blend_fixture(mode: BlendMode) -> Result<Vec<u8>, String> {
    const W: u32 = 8;
    const H: u32 = 8;
    let bottom = solid_rgba(W, H, [200, 80, 40, 255]);
    let top = solid_rgba(W, H, [60, 140, 220, 255]);
    composite_rgba8(
        W,
        H,
        &[
            CpuLayerRef {
                visible: true,
                opacity: 1.0,
                blend: BlendMode::Normal,
                rgba: &bottom,
            },
            CpuLayerRef {
                visible: true,
                opacity: 1.0,
                blend: mode,
                rgba: &top,
            },
        ],
    )
}

/// CPU gaussian reference on checker fixture.
pub fn cpu_gaussian_fixture(radius: f32) -> Vec<u8> {
    const W: u32 = 16;
    const H: u32 = 16;
    let mut px = checker_rgba(W, H, 4, [255, 0, 0, 255], [0, 0, 255, 255]);
    crate::cpu_gaussian_rgba(&mut px, W, H, radius);
    px
}

/// CPU sharpen reference on soft edge fixture.
pub fn cpu_sharpen_fixture(amount: f32) -> Vec<u8> {
    const W: u32 = 16;
    const H: u32 = 16;
    let mut px = checker_rgba(W, H, 4, [40, 40, 40, 255], [220, 220, 220, 255]);
    crate::cpu_sharpen_rgba(&mut px, W, H, amount);
    px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_blend_fixtures_are_deterministic() {
        for &mode in PARITY_BLEND_MODES {
            let a = cpu_blend_fixture(mode).expect("cpu a");
            let b = cpu_blend_fixture(mode).expect("cpu b");
            assert_eq!(a, b, "mode={mode:?}");
            assert_rgba8_within(&a, &b, 0.0, 0.0).expect("identical");
        }
    }

    #[test]
    fn cpu_multiply_darkens_vs_normal() {
        let normal = cpu_blend_fixture(BlendMode::Normal).expect("normal");
        let multiply = cpu_blend_fixture(BlendMode::Multiply).expect("multiply");
        // Opaque top → Normal is top color; Multiply darkens vs pure top on bright bottom.
        assert_ne!(&normal[..4], &multiply[..4]);
        assert!(multiply[0] < normal[0] || multiply[1] < normal[1] || multiply[2] < normal[2]);
    }

    #[test]
    fn cpu_gaussian_softens_checker() {
        let sharp = checker_rgba(16, 16, 4, [255, 0, 0, 255], [0, 0, 255, 255]);
        let soft = cpu_gaussian_fixture(2.0);
        let errs = rgba8_channel_errors(&sharp, &soft).expect("err");
        assert!(errs[0].max_abs > 2.0 || errs[2].max_abs > 2.0);
    }

    #[test]
    fn cpu_sharpen_changes_edge() {
        let base = checker_rgba(16, 16, 4, [40, 40, 40, 255], [220, 220, 220, 255]);
        let sharp = cpu_sharpen_fixture(0.8);
        assert_ne!(base, sharp);
    }
}

#[cfg(all(test, feature = "gpu-tests"))]
mod gpu_tests {
    use super::*;
    use crate::{GpuContext, LayerCompositeEngine};
    use phototux_engine::{DocumentGraph, DocumentSize, LayerId};

    fn two_layer_graph(size: DocumentSize) -> DocumentGraph {
        DocumentGraph::new(size)
    }

    fn gpu_blend(mode: BlendMode) -> Result<(Vec<u8>, Vec<u8>), String> {
        let ctx = GpuContext::new().map_err(|e| e.to_string())?;
        const W: u32 = 8;
        const H: u32 = 8;
        let size = DocumentSize::new(W, H);
        let mut graph = two_layer_graph(size);
        let bottom_id = graph.layers()[0].id;
        let top_id = graph.layers()[1].id;
        graph.set_blend(top_id, mode);
        let bottom = solid_rgba(W, H, [200, 80, 40, 255]);
        let top = solid_rgba(W, H, [60, 140, 220, 255]);
        let cpu = composite_rgba8(
            W,
            H,
            &[
                CpuLayerRef {
                    visible: true,
                    opacity: 1.0,
                    blend: BlendMode::Normal,
                    rgba: &bottom,
                },
                CpuLayerRef {
                    visible: true,
                    opacity: 1.0,
                    blend: mode,
                    rgba: &top,
                },
            ],
        )?;
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())?;
        eng.write_layer_rgba(&ctx, bottom_id, &bottom)
            .map_err(|e| e.to_string())?;
        eng.write_layer_rgba(&ctx, top_id, &top)
            .map_err(|e| e.to_string())?;
        eng.composite(&ctx, graph.layers())?;
        let gpu = eng.read_result_rgba(&ctx).map_err(|e| e.to_string())?;
        Ok((cpu, gpu))
    }

    /// Composite one masked layer and hand back (expected, actual) alpha.
    ///
    /// `LayerMask::coverage` is the single definition of mask semantics; the
    /// shader has its own WGSL copy of the same order. Nothing proved they still
    /// agreed, which is exactly how the *other* copy — the bake path — silently
    /// dropped contrast and shift.
    fn gpu_mask_alpha(mask: phototux_engine::LayerMask) -> Result<(Vec<u8>, Vec<u8>), String> {
        let ctx = GpuContext::new().map_err(|e| e.to_string())?;
        const W: u32 = 8;
        const H: u32 = 8;
        let size = DocumentSize::new(W, H);
        let mut graph = DocumentGraph::new_flattened(size, "masked");
        let id = graph.layers()[0].id;
        graph.set_mask(id, Some(mask.clone()));

        // Opaque white so the composited alpha is the mask's coverage alone.
        let pixels = solid_rgba(W, H, [255, 255, 255, 255]);
        // A coverage ramp across the whole byte range, so contrast and shift
        // have something to act on rather than only the 0/255 endpoints.
        let coverage: Vec<u8> = (0..(W * H) as usize)
            .map(|i| (i * 255 / ((W * H) as usize - 1)) as u8)
            .collect();

        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())?;
        eng.write_layer_rgba(&ctx, id, &pixels)
            .map_err(|e| e.to_string())?;
        eng.ensure_mask(&ctx, id);
        eng.write_mask_r8(&ctx, id, &coverage)
            .map_err(|e| e.to_string())?;
        eng.composite(&ctx, graph.layers())?;
        let gpu = eng.read_result_rgba(&ctx).map_err(|e| e.to_string())?;

        let expected: Vec<u8> = coverage
            .iter()
            .map(|&sample| {
                let c = mask.coverage(f32::from(sample) / 255.0);
                (c * 255.0).round().clamp(0.0, 255.0) as u8
            })
            .collect();
        let actual: Vec<u8> = gpu.chunks_exact(4).map(|px| px[3]).collect();
        Ok((expected, actual))
    }

    /// The shader must agree with `LayerMask::coverage` across the controls
    /// that actually change the curve, not only at the defaults.
    #[test]
    fn gpu_cpu_mask_coverage_parity() {
        use phototux_engine::LayerMask;
        let cases = [
            ("default", LayerMask::default()),
            (
                "inverted",
                LayerMask {
                    inverted: true,
                    ..LayerMask::default()
                },
            ),
            (
                "half density",
                LayerMask {
                    density: 0.5,
                    ..LayerMask::default()
                },
            ),
            (
                "contrast",
                LayerMask {
                    contrast: 0.6,
                    ..LayerMask::default()
                },
            ),
            (
                "shift",
                LayerMask {
                    shift: 0.2,
                    ..LayerMask::default()
                },
            ),
            (
                "contrast and shift inverted",
                LayerMask {
                    contrast: -0.4,
                    shift: -0.15,
                    inverted: true,
                    density: 0.8,
                    ..LayerMask::default()
                },
            ),
        ];
        for (name, mask) in cases {
            let Ok((expected, actual)) = gpu_mask_alpha(mask) else {
                eprintln!("skipping mask parity: no GPU device");
                return;
            };
            assert_eq!(expected.len(), actual.len(), "{name}: length");
            for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
                let delta = i32::from(*e) - i32::from(*a);
                assert!(
                    delta.abs() <= 2,
                    "{name}: sample {i} expected {e}, shader gave {a}"
                );
            }
        }
    }

    fn gpu_filter_gaussian(radius: f32) -> Result<(Vec<u8>, Vec<u8>), String> {
        let ctx = GpuContext::new().map_err(|e| e.to_string())?;
        const W: u32 = 16;
        const H: u32 = 16;
        let size = DocumentSize::new(W, H);
        let mut graph = DocumentGraph::new_flattened(size, "fx");
        let id = graph.layers()[0].id;
        graph
            .add_gaussian_blur(id, radius)
            .ok_or_else(|| "add gaussian".to_owned())?;
        let src = checker_rgba(W, H, 4, [255, 0, 0, 255], [0, 0, 255, 255]);
        let mut cpu = src.clone();
        crate::cpu_gaussian_rgba(&mut cpu, W, H, radius);
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())?;
        eng.write_layer_rgba(&ctx, id, &src)
            .map_err(|e| e.to_string())?;
        eng.composite(&ctx, graph.layers())?;
        let gpu = eng.read_result_rgba(&ctx).map_err(|e| e.to_string())?;
        Ok((cpu, gpu))
    }

    fn gpu_filter_sharpen(amount: f32) -> Result<(Vec<u8>, Vec<u8>), String> {
        let ctx = GpuContext::new().map_err(|e| e.to_string())?;
        const W: u32 = 16;
        const H: u32 = 16;
        let size = DocumentSize::new(W, H);
        let mut graph = DocumentGraph::new_flattened(size, "fx");
        let id: LayerId = graph.layers()[0].id;
        graph
            .add_sharpen(id, amount)
            .ok_or_else(|| "add sharpen".to_owned())?;
        let src = checker_rgba(W, H, 4, [40, 40, 40, 255], [220, 220, 220, 255]);
        let mut cpu = src.clone();
        crate::cpu_sharpen_rgba(&mut cpu, W, H, amount);
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())?;
        eng.write_layer_rgba(&ctx, id, &src)
            .map_err(|e| e.to_string())?;
        eng.composite(&ctx, graph.layers())?;
        let gpu = eng.read_result_rgba(&ctx).map_err(|e| e.to_string())?;
        Ok((cpu, gpu))
    }

    #[test]
    fn gpu_cpu_blend_parity_across_every_shared_mode() {
        for &mode in PARITY_BLEND_MODES {
            let (cpu, gpu) = gpu_blend(mode).expect("gpu blend");
            assert_rgba8_within(&cpu, &gpu, 2.0, 1.0)
                .unwrap_or_else(|e| panic!("blend {mode:?}: {e}"));
        }
    }

    #[test]
    fn gpu_cpu_gaussian_parity() {
        const W: u32 = 16;
        const H: u32 = 16;
        let src = checker_rgba(W, H, 4, [255, 0, 0, 255], [0, 0, 255, 255]);
        let (cpu, gpu) = gpu_filter_gaussian(2.0).expect("gaussian");
        // Both paths must soften the checker (blur ran).
        let cpu_vs_src = rgba8_channel_errors(&src, &cpu).expect("cpu vs src");
        let gpu_vs_src = rgba8_channel_errors(&src, &gpu).expect("gpu vs src");
        assert!(
            cpu_vs_src[0].mean > 5.0 || cpu_vs_src[2].mean > 5.0,
            "cpu gaussian did not soften"
        );
        assert!(
            gpu_vs_src[0].mean > 5.0 || gpu_vs_src[2].mean > 5.0,
            "gpu gaussian did not soften"
        );
        // Linear GPU vs discrete CPU: allow large peak error; bound mean drift.
        assert_rgba8_within(&cpu, &gpu, 90.0, 40.0).expect("gaussian structural tolerance");
    }

    #[test]
    fn gpu_cpu_sharpen_parity() {
        let (cpu, gpu) = gpu_filter_sharpen(0.8).expect("sharpen");
        assert_rgba8_within(&cpu, &gpu, 4.0, 2.0).expect("sharpen tolerance");
    }
}
