//! 2D camera for the document viewport (pan / zoom).

/// Axis-aligned rectangle in screen or world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.width && py < self.y + self.height
    }
}

/// Orthographic 2D camera: world origin at document top-left; Y down (screen-like).
///
/// Screen position of a world point:
/// `screen = (world - pan) * zoom + viewport_center`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2D {
    /// World-space point shown at the viewport center.
    pub pan_x: f32,
    pub pan_y: f32,
    /// Scale factor (1.0 = 100%).
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Camera2D {
    pub const MIN_ZOOM: f32 = 0.05;
    pub const MAX_ZOOM: f32 = 32.0;

    /// The stops Zoom In and Zoom Out walk, `MIN_ZOOM` to `MAX_ZOOM`.
    ///
    /// A ladder rather than a fixed multiplier, for two reasons a user can
    /// feel: zooming in and back out again returns to the number you started
    /// from, and the ladder passes through 100% exactly rather than 99.6%.
    /// The stops are the familiar ones — halves and thirds below 100%, whole
    /// multiples above it.
    pub const ZOOM_STOPS: [f32; 17] = [
        0.05,
        1.0 / 12.0,
        0.125,
        1.0 / 6.0,
        0.25,
        1.0 / 3.0,
        0.5,
        2.0 / 3.0,
        1.0,
        1.5,
        2.0,
        3.0,
        4.0,
        6.0,
        8.0,
        16.0,
        32.0,
    ];

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// The next stop above (or below) the current zoom.
    ///
    /// Current zoom is rarely *on* the ladder — a fit, a pinch or a wheel
    /// zoom lands anywhere — so this takes the first stop strictly past it in
    /// the requested direction rather than snapping to the nearest one first.
    /// Snapping first would make the press after a fit do nothing visible
    /// about half the time.
    #[must_use]
    pub fn stepped_zoom(self, zoom_in: bool) -> f32 {
        // Wide enough to skip a stop the camera is already sitting on after
        // float round-tripping, narrow enough not to skip a real neighbour:
        // the closest pair on the ladder differ by about 0.03.
        const EPSILON: f32 = 1e-3;
        let mut stops = Self::ZOOM_STOPS.into_iter();
        if zoom_in {
            stops
                .find(|&stop| stop > self.zoom + EPSILON)
                .unwrap_or(Self::MAX_ZOOM)
        } else {
            stops
                .rfind(|&stop| stop < self.zoom - EPSILON)
                .unwrap_or(Self::MIN_ZOOM)
        }
    }

    /// Pan by screen-space delta (drag gesture). Positive dx moves content right.
    pub fn pan_by_screen(&mut self, dx: f32, dy: f32) {
        if self.zoom <= f32::EPSILON {
            return;
        }
        self.pan_x -= dx / self.zoom;
        self.pan_y -= dy / self.zoom;
    }

    /// Zoom by multiplicative factor, keeping `anchor_screen` fixed in world space.
    pub fn zoom_at(
        &mut self,
        factor: f32,
        anchor_sx: f32,
        anchor_sy: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        if factor <= 0.0 || !factor.is_finite() {
            return;
        }
        let old = self.zoom;
        let new = (old * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        if (new - old).abs() < f32::EPSILON {
            return;
        }
        let (wx, wy) = self.screen_to_world(anchor_sx, anchor_sy, viewport_w, viewport_h);
        self.zoom = new;
        // Re-solve pan so (wx,wy) stays under the same screen pixel.
        let cx = viewport_w * 0.5;
        let cy = viewport_h * 0.5;
        self.pan_x = wx - (anchor_sx - cx) / self.zoom;
        self.pan_y = wy - (anchor_sy - cy) / self.zoom;
    }

    pub fn screen_to_world(
        &self,
        sx: f32,
        sy: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32) {
        let cx = viewport_w * 0.5;
        let cy = viewport_h * 0.5;
        let wx = self.pan_x + (sx - cx) / self.zoom;
        let wy = self.pan_y + (sy - cy) / self.zoom;
        (wx, wy)
    }

    pub fn world_to_screen(
        &self,
        wx: f32,
        wy: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (f32, f32) {
        let cx = viewport_w * 0.5;
        let cy = viewport_h * 0.5;
        let sx = (wx - self.pan_x) * self.zoom + cx;
        let sy = (wy - self.pan_y) * self.zoom + cy;
        (sx, sy)
    }

    /// Axis-aligned document rectangle in screen pixels.
    pub fn document_screen_rect(
        &self,
        doc_w: f32,
        doc_h: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Rect {
        let (x0, y0) = self.world_to_screen(0.0, 0.0, viewport_w, viewport_h);
        let (x1, y1) = self.world_to_screen(doc_w, doc_h, viewport_w, viewport_h);
        Rect::new(x0, y0, (x1 - x0).abs().max(1.0), (y1 - y0).abs().max(1.0))
    }

    /// Fit the full document in the viewport with a relative margin (0.05 = 5%).
    pub fn zoom_to_fit(
        &mut self,
        doc_w: f32,
        doc_h: f32,
        viewport_w: f32,
        viewport_h: f32,
        margin: f32,
    ) {
        if doc_w <= 0.0 || doc_h <= 0.0 || viewport_w <= 0.0 || viewport_h <= 0.0 {
            return;
        }
        let m = (1.0 - margin.clamp(0.0, 0.4)).max(0.2);
        let zx = (viewport_w * m) / doc_w;
        let zy = (viewport_h * m) / doc_h;
        self.set_zoom(zx.min(zy));
        self.pan_x = doc_w * 0.5;
        self.pan_y = doc_h * 0.5;
    }
}

/// Rolling FPS estimate from frame intervals.
#[derive(Debug, Clone)]
pub struct FpsTracker {
    samples: [f32; 32],
    index: usize,
    filled: usize,
    last_instant_ms: Option<f64>,
    fps: f32,
}

impl Default for FpsTracker {
    fn default() -> Self {
        Self {
            samples: [0.0; 32],
            index: 0,
            filled: 0,
            last_instant_ms: None,
            fps: 0.0,
        }
    }
}

impl FpsTracker {
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Record a frame at `now_ms` (monotonic milliseconds). Returns updated FPS.
    pub fn tick(&mut self, now_ms: f64) -> f32 {
        if let Some(prev) = self.last_instant_ms {
            let dt = (now_ms - prev) as f32;
            if dt > 0.0 && dt < 1000.0 {
                self.samples[self.index] = 1000.0 / dt;
                self.index = (self.index + 1) % self.samples.len();
                self.filled = (self.filled + 1).min(self.samples.len());
                let n = self.filled.max(1);
                let sum: f32 = self.samples[..n].iter().sum();
                self.fps = sum / n as f32;
            }
        }
        self.last_instant_ms = Some(now_ms);
        self.fps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ladder that is unsorted or reaches past the clamp makes the step
    /// commands skip stops or stall against `set_zoom`.
    #[test]
    fn the_zoom_ladder_is_sorted_and_within_the_clamp() {
        let stops = Camera2D::ZOOM_STOPS;
        assert!(stops.windows(2).all(|w| w[0] < w[1]), "{stops:?}");
        assert_eq!(stops[0], Camera2D::MIN_ZOOM);
        assert_eq!(stops[stops.len() - 1], Camera2D::MAX_ZOOM);
        assert!(
            stops.contains(&1.0),
            "the ladder has to pass through 100% exactly"
        );
    }

    /// Zoom in then out and you are back where you started.
    ///
    /// The reason for a ladder rather than a multiplier: a user who
    /// overshoots by one press undoes it with one press.
    #[test]
    fn stepping_in_then_out_returns_to_the_same_stop() {
        for &start in &Camera2D::ZOOM_STOPS {
            let mut c = Camera2D {
                zoom: start,
                ..Camera2D::default()
            };
            if start == Camera2D::MAX_ZOOM {
                continue;
            }
            c.set_zoom(c.stepped_zoom(true));
            c.set_zoom(c.stepped_zoom(false));
            assert!(
                (c.zoom - start).abs() < 1e-4,
                "from {start} went to {}",
                c.zoom
            );
        }
    }

    /// A zoom that is not on the ladder still moves on the first press.
    ///
    /// Zoom-to-fit and the wheel land anywhere, and snapping to the nearest
    /// stop first would make about half of those first presses do nothing.
    #[test]
    fn a_step_from_an_off_ladder_zoom_always_moves() {
        for &zoom in &[0.37_f32, 0.99, 1.01, 5.2, 0.051] {
            let c = Camera2D {
                zoom,
                ..Camera2D::default()
            };
            assert!(c.stepped_zoom(true) > zoom, "in from {zoom}");
            assert!(c.stepped_zoom(false) < zoom, "out from {zoom}");
        }
    }

    #[test]
    fn stepping_stops_at_the_limits() {
        let top = Camera2D {
            zoom: Camera2D::MAX_ZOOM,
            ..Camera2D::default()
        };
        assert_eq!(top.stepped_zoom(true), Camera2D::MAX_ZOOM);
        let bottom = Camera2D {
            zoom: Camera2D::MIN_ZOOM,
            ..Camera2D::default()
        };
        assert_eq!(bottom.stepped_zoom(false), Camera2D::MIN_ZOOM);
    }

    /// A centre-anchored step leaves the middle of the view where it was.
    #[test]
    fn a_zoom_step_does_not_move_what_is_in_the_middle() {
        let mut c = Camera2D {
            pan_x: 120.0,
            pan_y: 80.0,
            zoom: 1.0,
        };
        let (vw, vh) = (800.0, 600.0);
        let before = c.world_to_screen(c.pan_x, c.pan_y, vw, vh);
        c.set_zoom(c.stepped_zoom(true));
        let after = c.world_to_screen(c.pan_x, c.pan_y, vw, vh);
        assert!((before.0 - after.0).abs() < 1e-3);
        assert!((before.1 - after.1).abs() < 1e-3);
    }

    #[test]
    fn zoom_clamped() {
        let mut c = Camera2D::default();
        c.set_zoom(100.0);
        assert!((c.zoom - Camera2D::MAX_ZOOM).abs() < f32::EPSILON);
        c.set_zoom(0.001);
        assert!((c.zoom - Camera2D::MIN_ZOOM).abs() < f32::EPSILON);
    }

    #[test]
    fn pan_moves_world_under_cursor() {
        let mut c = Camera2D {
            pan_x: 100.0,
            pan_y: 50.0,
            zoom: 2.0,
        };
        c.pan_by_screen(20.0, 10.0);
        assert!((c.pan_x - 90.0).abs() < 1e-4);
        assert!((c.pan_y - 45.0).abs() < 1e-4);
    }

    #[test]
    fn zoom_at_keeps_anchor() {
        let mut c = Camera2D {
            pan_x: 960.0,
            pan_y: 540.0,
            zoom: 1.0,
        };
        let vw = 800.0;
        let vh = 600.0;
        let ax = 400.0;
        let ay = 300.0;
        let (wx0, wy0) = c.screen_to_world(ax, ay, vw, vh);
        c.zoom_at(2.0, ax, ay, vw, vh);
        let (wx1, wy1) = c.screen_to_world(ax, ay, vw, vh);
        assert!((wx0 - wx1).abs() < 1e-3);
        assert!((wy0 - wy1).abs() < 1e-3);
        assert!((c.zoom - 2.0).abs() < 1e-4);
    }

    #[test]
    fn zoom_to_fit_centers_document() {
        let mut c = Camera2D::default();
        c.zoom_to_fit(1920.0, 1080.0, 960.0, 540.0, 0.1);
        assert!((c.pan_x - 960.0).abs() < 1e-3);
        assert!((c.pan_y - 540.0).abs() < 1e-3);
        assert!(c.zoom > 0.0 && c.zoom <= 1.0);
        let r = c.document_screen_rect(1920.0, 1080.0, 960.0, 540.0);
        assert!(r.width <= 960.0 + 1.0);
        assert!(r.height <= 540.0 + 1.0);
    }

    #[test]
    fn fps_tracker_averages() {
        let mut t = FpsTracker::default();
        // 16.67ms ≈ 60 FPS
        for i in 0..10 {
            t.tick(f64::from(i) * 16.67);
        }
        assert!(t.fps() > 50.0 && t.fps() < 70.0, "fps={}", t.fps());
    }
}
