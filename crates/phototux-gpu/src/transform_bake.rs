//! Commit-only CPU raster transforms (crop / flip / rotate / affine bake).
//!
//! Hot-path preview stays on the GPU composite shader; these helpers run once
//! per user commit so ADR-005 (no steady-state CPU canvas upload) is preserved.

use phototux_engine::{CropRect, LayerTransform};

/// Flip tightly packed RGBA8 horizontally or vertically in place.
pub fn flip_rgba(pixels: &mut [u8], width: u32, height: u32, horizontal: bool) {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return;
    }
    if horizontal {
        for y in 0..h {
            let row = y * w * 4;
            for x in 0..(w / 2) {
                let left = row + x * 4;
                let right = row + (w - 1 - x) * 4;
                for c in 0..4 {
                    pixels.swap(left + c, right + c);
                }
            }
        }
    } else {
        for y in 0..(h / 2) {
            let top = y * w * 4;
            let bot = (h - 1 - y) * w * 4;
            for i in 0..(w * 4) {
                pixels.swap(top + i, bot + i);
            }
        }
    }
}

/// Crop tightly packed RGBA8 to `rect` (already clamped).
///
/// # Errors
/// Returns an error when dimensions or buffer length are invalid.
pub fn crop_rgba(
    pixels: &[u8],
    width: u32,
    height: u32,
    rect: CropRect,
) -> Result<Vec<u8>, String> {
    let Some(rect) = rect.clamp_to(width, height) else {
        return Err("invalid crop rect".to_owned());
    };
    let src_w = width as usize;
    let expected = src_w
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimension overflow".to_owned())?;
    if pixels.len() < expected {
        return Err("source buffer too small".to_owned());
    }
    let dw = rect.width as usize;
    let dh = rect.height as usize;
    let ox = rect.x as usize;
    let oy = rect.y as usize;
    let mut out = vec![0_u8; dw * dh * 4];
    for y in 0..dh {
        let src_row = (oy + y) * src_w * 4 + ox * 4;
        let dst_row = y * dw * 4;
        out[dst_row..dst_row + dw * 4].copy_from_slice(&pixels[src_row..src_row + dw * 4]);
    }
    Ok(out)
}

/// Rotate tightly packed RGBA8 90° clockwise. Output size is `(height, width)`.
///
/// # Errors
/// Returns an error when dimensions or buffer length are invalid.
pub fn rotate_rgba_90_cw(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    let w = width as usize;
    let h = height as usize;
    let expected = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimension overflow".to_owned())?;
    if pixels.len() < expected {
        return Err("source buffer too small".to_owned());
    }
    let out_w = h;
    let out_h = w;
    let mut out = vec![0_u8; out_w * out_h * 4];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) * 4;
            let dx = h - 1 - y;
            let dy = x;
            let dst = (dy * out_w + dx) * 4;
            out[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
        }
    }
    Ok((height, width, out))
}

fn sample_bilinear(pixels: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    let w = width as i32;
    let h = height as i32;
    if w <= 0 || h <= 0 {
        return [0; 4];
    }
    if x < -0.5 || y < -0.5 || x > (w as f32) - 0.5 || y > (h as f32) - 0.5 {
        // Outside with small margin → transparent
        if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 {
            return [0; 4];
        }
    }
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let fetch = |px: i32, py: i32| -> [f32; 4] {
        if px < 0 || py < 0 || px >= w || py >= h {
            return [0.0; 4];
        }
        let idx = ((py as usize) * (w as usize) + (px as usize)) * 4;
        [
            f32::from(pixels[idx]),
            f32::from(pixels[idx + 1]),
            f32::from(pixels[idx + 2]),
            f32::from(pixels[idx + 3]),
        ]
    };

    let c00 = fetch(x0, y0);
    let c10 = fetch(x1, y0);
    let c01 = fetch(x0, y1);
    let c11 = fetch(x1, y1);
    let mut out = [0_u8; 4];
    for i in 0..4 {
        let top = c00[i] * (1.0 - fx) + c10[i] * fx;
        let bot = c01[i] * (1.0 - fx) + c11[i] * fx;
        let v = top * (1.0 - fy) + bot * fy;
        out[i] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Bake `transform` into a same-sized RGBA buffer (clip to document, bilinear).
///
/// # Errors
/// Returns an error when dimensions or buffer length are invalid.
pub fn bake_affine_rgba(
    pixels: &[u8],
    width: u32,
    height: u32,
    transform: LayerTransform,
) -> Result<Vec<u8>, String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimension overflow".to_owned())?;
    if pixels.len() < expected {
        return Err("source buffer too small".to_owned());
    }
    if transform.is_identity() {
        return Ok(pixels[..expected].to_vec());
    }
    let pivot_x = width as f32 * 0.5;
    let pivot_y = height as f32 * 0.5;
    let inv = transform.inverse_affine(pivot_x, pivot_y);
    let mut out = vec![0_u8; expected];
    for y in 0..height {
        for x in 0..width {
            let (sx, sy) = inv.map_point(x as f32 + 0.5, y as f32 + 0.5);
            let sample = sample_bilinear(pixels, width, height, sx - 0.5, sy - 0.5);
            let dst = ((y as usize) * (width as usize) + (x as usize)) * 4;
            out[dst..dst + 4].copy_from_slice(&sample);
        }
    }
    Ok(out)
}

/// Pack inverse affine coeffs for the composite shader (dest → source pixels).
pub fn inverse_affine_coeffs(
    transform: LayerTransform,
    width: u32,
    height: u32,
) -> (f32, f32, f32, f32, f32, f32) {
    let inv = transform.inverse_affine(width as f32 * 0.5, height as f32 * 0.5);
    (inv.a, inv.b, inv.c, inv.d, inv.tx, inv.ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            v.extend_from_slice(&rgba);
        }
        v
    }

    #[test]
    fn flip_horizontal_swaps_corners() {
        let mut px = vec![
            1, 0, 0, 255, 2, 0, 0, 255, //
            3, 0, 0, 255, 4, 0, 0, 255,
        ];
        flip_rgba(&mut px, 2, 2, true);
        assert_eq!(&px[0..4], &[2, 0, 0, 255]);
        assert_eq!(&px[4..8], &[1, 0, 0, 255]);
    }

    #[test]
    fn crop_extracts_region() {
        let px = solid(4, 4, [10, 20, 30, 255]);
        let cropped = crop_rgba(
            &px,
            4,
            4,
            CropRect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
        )
        .expect("crop");
        assert_eq!(cropped.len(), 2 * 2 * 4);
        assert_eq!(&cropped[0..4], &[10, 20, 30, 255]);
    }

    #[test]
    fn rotate_90_swaps_dims() {
        let mut px = vec![0_u8; 2 * 3 * 4];
        px[0] = 9; // (0,0)
        let (w, h, out) = rotate_rgba_90_cw(&px, 2, 3).expect("rot");
        assert_eq!((w, h), (3, 2));
        // (0,0) -> (2,0) in 3×2 → byte index 8
        assert_eq!(out[8], 9);
    }

    #[test]
    fn identity_bake_unchanged() {
        let px = solid(3, 2, [7, 8, 9, 255]);
        let out = bake_affine_rgba(&px, 3, 2, LayerTransform::identity()).expect("bake");
        assert_eq!(out, px);
    }
}
