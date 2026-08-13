//! Rasterize a [`ShapeContent`] to RGBA8 (handbook 19 / DR-027).
//!
//! This logic lived in a helper on the qtbridge `AppSession`, calling only
//! engine functions and touching neither Qt nor the GPU. DR-022 requires the
//! core to be headless-testable, and nothing inside a `#[qobject]` is: the
//! object cannot be constructed in a unit test. Moving it here puts shape
//! rasterization in the crate that owns `ShapeContent`, and in the crate with
//! the test suite.

use crate::layer::ShapeContent;
use crate::paths::{fill_gradient_even_odd, rasterize_shape_rgba8, stroke_path_rgba8};
use crate::shape_boolean::{BooleanOp, boolean_rgba8};

/// Convert a linear f32 RGBA quad to 8-bit, clamping out-of-gamut input.
///
/// The caller open-coded this conversion six times in one function — for the
/// stroke, the flat fill, both gradient stops and the boolean operand — so a
/// rounding change had six places to miss.
#[must_use]
pub fn rgba_f32_to_u8(rgba: [f32; 4]) -> [u8; 4] {
    [
        (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

/// Rasterize `content` into a `width * height * 4` RGBA8 buffer.
///
/// A gradient fill takes a different route from a flat fill: the gradient is
/// painted through the path's even-odd interior and the stroke is then composited
/// over it, whereas a flat fill hands both to `rasterize_shape_rgba8` in one
/// pass. A boolean partner, when present, is rasterized separately and combined
/// last, so the operand is a finished shape rather than a half-painted buffer.
///
/// # Errors
/// Returns a message when the dimensions overflow or a rasterization step fails.
pub fn rasterize_shape_content(
    content: &ShapeContent,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let stroke = content.stroked.then(|| rgba_f32_to_u8(content.stroke_rgba));

    let mut base = match content.gradient.as_ref() {
        Some(gradient) => {
            let len = (width as usize)
                .checked_mul(height as usize)
                .and_then(|n| n.checked_mul(4))
                .ok_or_else(|| "dimensions overflow".to_owned())?;
            let mut out = vec![0_u8; len];
            if content.filled {
                fill_gradient_even_odd(
                    &mut out,
                    width,
                    height,
                    &content.path,
                    gradient.x0,
                    gradient.y0,
                    gradient.x1,
                    gradient.y1,
                    rgba_f32_to_u8(gradient.c0_rgba),
                    rgba_f32_to_u8(gradient.c1_rgba),
                );
            }
            if let Some(stroke) = stroke {
                let stroked =
                    stroke_path_rgba8(width, height, &content.path, stroke, content.stroke_width)?;
                for (dst, src) in out.chunks_exact_mut(4).zip(stroked.chunks_exact(4)) {
                    if src[3] > 0 {
                        dst.copy_from_slice(src);
                    }
                }
            }
            out
        }
        None => {
            let fill = content.filled.then(|| rgba_f32_to_u8(content.fill_rgba));
            rasterize_shape_rgba8(
                width,
                height,
                &content.path,
                fill,
                stroke,
                content.stroke_width,
            )?
        }
    };

    if let Some(partner) = content.boolean_partner.as_ref() {
        let op = BooleanOp::parse(&partner.op).unwrap_or(BooleanOp::Union);
        let operand = rasterize_shape_rgba8(
            width,
            height,
            &partner.path,
            Some(rgba_f32_to_u8(partner.fill_rgba)),
            None,
            0.0,
        )?;
        base = boolean_rgba8(&base, &operand, op)?;
    }

    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{ShapeBooleanPartner, ShapeGradient};
    use crate::paths::VectorPath;

    fn square(size: f32) -> VectorPath {
        VectorPath::polyline(
            "Square",
            vec![
                crate::paths::PathPoint { x: 0.0, y: 0.0 },
                crate::paths::PathPoint { x: size, y: 0.0 },
                crate::paths::PathPoint { x: size, y: size },
                crate::paths::PathPoint { x: 0.0, y: size },
            ],
            true,
        )
    }

    fn filled_square() -> ShapeContent {
        ShapeContent {
            path: square(8.0),
            fill_rgba: [1.0, 0.0, 0.0, 1.0],
            filled: true,
            stroked: false,
            ..ShapeContent::default()
        }
    }

    #[test]
    fn rgba_conversion_clamps_out_of_gamut_input() {
        assert_eq!(rgba_f32_to_u8([1.5, -0.5, 0.5, 1.0]), [255, 0, 128, 255]);
    }

    #[test]
    fn a_filled_shape_paints_its_interior() {
        let out = rasterize_shape_content(&filled_square(), 8, 8).expect("rasterize");
        assert_eq!(out.len(), 8 * 8 * 4);
        let centre = (4 * 8 + 4) * 4;
        assert_eq!(out[centre + 3], 255, "interior should be opaque");
        assert_eq!(out[centre], 255, "interior should carry the fill colour");
    }

    #[test]
    fn an_unfilled_unstroked_shape_paints_nothing() {
        let content = ShapeContent {
            filled: false,
            stroked: false,
            ..filled_square()
        };
        let out = rasterize_shape_content(&content, 8, 8).expect("rasterize");
        assert!(
            out.chunks_exact(4).all(|px| px[3] == 0),
            "nothing to paint should leave the buffer clear"
        );
    }

    /// A gradient fill goes through `fill_gradient_even_odd` rather than the
    /// flat-fill path, so the two ends of the ramp must differ.
    #[test]
    fn a_gradient_fill_varies_across_the_shape() {
        let content = ShapeContent {
            gradient: Some(ShapeGradient {
                x0: 0.0,
                y0: 0.0,
                x1: 8.0,
                y1: 0.0,
                c0_rgba: [0.0, 0.0, 0.0, 1.0],
                c1_rgba: [1.0, 1.0, 1.0, 1.0],
            }),
            ..filled_square()
        };
        let out = rasterize_shape_content(&content, 8, 8).expect("rasterize");
        let row = 4 * 8 * 4;
        let left = out[row + 4];
        let right = out[row + 6 * 4];
        assert!(
            right > left,
            "gradient should ramp left to right, got {left} then {right}"
        );
    }

    /// The boolean operand is rasterized on its own and combined last, so a
    /// subtract must remove coverage the base had painted.
    #[test]
    fn a_boolean_subtract_removes_coverage() {
        let plain = rasterize_shape_content(&filled_square(), 8, 8).expect("rasterize");
        let content = ShapeContent {
            boolean_partner: Some(ShapeBooleanPartner {
                op: "subtract".into(),
                path: square(8.0),
                fill_rgba: [0.0, 1.0, 0.0, 1.0],
            }),
            ..filled_square()
        };
        let out = rasterize_shape_content(&content, 8, 8).expect("rasterize");
        let painted = |buf: &[u8]| buf.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(painted(&plain) > 0, "base shape should paint something");
        assert!(
            painted(&out) < painted(&plain),
            "subtracting an overlapping shape must reduce coverage"
        );
    }

    #[test]
    fn overflowing_dimensions_are_reported_not_panicked() {
        let content = ShapeContent {
            gradient: Some(ShapeGradient {
                x0: 0.0,
                y0: 0.0,
                x1: 1.0,
                y1: 0.0,
                c0_rgba: [0.0; 4],
                c1_rgba: [1.0; 4],
            }),
            ..filled_square()
        };
        assert!(rasterize_shape_content(&content, u32::MAX, u32::MAX).is_err());
    }
}
