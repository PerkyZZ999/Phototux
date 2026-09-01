//! Downsampling a composite to something a panel can show.
//!
//! The Navigator's whole purpose is to say *where in the image* the viewport
//! is, and it drew a flat rectangle — so it told the user where they were
//! relative to nothing. Showing the picture needs the composite at panel size,
//! and the composite is a GPU readback the host performs; deciding how to
//! shrink it is arithmetic, so it lives here where it can be tested without a
//! device.
//!
//! A box filter rather than nearest-neighbour. A thumbnail is a summary, and
//! dropping fifteen of every sixteen pixels makes a summary that flickers as
//! the picture changes underneath it — thin lines appear and vanish depending
//! on which pixels the stride happens to land on.

/// A downsampled RGBA8 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

/// Shrink `pixels` so neither edge exceeds `max_edge`, averaging each block.
///
/// Returns `None` when the input is not a complete RGBA8 buffer of that size,
/// or when either dimension is zero — a caller that cannot say what it is
/// handing over gets nothing back rather than a guess.
///
/// An image already within `max_edge` is returned unchanged rather than
/// upscaled: a Navigator that magnified a 32×32 document would be showing the
/// user something the document does not look like.
#[must_use]
pub fn downsample_rgba8(
    pixels: &[u8],
    width: u32,
    height: u32,
    max_edge: u32,
) -> Option<Thumbnail> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || max_edge == 0 || pixels.len() < w * h * 4 {
        return None;
    }
    let longest = width.max(height);
    if longest <= max_edge {
        return Some(Thumbnail {
            width,
            height,
            pixels: pixels[..w * h * 4].to_vec(),
        });
    }
    // Integer block size, so every output pixel averages the same shape and the
    // walk needs no per-pixel division.
    let block = longest.div_ceil(max_edge) as usize;
    let out_w = w.div_ceil(block);
    let out_h = h.div_ceil(block);
    let mut out = Vec::with_capacity(out_w * out_h * 4);
    for by in 0..out_h {
        for bx in 0..out_w {
            let x0 = bx * block;
            let y0 = by * block;
            let x1 = (x0 + block).min(w);
            let y1 = (y0 + block).min(h);
            let mut sums = [0_u32; 4];
            let mut count = 0_u32;
            for y in y0..y1 {
                let row = y * w * 4;
                for x in x0..x1 {
                    let i = row + x * 4;
                    for (c, sum) in sums.iter_mut().enumerate() {
                        *sum += u32::from(pixels[i + c]);
                    }
                    count += 1;
                }
            }
            // `count` cannot be zero: the block always overlaps the image, or
            // `out_w`/`out_h` would not have produced it.
            for sum in sums {
                out.push((sum / count) as u8);
            }
        }
    }
    Some(Thumbnail {
        width: out_w as u32,
        height: out_h as u32,
        pixels: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `w`×`h` where every pixel is `rgba`.
    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        rgba.iter()
            .copied()
            .cycle()
            .take((w * h * 4) as usize)
            .collect()
    }

    #[test]
    fn a_small_image_is_returned_unchanged_rather_than_magnified() {
        // A Navigator that upscaled a 32×32 document would show the user
        // something the document does not look like.
        let px = solid(32, 16, [10, 20, 30, 255]);
        let out = downsample_rgba8(&px, 32, 16, 64).expect("thumbnail");
        assert_eq!((out.width, out.height), (32, 16));
        assert_eq!(out.pixels, px);
    }

    #[test]
    fn the_long_edge_is_brought_within_the_limit() {
        let px = solid(1920, 1080, [1, 2, 3, 255]);
        let out = downsample_rgba8(&px, 1920, 1080, 200).expect("thumbnail");
        assert!(out.width <= 200 && out.height <= 200, "{out:?}");
        assert_eq!(out.pixels.len(), (out.width * out.height * 4) as usize);
    }

    #[test]
    fn the_aspect_ratio_survives() {
        // The viewport rectangle is drawn over this, so a thumbnail with the
        // wrong proportions would put the rectangle in the wrong place.
        let px = solid(1600, 400, [0, 0, 0, 255]);
        let out = downsample_rgba8(&px, 1600, 400, 100).expect("thumbnail");
        let source = 1600.0 / 400.0;
        let result = f64::from(out.width) / f64::from(out.height);
        assert!((source - result).abs() < 0.2, "{result} is not {source}");
    }

    #[test]
    fn a_solid_image_downsamples_to_the_same_colour() {
        let px = solid(400, 400, [200, 100, 50, 255]);
        let out = downsample_rgba8(&px, 400, 400, 40).expect("thumbnail");
        for chunk in out.pixels.chunks_exact(4) {
            assert_eq!(chunk, [200, 100, 50, 255]);
        }
    }

    #[test]
    fn a_block_is_averaged_rather_than_sampled() {
        // Half black, half white in a 2×1 block. Nearest-neighbour would give
        // one or the other; the average is the point of the box filter, and it
        // is what stops thin lines flickering in and out as the stride moves.
        let mut px = Vec::new();
        for _ in 0..2 {
            px.extend_from_slice(&[0, 0, 0, 255]);
            px.extend_from_slice(&[255, 255, 255, 255]);
        }
        let out = downsample_rgba8(&px, 2, 2, 1).expect("thumbnail");
        assert_eq!((out.width, out.height), (1, 1));
        assert_eq!(out.pixels, vec![127, 127, 127, 255]);
    }

    #[test]
    fn a_ragged_edge_block_averages_only_the_pixels_that_exist() {
        // 3 wide into blocks of 2 leaves a one-pixel column. Averaging it
        // against a phantom black pixel would darken the right edge.
        let px = solid(3, 1, [80, 80, 80, 255]);
        let out = downsample_rgba8(&px, 3, 1, 2).expect("thumbnail");
        assert_eq!((out.width, out.height), (2, 1));
        assert_eq!(&out.pixels[4..8], &[80, 80, 80, 255]);
    }

    #[test]
    fn an_incomplete_buffer_is_refused() {
        assert!(downsample_rgba8(&[0; 16], 100, 100, 32).is_none());
        assert!(downsample_rgba8(&[], 0, 0, 32).is_none());
        assert!(downsample_rgba8(&solid(4, 4, [0; 4]), 4, 4, 0).is_none());
    }

    #[test]
    fn a_four_k_composite_shrinks_to_a_panel_sized_image() {
        // The case this exists for. Asserted on dimensions rather than timing,
        // which is what the host's throttle is for.
        let px = solid(3840, 2160, [12, 34, 56, 255]);
        let out = downsample_rgba8(&px, 3840, 2160, 200).expect("thumbnail");
        assert!(out.width <= 200 && out.height <= 200);
        assert!(
            out.pixels.len() < px.len() / 300,
            "{} bytes is not much smaller than {}",
            out.pixels.len(),
            px.len()
        );
    }
}
