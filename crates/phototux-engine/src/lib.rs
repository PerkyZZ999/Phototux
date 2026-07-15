//! Pure document/session types — no Qt (ADR-006, ADR-011 Phase 1 stub).

/// Pixel dimensions of the open document canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentSize {
    pub width: u32,
    pub height: u32,
}

impl DocumentSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn pixel_count(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Aspect ratio for placeholder framing (width / height).
    pub fn aspect(self) -> f32 {
        if self.height == 0 {
            return 1.0;
        }
        self.width as f32 / self.height as f32
    }
}

/// Named size presets (ADR-013). 1080p is the recommended default highlight in UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizePreset {
    P720,
    P1080,
    P2k,
    P4k,
}

impl SizePreset {
    pub const ALL: [SizePreset; 4] = [
        SizePreset::P720,
        SizePreset::P1080,
        SizePreset::P2k,
        SizePreset::P4k,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::P720 => "720p",
            Self::P1080 => "1080p",
            Self::P2k => "2K",
            Self::P4k => "4K",
        }
    }

    pub fn size(self) -> DocumentSize {
        match self {
            Self::P720 => DocumentSize::new(1280, 720),
            Self::P1080 => DocumentSize::new(1920, 1080),
            Self::P2k => DocumentSize::new(2560, 1440),
            Self::P4k => DocumentSize::new(3840, 2160),
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "720p" => Some(Self::P720),
            "1080p" => Some(Self::P1080),
            "2K" | "2k" => Some(Self::P2k),
            "4K" | "4k" => Some(Self::P4k),
            _ => None,
        }
    }
}

/// Tool ids aligned with `assets/icons/ICON_MAP.md` (Phase 1 subset).
pub mod tool_id {
    pub const BRUSH: &str = "tool.brush";
    pub const PAN: &str = "tool.pan";
    pub const ZOOM: &str = "tool.zoom";
}

/// Lightweight session state until the Phase 3 document graph exists.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub size: DocumentSize,
    pub zoom: f32,
    pub brush_size: f32,
    pub active_tool: String,
    pub has_document: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            size: SizePreset::P1080.size(),
            zoom: 1.0,
            brush_size: 12.0,
            active_tool: tool_id::BRUSH.to_owned(),
            has_document: false,
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

    pub fn apply_size(&mut self, size: DocumentSize) {
        let w = size.width.clamp(1, 32_768);
        let h = size.height.clamp(1, 32_768);
        self.size = DocumentSize::new(w, h);
        self.has_document = true;
        // Zoom-to-fit is applied in UI; reset logical zoom to 1 until camera lands.
        self.zoom = 1.0;
    }

    pub fn apply_preset(&mut self, preset: SizePreset) {
        self.apply_size(preset.size());
    }

    pub fn set_active_tool(&mut self, tool: &str) {
        self.active_tool = tool.to_owned();
    }

    pub fn status_summary(&self) -> String {
        if !self.has_document {
            return "PhotoTux — create or open a document".to_owned();
        }
        format!(
            "{}×{} · zoom {:.0}% · {}",
            self.size.width,
            self.size.height,
            self.zoom * 100.0,
            self.active_tool
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_adr_013() {
        assert_eq!(SizePreset::P720.size(), DocumentSize::new(1280, 720));
        assert_eq!(SizePreset::P1080.size(), DocumentSize::new(1920, 1080));
        assert_eq!(SizePreset::P2k.size(), DocumentSize::new(2560, 1440));
        assert_eq!(SizePreset::P4k.size(), DocumentSize::new(3840, 2160));
    }

    #[test]
    fn zoom_clamped() {
        let mut s = SessionState::default();
        s.set_zoom(100.0);
        assert!((s.zoom - 32.0).abs() < f32::EPSILON);
        s.set_zoom(0.01);
        assert!((s.zoom - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn brush_clamped() {
        let mut s = SessionState::default();
        s.set_brush_size(0.0);
        assert!((s.brush_size - 1.0).abs() < f32::EPSILON);
        s.set_brush_size(9999.0);
        assert!((s.brush_size - 500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_preset_marks_document() {
        let mut s = SessionState::default();
        assert!(!s.has_document);
        s.apply_preset(SizePreset::P4k);
        assert!(s.has_document);
        assert_eq!(s.size, DocumentSize::new(3840, 2160));
    }

    #[test]
    fn preset_from_label() {
        assert_eq!(SizePreset::from_label("1080p"), Some(SizePreset::P1080));
        assert_eq!(SizePreset::from_label("2k"), Some(SizePreset::P2k));
        assert_eq!(SizePreset::from_label("nope"), None);
    }
}
