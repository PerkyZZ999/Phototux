//! Free-transform / crop / resize commands (transform chrome slice).

use serde::{Deserialize, Serialize};

use crate::layer::{LayerId, LayerTransform};

/// Row-major 2×3 affine: `x' = a*x + b*y + tx`, `y' = c*x + d*y + ty`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine2 {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Affine2 {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub fn map_point(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.b * y + self.tx,
            self.c * x + self.d * y + self.ty,
        )
    }

    pub fn inverse(self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if !det.is_finite() || det.abs() < 1e-8 {
            return None;
        }
        let inv_det = 1.0 / det;
        let a = self.d * inv_det;
        let b = -self.b * inv_det;
        let c = -self.c * inv_det;
        let d = self.a * inv_det;
        let tx = -(a * self.tx + b * self.ty);
        let ty = -(c * self.tx + d * self.ty);
        Some(Self { a, b, c, d, tx, ty })
    }
}

impl LayerTransform {
    pub fn identity() -> Self {
        Self::default()
    }

    pub fn is_identity(self) -> bool {
        self.translate_x.abs() < 1e-5
            && self.translate_y.abs() < 1e-5
            && (self.scale_x - 1.0).abs() < 1e-5
            && (self.scale_y - 1.0).abs() < 1e-5
            && self.rotation_deg.abs() < 1e-5
    }

    /// Forward affine mapping source pixel → destination pixel around `pivot`.
    pub fn forward_affine(self, pivot_x: f32, pivot_y: f32) -> Affine2 {
        let rad = self.rotation_deg.to_radians();
        let (sin, cos) = rad.sin_cos();
        let a = cos * self.scale_x;
        let b = -sin * self.scale_y;
        let c = sin * self.scale_x;
        let d = cos * self.scale_y;
        let tx = pivot_x + self.translate_x - (a * pivot_x + b * pivot_y);
        let ty = pivot_y + self.translate_y - (c * pivot_x + d * pivot_y);
        Affine2 { a, b, c, d, tx, ty }
    }

    /// Inverse affine for sampling destination → source (identity if singular).
    pub fn inverse_affine(self, pivot_x: f32, pivot_y: f32) -> Affine2 {
        self.forward_affine(pivot_x, pivot_y)
            .inverse()
            .unwrap_or(Affine2::IDENTITY)
    }

    pub fn flip_horizontal(mut self) -> Self {
        self.scale_x = -self.scale_x;
        self
    }

    pub fn flip_vertical(mut self) -> Self {
        self.scale_y = -self.scale_y;
        self
    }

    /// Smallest magnitude a draft scale may take.
    ///
    /// Zero collapses the layer to nothing, and once collapsed the gizmo has
    /// no handles left to drag it back out with — so the draft is never
    /// allowed to reach it.
    pub const MIN_DRAFT_SCALE: f32 = 0.01;

    /// Clean up an in-progress transform draft from the gizmo.
    ///
    /// Two things happen, in this order, and the order is the point.
    ///
    /// The scale is clamped by **magnitude**, keeping its sign. Clamping the
    /// value instead — `scale_x.max(0.01)` — also silently rectifies a
    /// negative scale to a positive one, which is a mirrored layer quietly
    /// becoming unmirrored. The shipped gizmo never sends a negative (it
    /// scales by distance from the centre, and takes `abs` first), but
    /// `LayerTransform` is serialised into `.ptx`, so a negative can arrive
    /// from a file and reach here on the first drag. Clamping the magnitude
    /// costs nothing and keeps the sign meaningful.
    ///
    /// Then, when `constrain` is set, both axes take the larger magnitude
    /// while keeping their own signs — so constraining the aspect of a
    /// mirrored layer does not also unmirror it.
    ///
    /// A non-finite scale carries no usable magnitude, so it falls back to the
    /// floor rather than reaching the affine maths, where an infinite scale
    /// produces a singular matrix that `inverse_affine` silently replaces with
    /// the identity — a transform that appears to have been discarded.
    #[must_use]
    pub fn with_usable_scale(mut self, constrain: bool) -> Self {
        self.scale_x = clamp_scale_magnitude(self.scale_x);
        self.scale_y = clamp_scale_magnitude(self.scale_y);
        if constrain {
            let uniform = self.scale_x.abs().max(self.scale_y.abs());
            self.scale_x = uniform.copysign(self.scale_x);
            self.scale_y = uniform.copysign(self.scale_y);
        }
        self
    }
}

/// Push a scale factor out to at least [`LayerTransform::MIN_DRAFT_SCALE`]
/// without changing which way it faces.
fn clamp_scale_magnitude(scale: f32) -> f32 {
    if !scale.is_finite() {
        // Explicit rather than leaning on `f32::max`, which happens to swallow
        // NaN but passes infinity straight through.
        return LayerTransform::MIN_DRAFT_SCALE.copysign(scale);
    }
    scale
        .abs()
        .max(LayerTransform::MIN_DRAFT_SCALE)
        .copysign(scale)
}

/// Preview state while a transform tool is active (commit = one history entry).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformPreview {
    pub layer_id: u64,
    pub draft: LayerTransform,
    pub constrain_aspect: bool,
}

/// Active free-transform editing session.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformSession {
    pub layer_id: LayerId,
    pub baseline: LayerTransform,
    pub draft: LayerTransform,
    pub constrain_aspect: bool,
}

impl TransformSession {
    pub fn new(layer_id: LayerId, baseline: LayerTransform) -> Self {
        Self {
            layer_id,
            baseline,
            draft: baseline,
            constrain_aspect: false,
        }
    }
}

/// Crop rectangle in document pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Image / canvas resize request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResizeRequest {
    pub width: u32,
    pub height: u32,
    /// When true, scales layer pixels; when false, only canvas bounds change.
    pub scale_content: bool,
}

impl CropRect {
    pub fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }

    pub fn clamp_to(self, doc_w: u32, doc_h: u32) -> Option<Self> {
        if !self.is_valid() {
            return None;
        }
        let doc_wi = i32::try_from(doc_w).unwrap_or(i32::MAX);
        let doc_hi = i32::try_from(doc_h).unwrap_or(i32::MAX);
        let x0 = self.x.clamp(0, doc_wi);
        let y0 = self.y.clamp(0, doc_hi);
        let x1 = self
            .x
            .saturating_add(i32::try_from(self.width).unwrap_or(i32::MAX))
            .clamp(0, doc_wi);
        let y1 = self
            .y
            .saturating_add(i32::try_from(self.height).unwrap_or(i32::MAX))
            .clamp(0, doc_hi);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(Self {
            x: x0,
            y: y0,
            width: u32::try_from(x1 - x0).unwrap_or(0),
            height: u32::try_from(y1 - y0).unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_inverse() {
        let t = LayerTransform::identity();
        let inv = t.inverse_affine(100.0, 50.0);
        let (x, y) = inv.map_point(10.0, 20.0);
        assert!((x - 10.0).abs() < 1e-4);
        assert!((y - 20.0).abs() < 1e-4);
    }

    #[test]
    fn translate_forward() {
        let t = LayerTransform {
            translate_x: 10.0,
            translate_y: -5.0,
            ..LayerTransform::identity()
        };
        let m = t.forward_affine(0.0, 0.0);
        let (x, y) = m.map_point(0.0, 0.0);
        assert!((x - 10.0).abs() < 1e-4);
        assert!((y + 5.0).abs() < 1e-4);
    }

    #[test]
    fn flip_helpers() {
        let t = LayerTransform::identity().flip_horizontal();
        assert!((t.scale_x + 1.0).abs() < 1e-5);
        let t = LayerTransform::identity().flip_vertical();
        assert!((t.scale_y + 1.0).abs() < 1e-5);
    }

    fn scaled(scale_x: f32, scale_y: f32) -> LayerTransform {
        LayerTransform {
            scale_x,
            scale_y,
            ..LayerTransform::identity()
        }
    }

    #[test]
    fn a_usable_draft_leaves_an_ordinary_scale_alone() {
        let t = scaled(1.5, 0.75).with_usable_scale(false);
        assert!((t.scale_x - 1.5).abs() < 1e-6);
        assert!((t.scale_y - 0.75).abs() < 1e-6);
    }

    #[test]
    fn a_collapsed_scale_is_pushed_out_to_the_floor() {
        let t = scaled(0.0, 0.0001).with_usable_scale(false);
        assert!((t.scale_x - LayerTransform::MIN_DRAFT_SCALE).abs() < 1e-6);
        assert!((t.scale_y - LayerTransform::MIN_DRAFT_SCALE).abs() < 1e-6);
    }

    #[test]
    fn a_mirrored_layer_stays_mirrored() {
        // The old clamp was `scale.max(MIN)`, which turned every negative into
        // +0.01 — a mirrored layer silently unmirroring on the first drag.
        let t = scaled(-2.0, -0.5).with_usable_scale(false);
        assert!(t.scale_x < 0.0, "scale_x flipped sign: {}", t.scale_x);
        assert!(t.scale_y < 0.0, "scale_y flipped sign: {}", t.scale_y);
        assert!((t.scale_x + 2.0).abs() < 1e-6);
        assert!((t.scale_y + 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_mirrored_scale_too_small_to_use_keeps_its_direction() {
        let t = scaled(-0.0001, 3.0).with_usable_scale(false);
        assert!((t.scale_x + LayerTransform::MIN_DRAFT_SCALE).abs() < 1e-6);
    }

    #[test]
    fn constraining_takes_the_larger_magnitude_on_both_axes() {
        let t = scaled(0.5, 2.0).with_usable_scale(true);
        assert!((t.scale_x - 2.0).abs() < 1e-6);
        assert!((t.scale_y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn constraining_a_mirrored_layer_does_not_unmirror_it() {
        // Each axis takes the shared magnitude but keeps its own direction.
        let t = scaled(-0.5, 2.0).with_usable_scale(true);
        assert!((t.scale_x + 2.0).abs() < 1e-6, "scale_x was {}", t.scale_x);
        assert!((t.scale_y - 2.0).abs() < 1e-6, "scale_y was {}", t.scale_y);
    }

    #[test]
    fn a_non_finite_scale_is_treated_as_degenerate() {
        let t = scaled(f32::NAN, f32::INFINITY).with_usable_scale(false);
        assert!(
            t.scale_x.is_finite() && t.scale_y.is_finite(),
            "a draft escaped with a non-finite scale: {} {}",
            t.scale_x,
            t.scale_y
        );
        assert!((t.scale_x.abs() - LayerTransform::MIN_DRAFT_SCALE).abs() < 1e-6);
    }

    #[test]
    fn crop_clamp() {
        let c = CropRect {
            x: -10,
            y: 10,
            width: 100,
            height: 50,
        }
        .clamp_to(80, 40)
        .expect("clamp");
        assert_eq!(c.x, 0);
        assert_eq!(c.y, 10);
        assert_eq!(c.width, 80);
        assert_eq!(c.height, 30);
    }
}
