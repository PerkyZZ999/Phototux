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
