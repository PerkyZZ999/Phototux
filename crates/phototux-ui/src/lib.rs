//! UI-facing QObjects via qtbridge (ADR-003). Package name `phototux_ui` → QML `import phototux_ui`.

use phototux_engine::SessionState;
use qtbridge::qobject;

/// Application session exposed to QML as a singleton.
///
/// Light property traffic only (ADR-007). Heavy work stays out of slots.
///
/// qtbridge constructs singletons via [`Default`] — seed real defaults here.
pub struct AppSession {
    /// Brush diameter in device-independent pixels (UI binding demo).
    brush_size: f32,
    /// Viewport zoom factor.
    zoom: f32,
    /// Status line for the shell footer.
    status_text: String,
    /// Internal pure-engine mirror (not a Q property).
    engine: SessionState,
}

impl Default for AppSession {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[qobject(Singleton)]
impl AppSession {
    qproperty!("brushSize", Member = brush_size, Notify = brush_size_changed);
    qproperty!("zoom", Member = zoom, Notify = zoom_changed);
    qproperty!("statusText", Member = status_text, Notify = status_text_changed);

    #[qsignal]
    fn brush_size_changed(&mut self);

    #[qsignal]
    fn zoom_changed(&mut self);

    #[qsignal]
    fn status_text_changed(&mut self);

    #[qslot]
    fn set_brush_size(&mut self, value: f32) {
        self.engine.set_brush_size(value);
        self.brush_size = self.engine.brush_size;
        self.status_text = format!("Brush size: {:.0}px", self.brush_size);
        self.brush_size_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn set_zoom(&mut self, value: f32) {
        self.engine.set_zoom(value);
        self.zoom = self.engine.zoom;
        self.status_text = format!("Zoom: {:.0}%", self.zoom * 100.0);
        self.zoom_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn reset_view(&mut self) {
        self.engine.set_zoom(1.0);
        self.zoom = 1.0;
        self.status_text = "View reset".into();
        self.zoom_changed();
        self.status_text_changed();
    }
}

impl AppSession {
    /// Seed defaults after `Default` for first paint.
    pub fn with_defaults() -> Self {
        let engine = SessionState::default();
        Self {
            brush_size: engine.brush_size,
            zoom: engine.zoom,
            status_text: "PhotoTux ready — GPU canvas Phase 2".into(),
            engine,
        }
    }
}
