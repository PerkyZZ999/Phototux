//! CPU reference compositor for headless tests (handbook Phase 4.1 / DR-006).
//!
//! Straight (non-premultiplied) RGBA8, bottom→top. Every mode in
//! [`BlendMode::ALL`] is implemented here, so a fixture disagreeing with the
//! canvas means the shader and this file have drifted — not that the mode is
//! unsupported.

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

/// Blend one channel for a mode that is defined per channel.
///
/// Callers outside this module want [`blend_rgb`]: the whole-colour and
/// component modes have no per-channel answer, and this function reports the
/// source for them the way `Normal` does.
fn blend_channel(mode: BlendMode, b: f32, s: f32) -> f32 {
    match mode {
        BlendMode::Multiply => b * s,
        BlendMode::Screen => 1.0 - (1.0 - b) * (1.0 - s),
        BlendMode::Overlay => hard_light_channel(s, b),
        BlendMode::Darken => b.min(s),
        BlendMode::Lighten => b.max(s),
        BlendMode::ColorDodge => color_dodge_channel(b, s),
        BlendMode::ColorBurn => color_burn_channel(b, s),
        BlendMode::HardLight => hard_light_channel(b, s),
        // Matches the shader's cheap approximation rather than the W3C
        // formulation, because parity with what the canvas draws is the point.
        BlendMode::SoftLight => (1.0 - 2.0 * s) * b * b + 2.0 * s * b,
        BlendMode::Difference => (b - s).abs(),
        BlendMode::Exclusion => b + s - 2.0 * b * s,
        BlendMode::LinearBurn => (b + s - 1.0).clamp(0.0, 1.0),
        BlendMode::LinearDodge => (b + s).clamp(0.0, 1.0),
        BlendMode::VividLight => vivid_light_channel(b, s),
        BlendMode::LinearLight => (b + 2.0 * s - 1.0).clamp(0.0, 1.0),
        BlendMode::PinLight => {
            if s <= 0.5 {
                b.min(2.0 * s)
            } else {
                b.max(2.0 * s - 1.0)
            }
        }
        // Vivid Light pushed to its extremes, which is what makes the result
        // posterize to the eight corners of the colour cube.
        BlendMode::HardMix => {
            if vivid_light_channel(b, s) < 0.5 {
                0.0
            } else {
                1.0
            }
        }
        BlendMode::Subtract => (b - s).clamp(0.0, 1.0),
        BlendMode::Divide => {
            if s <= 0.0 {
                1.0
            } else {
                (b / s).min(1.0)
            }
        }
        // Handled whole-pixel by `blend_rgb`; PassThrough is Normal in a flat
        // stack.
        BlendMode::Normal
        | BlendMode::Hue
        | BlendMode::Saturation
        | BlendMode::Color
        | BlendMode::Luminosity
        | BlendMode::DarkerColor
        | BlendMode::LighterColor
        | BlendMode::PassThrough => s,
    }
}

fn hard_light_channel(b: f32, s: f32) -> f32 {
    if s < 0.5 {
        2.0 * b * s
    } else {
        1.0 - 2.0 * (1.0 - b) * (1.0 - s)
    }
}

fn color_dodge_channel(b: f32, s: f32) -> f32 {
    if s >= 1.0 {
        1.0
    } else {
        (b / (1.0 - s)).min(1.0)
    }
}

fn color_burn_channel(b: f32, s: f32) -> f32 {
    if s <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - b) / s).min(1.0)
    }
}

fn vivid_light_channel(b: f32, s: f32) -> f32 {
    if s <= 0.5 {
        color_burn_channel(b, 2.0 * s)
    } else {
        color_dodge_channel(b, 2.0 * (s - 0.5))
    }
}

/// Rec.601 luma, the weighting the component blend modes are defined against.
fn luminosity(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn saturation_of(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// Pull `c` back inside the unit cube without moving its luminosity.
///
/// `set_luminosity` translates all three channels equally, which can push one
/// past an end of the range. Clipping each channel on its own would change the
/// luminosity that was just set, so the colour is scaled toward its own luma
/// instead.
fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = luminosity(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        let d = l - n;
        if d > f32::EPSILON {
            for v in &mut out {
                *v = l + (*v - l) * l / d;
            }
        } else {
            out = [l; 3];
        }
    }
    if x > 1.0 {
        let l = luminosity(out);
        let x = out[0].max(out[1]).max(out[2]);
        let d = x - l;
        if d > f32::EPSILON {
            for v in &mut out {
                *v = l + (*v - l) * (1.0 - l) / d;
            }
        } else {
            out = [l; 3];
        }
    }
    out
}

fn set_luminosity(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - luminosity(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

/// Rescale `c` to span `s` between its darkest and lightest channel.
fn set_saturation(c: [f32; 3], s: f32) -> [f32; 3] {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    if max <= min {
        return [0.0; 3];
    }
    let span = max - min;
    let mut out = [0.0_f32; 3];
    for (o, v) in out.iter_mut().zip(c) {
        *o = (v - min) * s / span;
    }
    out
}

/// Blend a whole source colour over a whole backdrop colour.
///
/// The component and whole-colour modes are only defined on all three channels
/// at once, so this — not [`blend_channel`] — is the compositor's entry point.
#[must_use]
pub fn blend_rgb(mode: BlendMode, backdrop: [f32; 3], source: [f32; 3]) -> [f32; 3] {
    match mode {
        BlendMode::Hue => set_luminosity(
            set_saturation(source, saturation_of(backdrop)),
            luminosity(backdrop),
        ),
        BlendMode::Saturation => set_luminosity(
            set_saturation(backdrop, saturation_of(source)),
            luminosity(backdrop),
        ),
        BlendMode::Color => set_luminosity(source, luminosity(backdrop)),
        BlendMode::Luminosity => set_luminosity(backdrop, luminosity(source)),
        BlendMode::DarkerColor => {
            if luminosity(source) < luminosity(backdrop) {
                source
            } else {
                backdrop
            }
        }
        BlendMode::LighterColor => {
            if luminosity(source) > luminosity(backdrop) {
                source
            } else {
                backdrop
            }
        }
        _ => {
            let mut out = [0.0_f32; 3];
            for (i, o) in out.iter_mut().enumerate() {
                *o = blend_channel(mode, backdrop[i], source[i]);
            }
            out
        }
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
    let blended = blend_rgb(mode, [dst[0], dst[1], dst[2]], [src[0], src[1], src[2]]);
    for (c, b) in blended.iter().enumerate() {
        // Porter-Duff over with blend substituting source color.
        let cs = b * sa + dst[c] * da * (1.0 - sa);
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
        for &mode in BlendMode::ALL {
            for bs in 0..=10 {
                for ss in 0..=10 {
                    let b = [bs as f32 / 10.0, 1.0 - ss as f32 / 10.0, 0.5];
                    let s = [ss as f32 / 10.0, bs as f32 / 10.0, 0.25];
                    for out in blend_rgb(mode, b, s) {
                        assert!(
                            (-1e-6..=1.0 + 1e-6).contains(&out),
                            "{mode:?} produced {out}"
                        );
                    }
                }
            }
        }
    }

    /// The four component modes are the reason `blend_rgb` exists. Each used
    /// to return the source unchanged, so a Luminosity layer over a coloured
    /// backdrop painted the source's hue instead of borrowing the backdrop's —
    /// visible in the app, invisible to a per-channel test.
    #[test]
    fn component_modes_borrow_from_the_backdrop() {
        let backdrop = [0.8, 0.2, 0.2];
        let source = [0.2, 0.2, 0.8];

        // Color and Hue keep the backdrop's brightness.
        for mode in [BlendMode::Color, BlendMode::Hue] {
            let out = blend_rgb(mode, backdrop, source);
            assert!(
                (luminosity(out) - luminosity(backdrop)).abs() < 1e-4,
                "{mode:?} moved the backdrop's luminosity: {out:?}"
            );
            assert!(out != source, "{mode:?} returned the source unchanged");
        }

        // Luminosity takes the source's brightness onto the backdrop's colour.
        let lum = blend_rgb(BlendMode::Luminosity, backdrop, source);
        assert!((luminosity(lum) - luminosity(source)).abs() < 1e-4);
        assert!(
            lum[0] > lum[2],
            "backdrop was red; the result should stay red"
        );

        // Saturation keeps the backdrop's brightness and takes the source's span.
        let sat = blend_rgb(BlendMode::Saturation, backdrop, source);
        assert!((luminosity(sat) - luminosity(backdrop)).abs() < 1e-4);
        assert!((saturation_of(sat) - saturation_of(source)).abs() < 1e-3);
    }

    /// A grey source carries no hue, so Color over grey must produce grey —
    /// the identity that catches a `set_saturation` that forgot to divide.
    #[test]
    fn color_mode_over_grey_source_is_grey() {
        let out = blend_rgb(BlendMode::Color, [0.25, 0.5, 0.75], [0.4, 0.4, 0.4]);
        assert!((out[0] - out[1]).abs() < 1e-4 && (out[1] - out[2]).abs() < 1e-4);
    }

    /// The whole-colour modes choose a pixel rather than mixing one, so the
    /// result is always one of their two inputs.
    #[test]
    fn whole_colour_modes_pick_one_input() {
        let dark = [0.1, 0.1, 0.2];
        let light = [0.9, 0.8, 0.7];
        assert_eq!(blend_rgb(BlendMode::DarkerColor, light, dark), dark);
        assert_eq!(blend_rgb(BlendMode::DarkerColor, dark, light), dark);
        assert_eq!(blend_rgb(BlendMode::LighterColor, dark, light), light);
        assert_eq!(blend_rgb(BlendMode::LighterColor, light, dark), light);
    }

    /// Hard Mix drives every channel to an end of the range, which is what
    /// gives it its eight-colour look.
    #[test]
    fn hard_mix_posterizes_to_the_cube_corners() {
        for bs in 0..=10 {
            for ss in 0..=10 {
                let v = blend_channel(BlendMode::HardMix, bs as f32 / 10.0, ss as f32 / 10.0);
                assert!(v == 0.0 || v == 1.0, "{v} is neither corner");
            }
        }
    }

    /// Neutral operands are the cheapest way to catch a transcribed formula:
    /// each mode has a source value that must leave the backdrop alone.
    #[test]
    fn each_mode_has_its_identity_source() {
        let b = 0.4;
        for (mode, neutral) in [
            (BlendMode::Multiply, 1.0),
            (BlendMode::Screen, 0.0),
            (BlendMode::LinearBurn, 1.0),
            (BlendMode::LinearDodge, 0.0),
            (BlendMode::LinearLight, 0.5),
            (BlendMode::Subtract, 0.0),
            (BlendMode::Divide, 1.0),
            (BlendMode::Difference, 0.0),
            (BlendMode::Exclusion, 0.0),
            (BlendMode::Darken, 1.0),
            (BlendMode::Lighten, 0.0),
        ] {
            let out = blend_channel(mode, b, neutral);
            assert!((out - b).abs() < 1e-5, "{mode:?} at {neutral} gave {out}");
        }
    }
}
