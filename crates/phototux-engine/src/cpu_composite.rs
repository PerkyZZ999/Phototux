//! CPU reference compositor for headless tests (handbook Phase 4.1 / DR-006).
//!
//! Straight (non-premultiplied) RGBA8, bottom→top. Unsupported blend modes fall back to Normal
//! so fixtures remain deterministic while GPU covers the full set.

use crate::layer::BlendMode;

/// One raster layer input for CPU composite.
#[derive(Debug, Clone, Copy)]
pub struct CpuLayerRef<'a> {
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    /// Length must be `width * height * 4`.
    pub rgba: &'a [u8],
}

/// Composite `layers` (index 0 = bottom) into a new RGBA8 buffer.
///
/// # Errors
/// Returns an error string when dimensions are zero or a layer buffer length mismatches.
pub fn composite_rgba8(
    width: u32,
    height: u32,
    layers: &[CpuLayerRef<'_>],
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("zero dimensions".into());
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimensions overflow".to_owned())?;
    for (i, layer) in layers.iter().enumerate() {
        if layer.rgba.len() != pixels {
            return Err(format!(
                "layer {i} buffer length {} != expected {pixels}",
                layer.rgba.len()
            ));
        }
    }

    let mut dst = vec![0_u8; pixels];
    for layer in layers {
        if !layer.visible {
            continue;
        }
        let opacity = layer.opacity.clamp(0.0, 1.0);
        if opacity <= 0.0 {
            continue;
        }
        for i in 0..pixels / 4 {
            let o = i * 4;
            let src = [
                layer.rgba[o] as f32 / 255.0,
                layer.rgba[o + 1] as f32 / 255.0,
                layer.rgba[o + 2] as f32 / 255.0,
                layer.rgba[o + 3] as f32 / 255.0,
            ];
            let mut dst_px = [
                dst[o] as f32 / 255.0,
                dst[o + 1] as f32 / 255.0,
                dst[o + 2] as f32 / 255.0,
                dst[o + 3] as f32 / 255.0,
            ];
            blend_over(&mut dst_px, src, opacity, layer.blend);
            dst[o] = (dst_px[0] * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[o + 1] = (dst_px[1] * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[o + 2] = (dst_px[2] * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[o + 3] = (dst_px[3] * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(dst)
}

fn blend_channel(mode: BlendMode, b: f32, s: f32) -> f32 {
    match mode {
        BlendMode::Multiply => b * s,
        BlendMode::Screen => 1.0 - (1.0 - b) * (1.0 - s),
        BlendMode::Overlay => {
            if b < 0.5 {
                2.0 * b * s
            } else {
                1.0 - 2.0 * (1.0 - b) * (1.0 - s)
            }
        }
        BlendMode::Darken => b.min(s),
        BlendMode::Lighten => b.max(s),
        BlendMode::ColorDodge => {
            if s >= 1.0 {
                1.0
            } else {
                (b / (1.0 - s)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - b) / s).min(1.0)
            }
        }
        BlendMode::HardLight => {
            if s < 0.5 {
                2.0 * b * s
            } else {
                1.0 - 2.0 * (1.0 - b) * (1.0 - s)
            }
        }
        // Matches the shader's cheap approximation rather than the W3C
        // formulation, because parity with what the canvas draws is the point.
        BlendMode::SoftLight => (1.0 - 2.0 * s) * b * b + 2.0 * s * b,
        BlendMode::Difference => (b - s).abs(),
        BlendMode::Exclusion => b + s - 2.0 * b * s,
        // Hue/Saturation/Color/Luminosity are separable-channel impossible and
        // the shader falls back to the source too; PassThrough is Normal in a
        // flat stack.
        BlendMode::Normal
        | BlendMode::Hue
        | BlendMode::Saturation
        | BlendMode::Color
        | BlendMode::Luminosity
        | BlendMode::PassThrough => s,
    }
}

fn blend_over(dst: &mut [f32; 4], src: [f32; 4], opacity: f32, mode: BlendMode) {
    let sa = (src[3] * opacity).clamp(0.0, 1.0);
    if sa <= 0.0 {
        return;
    }
    let da = dst[3];
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        *dst = [0.0, 0.0, 0.0, 0.0];
        return;
    }
    for c in 0..3 {
        let blended = blend_channel(mode, dst[c], src[c]);
        // Porter-Duff over with blend substituting source color.
        let cs = blended * sa + dst[c] * da * (1.0 - sa);
        dst[c] = (cs / out_a).clamp(0.0, 1.0);
    }
    dst[3] = out_a;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let n = (w * h) as usize;
        let mut v = Vec::with_capacity(n * 4);
        for _ in 0..n {
            v.extend_from_slice(&rgba);
        }
        v
    }

    #[test]
    fn normal_over_opaque() {
        let w = 2;
        let h = 2;
        let bottom = solid(w, h, [255, 0, 0, 255]);
        let top = solid(w, h, [0, 0, 255, 255]);
        let out = composite_rgba8(
            w,
            h,
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
                    blend: BlendMode::Normal,
                    rgba: &top,
                },
            ],
        )
        .expect("composite");
        assert_eq!(&out[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn multiply_darkens() {
        let w = 1;
        let h = 1;
        let bottom = solid(w, h, [200, 200, 200, 255]);
        let top = solid(w, h, [128, 128, 128, 255]);
        let out = composite_rgba8(
            w,
            h,
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
                    blend: BlendMode::Multiply,
                    rgba: &top,
                },
            ],
        )
        .expect("composite");
        assert!(out[0] < 200);
        assert_eq!(out[3], 255);
    }

    #[test]
    fn hidden_layer_skipped() {
        let w = 1;
        let h = 1;
        let bottom = solid(w, h, [10, 20, 30, 255]);
        let top = solid(w, h, [255, 255, 255, 255]);
        let out = composite_rgba8(
            w,
            h,
            &[
                CpuLayerRef {
                    visible: true,
                    opacity: 1.0,
                    blend: BlendMode::Normal,
                    rgba: &bottom,
                },
                CpuLayerRef {
                    visible: false,
                    opacity: 1.0,
                    blend: BlendMode::Normal,
                    rgba: &top,
                },
            ],
        )
        .expect("composite");
        assert_eq!(&out[..], &[10, 20, 30, 255]);
    }

    /// Colour dodge brightens and colour burn darkens. Both were absent from
    /// the CPU reference (it fell through to `_ => s`), and the shader had the
    /// arms of its `select` reversed, so dodge returned white for every source
    /// below 1.0 and burn returned black for every source above 0.0.
    #[test]
    fn dodge_brightens_and_burn_darkens() {
        let mid = 0.5;
        assert!(
            blend_channel(BlendMode::ColorDodge, mid, 0.5) > mid,
            "dodge must brighten the backdrop"
        );
        assert!(
            blend_channel(BlendMode::ColorBurn, mid, 0.5) < mid,
            "burn must darken the backdrop"
        );
    }

    #[test]
    fn dodge_and_burn_saturate_at_their_limits() {
        assert!((blend_channel(BlendMode::ColorDodge, 0.5, 1.0) - 1.0).abs() < 1e-6);
        assert!((blend_channel(BlendMode::ColorBurn, 0.5, 0.0) - 0.0).abs() < 1e-6);
    }

    /// Hard light is overlay with the operands swapped.
    #[test]
    fn hard_light_mirrors_overlay() {
        for step in 0..=10 {
            let v = step as f32 / 10.0;
            let hard = blend_channel(BlendMode::HardLight, 0.3, v);
            let overlay = blend_channel(BlendMode::Overlay, v, 0.3);
            assert!((hard - overlay).abs() < 1e-6, "at {v}: {hard} vs {overlay}");
        }
    }

    /// Every mode must stay in range; a formula that escapes 0..1 shows up as
    /// clipped or wrapped pixels rather than as an error.
    #[test]
    fn every_blend_mode_stays_in_range() {
        for mode in BlendMode::ALL {
            for bs in 0..=10 {
                for ss in 0..=10 {
                    let out = blend_channel(mode, bs as f32 / 10.0, ss as f32 / 10.0);
                    assert!(
                        (-1e-6..=1.0 + 1e-6).contains(&out),
                        "{mode:?} produced {out}"
                    );
                }
            }
        }
    }
}
