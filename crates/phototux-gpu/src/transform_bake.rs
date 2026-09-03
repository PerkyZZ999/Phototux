//! Commit-only CPU raster transforms (crop / flip / rotate / affine bake).
//!
//! Hot-path preview stays on the GPU composite shader; these helpers run once
//! per user commit so DR-023 (no steady-state CPU canvas upload) is preserved.

use phototux_engine::{CropRect, LayerTransform};

/// Flip tightly packed RGBA8 horizontally or vertically in place.
pub fn flip_rgba(pixels: &mut [u8], width: u32, height: u32, horizontal: bool) {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return;
    }
    if horizontal {
        flip_rgba_horizontal(pixels, w, h);
    } else {
        flip_rgba_vertical(pixels, w, h);
    }
}

fn flip_rgba_horizontal(pixels: &mut [u8], w: usize, h: usize) {
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
}

fn flip_rgba_vertical(pixels: &mut [u8], w: usize, h: usize) {
    for y in 0..(h / 2) {
        let top = y * w * 4;
        let bot = (h - 1 - y) * w * 4;
        for i in 0..(w * 4) {
            pixels.swap(top + i, bot + i);
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

/// Place tightly packed RGBA8 into a new `(dest_width, dest_height)` buffer at
/// `(offset_x, offset_y)`, clipping what falls outside and leaving the rest
/// transparent.
///
/// Canvas Size, which is not a resample: no pixel is resampled, the image is
/// just given more or less room around it. Offsets are signed because
/// shrinking with a centred anchor puts the old top-left outside the new
/// canvas.
///
/// # Errors
/// Returns an error when dimensions or buffer length are invalid.
pub fn place_rgba(
    pixels: &[u8],
    width: u32,
    height: u32,
    dest_width: u32,
    dest_height: u32,
    offset_x: i64,
    offset_y: i64,
) -> Result<Vec<u8>, String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimension overflow".to_owned())?;
    if pixels.len() < expected {
        return Err("source buffer too small".to_owned());
    }
    let out_len = (dest_width as usize)
        .checked_mul(dest_height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimension overflow".to_owned())?;
    let mut out = vec![0_u8; out_len];
    for sy in 0..i64::from(height) {
        let dy = sy + offset_y;
        if dy < 0 || dy >= i64::from(dest_height) {
            continue;
        }
        // Clip the row once rather than testing every pixel in it.
        let sx0 = (-offset_x).max(0);
        let sx1 = i64::from(width).min(i64::from(dest_width) - offset_x);
        if sx1 <= sx0 {
            continue;
        }
        let span = ((sx1 - sx0) as usize) * 4;
        let src = ((sy as usize) * (width as usize) + sx0 as usize) * 4;
        let dst = ((dy as usize) * (dest_width as usize) + (sx0 + offset_x) as usize) * 4;
        out[dst..dst + span].copy_from_slice(&pixels[src..src + span]);
    }
    Ok(out)
}

/// Resample tightly packed RGBA8 to `(dest_width, dest_height)`.
///
/// Bilinear on the way up and a box average on the way down, chosen per axis:
/// bilinear point-samples, so halving an image with it reads every other pixel
/// and drops the rest, which is what turns a downscale into aliasing. The box
/// covers the whole source footprint of each destination pixel.
///
/// # Errors
/// Returns an error when dimensions or buffer length are invalid.
pub fn resize_rgba(
    pixels: &[u8],
    width: u32,
    height: u32,
    dest_width: u32,
    dest_height: u32,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 || dest_width == 0 || dest_height == 0 {
        return Err("resample needs a non-empty source and destination".to_owned());
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimension overflow".to_owned())?;
    if pixels.len() < expected {
        return Err("source buffer too small".to_owned());
    }
    let out_len = (dest_width as usize)
        .checked_mul(dest_height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimension overflow".to_owned())?;
    let mut out = vec![0_u8; out_len];
    let sx = f64::from(width) / f64::from(dest_width);
    let sy = f64::from(height) / f64::from(dest_height);
    let shrinking = sx > 1.0 || sy > 1.0;
    for y in 0..dest_height {
        for x in 0..dest_width {
            let dst = ((y as usize) * (dest_width as usize) + x as usize) * 4;
            let rgba = if shrinking {
                box_average(pixels, width, height, x, y, sx, sy)
            } else {
                // Sample the centre of the destination pixel mapped back into
                // source space; the half-pixel shifts are what keep the image
                // from creeping half a pixel toward the origin.
                //
                // Clamped to the edge because `sample_bilinear` treats outside
                // as transparent — right for baking a transform, where content
                // genuinely leaves the canvas, and wrong here, where it would
                // fade the border of every upscale into nothing.
                let fx = ((f64::from(x) + 0.5) * sx - 0.5).clamp(0.0, f64::from(width - 1));
                let fy = ((f64::from(y) + 0.5) * sy - 0.5).clamp(0.0, f64::from(height - 1));
                sample_bilinear(pixels, width, height, fx as f32, fy as f32)
            };
            out[dst..dst + 4].copy_from_slice(&rgba);
        }
    }
    Ok(out)
}

/// Average the source footprint of destination pixel `(x, y)`.
fn box_average(
    pixels: &[u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    sx: f64,
    sy: f64,
) -> [u8; 4] {
    let x0 = (f64::from(x) * sx).floor().max(0.0) as u32;
    let y0 = (f64::from(y) * sy).floor().max(0.0) as u32;
    let x1 = (((f64::from(x) + 1.0) * sx).ceil() as u32).clamp(x0 + 1, width);
    let y1 = (((f64::from(y) + 1.0) * sy).ceil() as u32).clamp(y0 + 1, height);
    let mut sum = [0_u64; 4];
    let mut count = 0_u64;
    for sy in y0..y1 {
        for sx in x0..x1 {
            let i = ((sy as usize) * (width as usize) + sx as usize) * 4;
            for c in 0..4 {
                sum[c] += u64::from(pixels[i + c]);
            }
            count += 1;
        }
    }
    if count == 0 {
        return [0, 0, 0, 0];
    }
    [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
        (sum[3] / count) as u8,
    ]
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

#[cfg(test)]
mod place_tests {
    use super::place_rgba;

    fn ramp(w: u32, h: u32) -> Vec<u8> {
        (0..(w * h))
            .flat_map(|i| {
                let v = (i % 251) as u8;
                [v, v, v, 255]
            })
            .collect()
    }

    #[test]
    fn growing_keeps_every_source_pixel_and_pads_the_rest() {
        let src = ramp(2, 2);
        let out = place_rgba(&src, 2, 2, 4, 4, 1, 1).expect("place");
        assert_eq!(out.len(), 4 * 4 * 4);
        // The 2×2 lands at (1, 1).
        for y in 0..2_usize {
            for x in 0..2_usize {
                let s = (y * 2 + x) * 4;
                let d = ((y + 1) * 4 + (x + 1)) * 4;
                assert_eq!(&out[d..d + 4], &src[s..s + 4], "pixel {x},{y}");
            }
        }
        // The border is untouched, which means transparent.
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);
    }

    /// A negative offset is a crop, not a panic.
    #[test]
    fn shrinking_clips_what_falls_outside() {
        let src = ramp(4, 4);
        let out = place_rgba(&src, 4, 4, 2, 2, -1, -1).expect("place");
        assert_eq!(out.len(), 2 * 2 * 4);
        // Destination (0, 0) is source (1, 1) in a 4-wide buffer.
        let s = (4 + 1) * 4;
        assert_eq!(&out[0..4], &src[s..s + 4]);
    }

    #[test]
    fn an_offset_past_the_canvas_leaves_it_empty() {
        let src = ramp(2, 2);
        let out = place_rgba(&src, 2, 2, 2, 2, 5, 5).expect("place");
        assert!(out.iter().all(|&b| b == 0));
    }

    #[test]
    fn place_rejects_a_short_buffer() {
        assert!(place_rgba(&[0; 4], 4, 4, 4, 4, 0, 0).is_err());
    }
}

#[cfg(test)]
mod resize_tests {
    use super::resize_rgba;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        (0..(w * h)).flat_map(|_| rgba).collect()
    }

    #[test]
    fn a_resize_keeps_a_flat_colour_flat() {
        let src = solid(4, 4, [10, 200, 30, 255]);
        for (w, h) in [(8, 8), (2, 2), (7, 3)] {
            let out = resize_rgba(&src, 4, 4, w, h).expect("resize");
            assert_eq!(out.len() as u32, w * h * 4);
            assert!(
                out.chunks_exact(4).all(|px| px == [10, 200, 30, 255]),
                "{w}x{h} did not stay flat"
            );
        }
    }

    /// Halving must average, not point-sample.
    ///
    /// A one-pixel checkerboard reduced by point sampling reads every other
    /// pixel and returns one of the two colours; averaged, it returns the
    /// midpoint. This is the difference between a downscale and aliasing.
    #[test]
    fn shrinking_averages_rather_than_dropping_pixels() {
        let mut src = Vec::new();
        for y in 0..2 {
            for x in 0..2 {
                let v = if (x + y) % 2 == 0 { 0 } else { 255 };
                src.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let out = resize_rgba(&src, 2, 2, 1, 1).expect("resize");
        assert_eq!(out[0], 127, "expected the midpoint, got {}", out[0]);
    }

    #[test]
    fn a_resize_rejects_an_empty_target() {
        let src = solid(2, 2, [1, 2, 3, 4]);
        assert!(resize_rgba(&src, 2, 2, 0, 4).is_err());
        assert!(resize_rgba(&src, 2, 2, 4, 0).is_err());
    }

    #[test]
    fn a_resize_rejects_a_short_buffer() {
        assert!(resize_rgba(&[0; 4], 4, 4, 2, 2).is_err());
    }
}
