//! One-shot fill and gradient into layer RGBA (commit path; ADR-005).

use phototux_engine::GradientRamp;

/// Solid fill. When `mask` is present (R8, same pixel count), blend by mask coverage.
pub fn fill_rgba(pixels: &mut [u8], color: [f32; 4], mask: Option<&[u8]>) {
    let rgba = color_to_u8(color);
    match mask {
        None => {
            for px in pixels.chunks_exact_mut(4) {
                px.copy_from_slice(&rgba);
            }
        }
        Some(mask) => {
            let n = pixels.len() / 4;
            for i in 0..n {
                let m = f32::from(*mask.get(i).unwrap_or(&0)) / 255.0;
                if m < 1e-4 {
                    continue;
                }
                let base = i * 4;
                for c in 0..4 {
                    let src = f32::from(pixels[base + c]);
                    let dst = f32::from(rgba[c]);
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "blend result clamped to byte"
                    )]
                    {
                        pixels[base + c] = (src + (dst - src) * m).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}

/// Linear gradient from `p0`→`p1` in document pixels, colors `c0`→`c1` (RGBA 0..1).
pub fn gradient_rgba(
    ramp: GradientRamp,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    mask: Option<&[u8]>,
) {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return;
    }
    if !ramp.has_direction() {
        fill_rgba(pixels, ramp.start_rgba, mask);
        return;
    }
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let m = mask
                .map(|mask| f32::from(*mask.get(idx).unwrap_or(&0)) / 255.0)
                .unwrap_or(1.0);
            if m < 1e-4 {
                continue;
            }
            // Which shape the drag sweeps, and what colour that means, are
            // document policy and live on `GradientRamp`; this walk only
            // blends the answer into the buffer.
            let color = ramp.color_at(x as f32 + 0.5, y as f32 + 0.5);
            let rgba = color_to_u8(color);
            let base = idx * 4;
            for c in 0..4 {
                let src = f32::from(pixels[base + c]);
                let dst = f32::from(rgba[c]);
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "blend result clamped to byte"
                )]
                {
                    pixels[base + c] = (src + (dst - src) * m).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

/// Sample opaque RGB at integer document coordinates.
pub fn sample_rgba_at(pixels: &[u8], width: u32, height: u32, x: i32, y: i32) -> Option<[f32; 3]> {
    if x < 0 || y < 0 {
        return None;
    }
    let x = x as u32;
    let y = y as u32;
    if x >= width || y >= height {
        return None;
    }
    let idx = (y as usize * width as usize + x as usize) * 4;
    if idx + 3 >= pixels.len() {
        return None;
    }
    Some([
        f32::from(pixels[idx]) / 255.0,
        f32::from(pixels[idx + 1]) / 255.0,
        f32::from(pixels[idx + 2]) / 255.0,
    ])
}

/// True when any mask byte is selected.
pub fn mask_has_selection(mask: &[u8]) -> bool {
    mask.iter().any(|&v| v > 0)
}

fn color_to_u8(color: [f32; 4]) -> [u8; 4] {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "channels clamped to unit interval"
    )]
    [
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use phototux_engine::GradientKind;

    #[test]
    fn fill_solid_overwrites() {
        let mut px = vec![0_u8; 4];
        fill_rgba(&mut px, [1.0, 0.0, 0.0, 1.0], None);
        assert_eq!(px, [255, 0, 0, 255]);
    }

    #[test]
    fn fill_respects_mask() {
        let mut px = vec![0_u8, 0, 0, 255, 10, 20, 30, 255];
        let mask = [255_u8, 0];
        fill_rgba(&mut px, [1.0, 1.0, 1.0, 1.0], Some(&mask));
        assert_eq!(&px[..4], &[255, 255, 255, 255]);
        assert_eq!(&px[4..], &[10, 20, 30, 255]);
    }

    #[test]
    fn gradient_midpoint_mixes() {
        let mut px = vec![0_u8; 3 * 4];
        gradient_rgba(
            GradientRamp {
                kind: GradientKind::Linear,
                start: [0.0, 0.0],
                end: [2.0, 0.0],
                start_rgba: [0.0, 0.0, 0.0, 1.0],
                end_rgba: [1.0, 0.0, 0.0, 1.0],
            },
            &mut px,
            3,
            1,
            None,
        );
        assert!(px[4] > 50 && px[4] < 200);
    }

    #[test]
    fn sample_center() {
        let px = [10_u8, 20, 30, 255];
        let rgb = sample_rgba_at(&px, 1, 1, 0, 0).expect("sample");
        assert!((rgb[0] - 10.0 / 255.0).abs() < 1e-4);
    }
}
