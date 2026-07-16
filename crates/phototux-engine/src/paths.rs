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
}
