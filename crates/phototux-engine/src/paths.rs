//! Vector path document + CPU stroke raster (Phase 4.5).
//!
//! Shape layer kind is a separate graph amend (DR-020). Paths here are free
//! vectors that can stroke to a raster target.

use serde::{Deserialize, Serialize};

/// 2D point in document pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathPoint {
    pub x: f32,
    pub y: f32,
}

/// One subpath (open or closed polyline / cubic chain).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorPath {
    pub name: String,
    pub closed: bool,
    /// Anchor points. Cubics use `controls` in parallel (empty = polyline).
    pub anchors: Vec<PathPoint>,
    /// Optional cubic handles: length `anchors.len()` of (in, out) pairs when used.
    #[serde(default)]
    pub controls: Vec<(PathPoint, PathPoint)>,
}

impl VectorPath {
    pub fn polyline(name: impl Into<String>, anchors: Vec<PathPoint>, closed: bool) -> Self {
        Self {
            name: name.into(),
            closed,
            anchors,
            controls: Vec::new(),
        }
    }
}

/// Document-owned path list (not yet a layer kind).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PathDocument {
    pub paths: Vec<VectorPath>,
    pub active: Option<usize>,
}

impl PathDocument {
    pub fn add(&mut self, path: VectorPath) -> usize {
        self.paths.push(path);
        let idx = self.paths.len() - 1;
        self.active = Some(idx);
        idx
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.active = None;
    }
}

/// Stroke a polyline path into straight RGBA8 (1 px round brush, solid color).
///
/// # Errors
/// Returns an error when dimensions are zero or overflow.
pub fn stroke_path_rgba8(
    width: u32,
    height: u32,
    path: &VectorPath,
    rgba: [u8; 4],
    stroke_width: f32,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("zero dimensions".into());
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimensions overflow".to_owned())?;
    let mut out = vec![0_u8; pixels];
    if path.anchors.len() < 2 {
        return Ok(out);
    }
    let radius = (stroke_width * 0.5).max(0.5);
    let mut segments: Vec<(PathPoint, PathPoint)> =
        path.anchors.windows(2).map(|w| (w[0], w[1])).collect();
    if path.closed {
        if let (Some(&first), Some(&last)) = (path.anchors.first(), path.anchors.last()) {
            segments.push((last, first));
        }
    }
    for (a, b) in segments {
        stamp_segment(&mut out, width as i32, height as i32, a, b, radius, rgba);
    }
    Ok(out)
}

fn stamp_segment(
    out: &mut [u8],
    w: i32,
    h: i32,
    a: PathPoint,
    b: PathPoint,
    radius: f32,
    rgba: [u8; 4],
) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = dx.hypot(dy);
    let steps = (len * 2.0).ceil().max(1.0) as i32;
    let r = radius.ceil() as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = a.x + dx * t;
        let cy = a.y + dy * t;
        let ix = cx.round() as i32;
        let iy = cy.round() as i32;
        for oy in -r..=r {
            for ox in -r..=r {
                if (ox * ox + oy * oy) as f32 > radius * radius {
                    continue;
                }
                let x = ix + ox;
                let y = iy + oy;
                if x < 0 || y < 0 || x >= w || y >= h {
                    continue;
                }
                let o = (y as usize * w as usize + x as usize) * 4;
                out[o] = rgba[0];
                out[o + 1] = rgba[1];
                out[o + 2] = rgba[2];
                out[o + 3] = rgba[3];
            }
        }
    }
}

/// Even-odd fill of a closed polyline into straight RGBA8, then optional stroke.
///
/// # Errors
/// Returns an error when dimensions are zero or overflow.
pub fn rasterize_shape_rgba8(
    width: u32,
    height: u32,
    path: &VectorPath,
    fill_rgba: Option<[u8; 4]>,
    stroke_rgba: Option<[u8; 4]>,
    stroke_width: f32,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("zero dimensions".into());
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimensions overflow".to_owned())?;
    let mut out = vec![0_u8; pixels];
    if let Some(fill) = fill_rgba {
        if path.anchors.len() >= 3 {
            fill_even_odd(&mut out, width, height, path, fill);
        }
    }
    if let Some(stroke) = stroke_rgba {
        let stroked = stroke_path_rgba8(width, height, path, stroke, stroke_width)?;
        for (d, s) in out.chunks_exact_mut(4).zip(stroked.chunks_exact(4)) {
            if s[3] > 0 {
                d.copy_from_slice(s);
            }
        }
    }
    Ok(out)
}

fn fill_even_odd(out: &mut [u8], width: u32, height: u32, path: &VectorPath, rgba: [u8; 4]) {
    let w = width as i32;
    let h = height as i32;
    let pts = &path.anchors;
    let n = pts.len();
    for y in 0..h {
        for x in 0..w {
            let mut inside = false;
            let mut j = n - 1;
            for i in 0..n {
                let yi = pts[i].y;
                let yj = pts[j].y;
                let xi = pts[i].x;
                let xj = pts[j].x;
                if ((yi > y as f32) != (yj > y as f32))
                    && ((x as f32) < (xj - xi) * (y as f32 - yi) / (yj - yi + f32::EPSILON) + xi)
                {
                    inside = !inside;
                }
                j = i;
            }
            if inside {
                let o = (y as usize * width as usize + x as usize) * 4;
                out[o] = rgba[0];
                out[o + 1] = rgba[1];
                out[o + 2] = rgba[2];
                out[o + 3] = rgba[3];
            }
        }
    }
}

/// Axis-aligned rectangle path (closed).
pub fn rect_path(name: impl Into<String>, x: f32, y: f32, w: f32, h: f32) -> VectorPath {
    VectorPath::polyline(
        name,
        vec![
            PathPoint { x, y },
            PathPoint { x: x + w, y },
            PathPoint { x: x + w, y: y + h },
            PathPoint { x, y: y + h },
        ],
        true,
    )
}

/// Approximate ellipse as a closed 32-gon.
pub fn ellipse_path(name: impl Into<String>, cx: f32, cy: f32, rx: f32, ry: f32) -> VectorPath {
    const N: usize = 32;
    let mut anchors = Vec::with_capacity(N);
    for i in 0..N {
        let t = (i as f32 / N as f32) * std::f32::consts::TAU;
        anchors.push(PathPoint {
            x: cx + rx * t.cos(),
            y: cy + ry * t.sin(),
        });
    }
    VectorPath::polyline(name, anchors, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stroke_line_paints_pixels() {
        let path = VectorPath::polyline(
            "line",
            vec![PathPoint { x: 2.0, y: 2.0 }, PathPoint { x: 20.0, y: 2.0 }],
            false,
        );
        let rgba = stroke_path_rgba8(32, 8, &path, [0, 0, 0, 255], 1.0).expect("stroke");
        let painted = rgba.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(painted >= 10, "painted={painted}");
    }

    #[test]
    fn fill_rect_covers_interior() {
        let path = rect_path("r", 2.0, 2.0, 6.0, 6.0);
        let rgba =
            rasterize_shape_rgba8(12, 12, &path, Some([255, 0, 0, 255]), None, 0.0).expect("fill");
        let o = (5 * 12 + 5) * 4;
        assert_eq!(rgba[o + 3], 255);
    }
}
