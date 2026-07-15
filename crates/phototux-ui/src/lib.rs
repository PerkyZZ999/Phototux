//! QML-facing session via qtbridge (ADR-003). Package name `phototux_ui` → `import phototux_ui`.

use std::path::PathBuf;

use phototux_engine::{DocumentSize, SessionState, SizePreset, tool_id};
use qtbridge::qobject;

/// Absolute path to Phosphor `regular/` SVGs (dev tree layout).
fn resolve_icon_root() -> String {
    // crates/phototux-ui → repo root
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../assets/icons/phosphor/regular");
    p.canonicalize().unwrap_or(p).to_string_lossy().into_owned()
}

/// Application session singleton for the desktop shell.
pub struct AppSession {
    doc_width: i32,
    doc_height: i32,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    brush_size: f32,
    fps: f32,
    status_text: String,
    active_tool: String,
    has_document: bool,
    /// Absolute path to Phosphor regular SVG directory (file:// friendly).
    icon_root: String,
    engine: SessionState,
}

impl Default for AppSession {
    fn default() -> Self {
        Self::new(resolve_icon_root())
    }
}

impl AppSession {
    pub fn new(icon_root: String) -> Self {
        let engine = SessionState::default();
        Self {
            doc_width: engine.size.width as i32,
            doc_height: engine.size.height as i32,
            zoom: engine.camera.zoom,
            pan_x: engine.camera.pan_x,
            pan_y: engine.camera.pan_y,
            brush_size: engine.brush_size,
            fps: engine.fps,
            status_text: engine.status_summary(),
            active_tool: engine.active_tool.clone(),
            has_document: engine.has_document,
            icon_root,
            engine,
        }
    }

    fn sync_from_engine(&mut self) {
        self.doc_width = self.engine.size.width as i32;
        self.doc_height = self.engine.size.height as i32;
        self.zoom = self.engine.camera.zoom;
        self.pan_x = self.engine.camera.pan_x;
        self.pan_y = self.engine.camera.pan_y;
        self.brush_size = self.engine.brush_size;
        self.fps = self.engine.fps;
        self.active_tool = self.engine.active_tool.clone();
        self.has_document = self.engine.has_document;
        self.status_text = self.engine.status_summary();
    }

    fn emit_camera_fields(&mut self) {
        self.zoom_changed();
        self.pan_x_changed();
        self.pan_y_changed();
        self.status_text_changed();
    }

    fn emit_doc_fields(&mut self) {
        self.doc_width_changed();
        self.doc_height_changed();
        self.emit_camera_fields();
        self.brush_size_changed();
        self.active_tool_changed();
        self.has_document_changed();
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl AppSession {
    qproperty!("docWidth", Member = doc_width, Notify = doc_width_changed);
    qproperty!(
        "docHeight",
        Member = doc_height,
        Notify = doc_height_changed
    );
    qproperty!("zoom", Member = zoom, Notify = zoom_changed);
    qproperty!("panX", Member = pan_x, Notify = pan_x_changed);
    qproperty!("panY", Member = pan_y, Notify = pan_y_changed);
    qproperty!(
        "brushSize",
        Member = brush_size,
        Notify = brush_size_changed
    );
    qproperty!("fps", Member = fps, Notify = fps_changed);
    qproperty!(
        "statusText",
        Member = status_text,
        Notify = status_text_changed
    );
    qproperty!(
        "activeTool",
        Member = active_tool,
        Notify = active_tool_changed
    );
    qproperty!(
        "hasDocument",
        Member = has_document,
        Notify = has_document_changed
    );
    qproperty!("iconRoot", Member = icon_root, Notify = icon_root_changed);

    #[qsignal]
    fn doc_width_changed(&mut self);

    #[qsignal]
    fn doc_height_changed(&mut self);

    #[qsignal]
    fn zoom_changed(&mut self);

    #[qsignal]
    fn pan_x_changed(&mut self);

    #[qsignal]
    fn pan_y_changed(&mut self);

    #[qsignal]
    fn brush_size_changed(&mut self);

    #[qsignal]
    fn fps_changed(&mut self);

    #[qsignal]
    fn status_text_changed(&mut self);

    #[qsignal]
    fn active_tool_changed(&mut self);

    #[qsignal]
    fn has_document_changed(&mut self);

    #[qsignal]
    fn icon_root_changed(&mut self);

    #[qslot]
    fn set_zoom(&mut self, value: f32) {
        self.engine.set_zoom(value);
        self.sync_from_engine();
        self.emit_camera_fields();
    }

    #[qslot]
    fn set_brush_size(&mut self, value: f32) {
        self.engine.set_brush_size(value);
        self.sync_from_engine();
        self.brush_size_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn set_active_tool(&mut self, tool: String) {
        let id = match tool.as_str() {
            tool_id::BRUSH | tool_id::PAN | tool_id::ZOOM => tool,
            _ => tool_id::BRUSH.to_owned(),
        };
        self.engine.set_active_tool(&id);
        self.sync_from_engine();
        self.active_tool_changed();
        self.status_text_changed();
    }

    /// Report canvas viewport size (CSS pixels) for zoom-to-fit / zoom-at.
    #[qslot]
    fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.engine.set_viewport(width, height);
    }

    /// Pan by screen-space delta (pixels). Used by pan tool / middle-drag.
    #[qslot]
    fn pan_by(&mut self, dx: f32, dy: f32) {
        self.engine.pan_by(dx, dy);
        self.sync_from_engine();
        self.emit_camera_fields();
    }

    /// Zoom by multiplicative factor around a screen-space anchor.
    #[qslot]
    fn zoom_at(&mut self, factor: f32, anchor_x: f32, anchor_y: f32) {
        self.engine.zoom_at(factor, anchor_x, anchor_y);
        self.sync_from_engine();
        self.emit_camera_fields();
    }

    #[qslot]
    fn zoom_to_fit(&mut self) {
        self.engine.zoom_to_fit();
        self.sync_from_engine();
        self.emit_camera_fields();
    }

    /// Apply a named size preset: `"720p" | "1080p" | "2K" | "4K"`.
    #[qslot]
    fn apply_size_preset(&mut self, label: String) {
        if let Some(preset) = SizePreset::from_label(&label) {
            self.engine.apply_preset(preset);
            self.sync_from_engine();
            self.emit_doc_fields();
        }
    }

    /// Custom document size in pixels (clamped in engine).
    #[qslot]
    fn apply_document_size(&mut self, width: i32, height: i32) {
        let w = width.max(1) as u32;
        let h = height.max(1) as u32;
        self.engine.apply_size(DocumentSize::new(w, h));
        self.sync_from_engine();
        self.emit_doc_fields();
    }

    /// Update FPS estimate from QML `FrameAnimation` / canvas timer.
    #[qslot]
    fn report_fps(&mut self, fps: f32) {
        self.engine.set_fps(fps);
        self.fps = self.engine.fps;
        self.fps_changed();
    }

    #[qslot]
    fn set_status(&mut self, text: String) {
        self.status_text = text;
        self.status_text_changed();
    }
}
