//! One-shot fill and linear gradient into layer RGBA (commit path; ADR-005).

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
pub fn linear_gradient_rgba(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p0: [f32; 2],
    p1: [f32; 2],
    c0: [f32; 4],
    c1: [f32; 4],
    mask: Option<&[u8]>,
) {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return;
    }
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-6 {
        fill_rgba(pixels, c0, mask);
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
            let t = ((x as f32 + 0.5 - p0[0]) * dx + (y as f32 + 0.5 - p0[1]) * dy) / len2;
            let t = t.clamp(0.0, 1.0);
            let color = [
                c0[0] + (c1[0] - c0[0]) * t,
                c0[1] + (c1[1] - c0[1]) * t,
                c0[2] + (c1[2] - c0[2]) * t,
                c0[3] + (c1[3] - c0[3]) * t,
            ];
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
        linear_gradient_rgba(
            &mut px,
            3,
            1,
            [0.0, 0.0],
            [2.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
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
