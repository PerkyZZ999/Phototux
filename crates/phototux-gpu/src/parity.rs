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

/// Blend modes compared CPU↔GPU.
///
/// Every mode, because both sides now compute every mode. This used to exclude
/// Hue/Saturation/Color/Luminosity, which neither side computed — both
/// returned the source — so the exclusion was hiding four modes that rendered
/// as Normal rather than describing a tolerance problem.
pub const PARITY_BLEND_MODES: &[BlendMode] = BlendMode::ALL;

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

/// CPU blend fixture whose two inputs both vary per pixel.
///
/// [`cpu_blend_fixture`] composites two solids, where `DarkerColor` and
/// `LighterColor` necessarily equal one of their inputs — they pick a pixel
/// rather than mixing one. Alternating both inputs lets every mode produce a
/// buffer of its own.
///
/// # Errors
/// Composite failure.
pub fn cpu_blend_fixture_varied(mode: BlendMode) -> Result<Vec<u8>, String> {
    const W: u32 = 8;
    const H: u32 = 8;
    // The backdrop straddles the source in luminosity — one cell brighter than
    // both source cells, one darker — so `LighterColor` and `DarkerColor` each
    // pick the backdrop somewhere and cannot coincide with Normal.
    let bottom = checker_rgba(W, H, 2, [230, 220, 200, 255], [30, 60, 90, 255]);
    let top = checker_rgba(W, H, 3, [60, 140, 220, 255], [180, 190, 70, 255]);
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

    /// A parity set that silently skipped a mode was how four modes shipped
    /// rendering as Normal. The set is the whole vocabulary or the omission is
    /// a decision someone has to write down here.
    #[test]
    fn the_parity_set_covers_every_blend_mode() {
        for &mode in BlendMode::ALL {
            assert!(
                PARITY_BLEND_MODES.contains(&mode),
                "{mode:?} is not compared against the shader"
            );
        }
    }

    /// Distinct modes must produce distinct output — the cheapest check that a
    /// new arm was wired to its own formula rather than falling through to the
    /// default, which is exactly how the component modes shipped as Normal.
    #[test]
    fn distinct_modes_produce_distinct_output() {
        let mut seen: Vec<(BlendMode, Vec<u8>)> = Vec::new();
        for &mode in BlendMode::ALL {
            // Pass-through is Normal in a flat stack by definition.
            if mode == BlendMode::PassThrough {
                continue;
            }
            let px = cpu_blend_fixture_varied(mode).expect("fixture");
            if let Some((other, _)) = seen.iter().find(|(_, p)| *p == px) {
                panic!("{mode:?} and {other:?} composite identically");
            }
            seen.push((mode, px));
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

    /// Composite a solid layer under an adjustment layer and hand back
    /// (expected, actual) RGB.
    ///
    /// `AdjustmentParams::apply_rgb` is the reference the shader mirrors.
    /// Nothing held them together before, and the shader's own mapping had a
    /// `_ => 0` arm — so four adjustment kinds composited as nothing while
    /// their parameters round-tripped through the document perfectly.
    fn gpu_adjustment_rgb(
        params: phototux_engine::AdjustmentParams,
        base: [u8; 4],
    ) -> Result<([u8; 3], [u8; 3]), String> {
        let ctx = GpuContext::new().map_err(|e| e.to_string())?;
        const W: u32 = 8;
        const H: u32 = 8;
        let size = DocumentSize::new(W, H);
        let mut graph = DocumentGraph::new_flattened(size, "base");
        let base_id = graph.layers()[0].id;
        // `add_adjustment_top`, not a raster layer with `set_adjustment` after:
        // that setter refuses a layer whose kind is not Adjustment, so the
        // fixture would have composited a blank raster layer instead.
        graph
            .add_adjustment_top(Some("adjust".to_owned()), params.clone())
            .map_err(|e| e.to_string())?;

        let pixels = solid_rgba(W, H, base);
        let mut eng = LayerCompositeEngine::new(&ctx, size);
        eng.sync_layers_from_graph(&ctx, graph.layers())?;
        eng.write_layer_rgba(&ctx, base_id, &pixels)
            .map_err(|e| e.to_string())?;
        eng.composite(&ctx, graph.layers())?;
        let gpu = eng.read_result_rgba(&ctx).map_err(|e| e.to_string())?;

        let expected = params.apply_rgb([
            f32::from(base[0]) / 255.0,
            f32::from(base[1]) / 255.0,
            f32::from(base[2]) / 255.0,
        ]);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "apply_rgb clamps to 0..1 before the scale"
        )]
        let expected = expected.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8);
        Ok((expected, [gpu[0], gpu[1], gpu[2]]))
    }

    /// Every adjustment kind must move the pixel the way the engine says, on
    /// the device. Tolerance is generous because the shader works in f32 and
    /// the HSL round trip is not exact, but "renders as nothing" fails by a
    /// mile rather than by a rounding step.
    #[test]
    fn every_adjustment_kind_matches_the_engine_on_device() {
        use phototux_engine::AdjustmentParams;
        let base = [180, 90, 60, 255];
        for kind in AdjustmentParams::ALL_KINDS {
            // Defaults are neutral by design, so each kind is nudged off it —
            // a neutral adjustment cannot tell "applied" from "skipped".
            let mut slots = kind.slots();
            for (i, (_, min, max)) in kind.editor_slots().iter().enumerate() {
                slots[i] = (slots[i] + (max - min) * 0.25).clamp(*min, *max);
            }
            let moved = kind.with_slots(slots).clamped();
            let (expected, actual) = match gpu_adjustment_rgb(moved.clone(), base) {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("skipping {}: {e}", moved.kind_key());
                    return;
                }
            };
            for (c, (e, a)) in expected.iter().zip(actual).enumerate() {
                let diff = i32::from(*e).abs_diff(i32::from(a));
                assert!(
                    diff <= 3,
                    "{}: channel {c} expected {e}, device gave {a}",
                    moved.kind_key()
                );
            }
        }
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
