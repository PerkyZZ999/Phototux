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

/// Regular N-gon centered at `(cx, cy)` with outer radius `r`.
pub fn polygon_path(name: impl Into<String>, cx: f32, cy: f32, r: f32, sides: u32) -> VectorPath {
    let n = sides.clamp(3, 64) as usize;
    let mut anchors = Vec::with_capacity(n);
    for i in 0..n {
        let t = (i as f32 / n as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        anchors.push(PathPoint {
            x: cx + r * t.cos(),
            y: cy + r * t.sin(),
        });
    }
    VectorPath::polyline(name, anchors, true)
}

/// Star with `points` tips, alternating between `r_outer` and `r_inner`.
///
/// A star is a polygon whose radius alternates, which is why it shares the
/// same anchor loop rather than getting a shape of its own.
#[must_use]
pub fn star_path(
    name: impl Into<String>,
    cx: f32,
    cy: f32,
    r_outer: f32,
    r_inner: f32,
    points: u32,
) -> VectorPath {
    let n = points.clamp(3, 32) as usize;
    let mut anchors = Vec::with_capacity(n * 2);
    for i in 0..n * 2 {
        let t = (i as f32 / (n * 2) as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let r = if i.is_multiple_of(2) {
            r_outer
        } else {
            r_inner
        };
        anchors.push(PathPoint {
            x: cx + r * t.cos(),
            y: cy + r * t.sin(),
        });
    }
    VectorPath::polyline(name, anchors, true)
}

/// Horizontal arrow from `(x0, y)` to `(x1, y)`, drawn as an outline.
///
/// Seven anchors: the shaft's four corners and the head's three, walked so the
/// outline closes without crossing itself.
#[must_use]
pub fn arrow_path(
    name: impl Into<String>,
    x0: f32,
    x1: f32,
    y: f32,
    shaft: f32,
    head: f32,
) -> VectorPath {
    let dir = if x1 >= x0 { 1.0 } else { -1.0 };
    let head_len = (x1 - x0).abs().min(head * 2.0) * 0.5;
    let neck = x1 - dir * head_len;
    let half_shaft = shaft * 0.5;
    let half_head = head * 0.5;
    let p = |x: f32, y: f32| PathPoint { x, y };
    VectorPath::polyline(
        name,
        vec![
            p(x0, y - half_shaft),
            p(neck, y - half_shaft),
            p(neck, y - half_head),
            p(x1, y),
            p(neck, y + half_head),
            p(neck, y + half_shaft),
            p(x0, y + half_shaft),
        ],
        true,
    )
}

/// Rectangle with corners cut at `radius`, approximated by three points each.
///
/// Chamfered rather than curved: the path model carries anchors, not Béziers,
/// so a true fillet would need a curve type the renderer does not yet read —
/// and a visibly wrong curve is worse than an honest bevel.
#[must_use]
pub fn rounded_rect_path(
    name: impl Into<String>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
) -> VectorPath {
    // A radius past half the shorter side would fold the outline inside out.
    let r = radius.clamp(0.0, w.min(h) * 0.5);
    let (x1, y1) = (x + w, y + h);

    // Four quarter-turns, walked clockwise in screen space (y down) from the
    // top-right corner. Each is sampled rather than curved: the path model
    // carries anchors, not Béziers, so a true fillet would need a curve type
    // the renderer does not yet read — and a visibly wrong curve is worse than
    // an honest polyline.
    const STEPS: usize = 4;
    let mut anchors = Vec::with_capacity((STEPS + 1) * 4);
    let mut arc = |cx: f32, cy: f32, from_deg: f32| {
        for i in 0..=STEPS {
            #[expect(
                clippy::cast_precision_loss,
                reason = "STEPS is 4; the ratio is exact in f32"
            )]
            let t = (from_deg + 90.0 * (i as f32 / STEPS as f32)).to_radians();
            anchors.push(PathPoint {
                x: cx + r * t.cos(),
                y: cy + r * t.sin(),
            });
        }
    };
    arc(x1 - r, y + r, -90.0);
    arc(x1 - r, y1 - r, 0.0);
    arc(x + r, y1 - r, 90.0);
    arc(x + r, y + r, 180.0);
    VectorPath::polyline(name, anchors, true)
}

/// Fill a closed path with a linear gradient (coverage from even-odd, color lerped).
#[expect(
    clippy::too_many_arguments,
    reason = "CPU fill helper mirrors rasterize_shape_rgba8 arity; packing would hide hot params"
)]
pub fn fill_gradient_even_odd(
    out: &mut [u8],
    width: u32,
    height: u32,
    path: &VectorPath,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    c0: [u8; 4],
    c1: [u8; 4],
) {
    if path.anchors.len() < 3 || width == 0 || height == 0 {
        return;
    }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-6 {
        fill_even_odd(out, width, height, path, c0);
        return;
    }
    let w = width as i32;
    let h = height as i32;
    let pts = &path.anchors;
    for y in 0..h {
        for x in 0..w {
            if !even_odd_contains(pts, x as f32, y as f32) {
                continue;
            }
            let t = ((x as f32 - x0) * dx + (y as f32 - y0) * dy) / len2;
            write_gradient_pixel(out, width, x as u32, y as u32, t.clamp(0.0, 1.0), c0, c1);
        }
    }
}

fn even_odd_contains(pts: &[PathPoint], x: f32, y: f32) -> bool {
    let n = pts.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = pts[i];
        let pj = pts[j];
        if (pi.y > y) != (pj.y > y)
            && x < (pj.x - pi.x) * (y - pi.y) / (pj.y - pi.y + f32::EPSILON) + pi.x
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn write_gradient_pixel(
    out: &mut [u8],
    width: u32,
    x: u32,
    y: u32,
    t: f32,
    c0: [u8; 4],
    c1: [u8; 4],
) {
    let o = (y as usize * width as usize + x as usize) * 4;
    for c in 0..4 {
        let v = f32::from(c0[c]) * (1.0 - t) + f32::from(c1[c]) * t;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "gradient lerp clamped to byte"
        )]
        {
            out[o + c] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    /// A star's tips must alternate between the two radii, or it is a polygon
    /// with twice as many sides.
    #[test]
    fn a_star_alternates_between_its_two_radii() {
        let star = star_path("s", 0.0, 0.0, 10.0, 4.0, 5);
        assert_eq!(star.anchors.len(), 10, "five points make ten anchors");
        let radius = |p: &PathPoint| (p.x * p.x + p.y * p.y).sqrt();
        for (i, p) in star.anchors.iter().enumerate() {
            let want = if i.is_multiple_of(2) { 10.0 } else { 4.0 };
            assert!(
                (radius(p) - want).abs() < 1e-3,
                "anchor {i} at radius {}",
                radius(p)
            );
        }
        assert!(star.closed);
    }

    /// The arrow's head must reach the point it was asked to end at, and its
    /// shaft must be the narrower of the two — a head no wider than the shaft
    /// is not an arrow.
    #[test]
    fn an_arrow_points_where_it_was_aimed() {
        let arrow = arrow_path("a", 0.0, 100.0, 50.0, 10.0, 30.0);
        assert!(arrow.closed);
        let tip = arrow
            .anchors
            .iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .expect("tip");
        assert!((tip.x - 100.0).abs() < 1e-3, "tip at {}", tip.x);
        assert!((tip.y - 50.0).abs() < 1e-3, "tip off the axis");
        let span = |f: fn(&PathPoint) -> f32| {
            let ys: Vec<f32> = arrow.anchors.iter().map(f).collect();
            ys.iter().cloned().fold(f32::MIN, f32::max)
                - ys.iter().cloned().fold(f32::MAX, f32::min)
        };
        assert!((span(|p| p.y) - 30.0).abs() < 1e-3, "head width wrong");
    }

    /// A rounded rectangle must stay inside the box it was given, and its
    /// corners must actually be cut — otherwise it is a rectangle.
    #[test]
    fn a_rounded_rectangle_stays_in_its_box_and_loses_its_corners() {
        let r = rounded_rect_path("r", 10.0, 20.0, 100.0, 60.0, 12.0);
        assert!(r.closed);
        for p in &r.anchors {
            assert!((10.0..=110.0).contains(&p.x), "x {} escaped", p.x);
            assert!((20.0..=80.0).contains(&p.y), "y {} escaped", p.y);
        }
        // No anchor sits on a square corner of the box.
        for &(cx, cy) in &[(10.0, 20.0), (110.0, 20.0), (110.0, 80.0), (10.0, 80.0)] {
            assert!(
                !r.anchors
                    .iter()
                    .any(|p| (p.x - cx).abs() < 1e-3 && (p.y - cy).abs() < 1e-3),
                "the corner at ({cx}, {cy}) was not cut"
            );
        }
    }

    /// A radius larger than the box would fold the outline inside out.
    #[test]
    fn an_oversized_corner_radius_is_clamped() {
        let r = rounded_rect_path("r", 0.0, 0.0, 20.0, 10.0, 999.0);
        for p in &r.anchors {
            assert!((0.0..=20.0).contains(&p.x) && (0.0..=10.0).contains(&p.y));
        }
    }

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
