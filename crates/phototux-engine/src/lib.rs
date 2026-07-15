//! Pure Rust engine: document/canvas state without Qt dependencies (ADR-006).

/// Placeholder document size used until the full graph lands in Phase 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSize {
    pub width: u32,
    pub height: u32,
}

impl CanvasSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn pixel_count(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

impl Default for CanvasSize {
    fn default() -> Self {
        Self::new(3840, 2160)
    }
}

/// Lightweight session state shared conceptually with the UI layer.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub canvas: CanvasSize,
    pub zoom: f32,
    pub brush_size: f32,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            canvas: CanvasSize::default(),
            zoom: 1.0,
            brush_size: 12.0,
        }
    }
}

impl SessionState {
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.05, 32.0);
    }

    pub fn set_brush_size(&mut self, size: f32) {
        self.brush_size = size.clamp(1.0, 500.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_pixel_count_4k() {
        let c = CanvasSize::new(3840, 2160);
        assert_eq!(c.pixel_count(), 3840 * 2160);
    }

    #[test]
    fn zoom_clamped() {
        let mut s = SessionState::default();
        s.set_zoom(100.0);
        assert_eq!(s.zoom, 32.0);
        s.set_zoom(0.01);
        assert_eq!(s.zoom, 0.05);
    }

    #[test]
    fn brush_size_clamped() {
        let mut s = SessionState::default();
        s.set_brush_size(0.0);
        assert_eq!(s.brush_size, 1.0);
        s.set_brush_size(9999.0);
        assert_eq!(s.brush_size, 500.0);
    }
}
