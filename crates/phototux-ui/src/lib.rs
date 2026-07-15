//! QML-facing session via qtbridge (ADR-003). Package name `phototux_ui` → `import phototux_ui`.

use std::path::PathBuf;

use phototux_canvas::PaintWorker;
use phototux_engine::{
    BlendMode, DocumentSize, EngineCommand, EngineEvent, LayerId, SessionState, SizePreset,
    tool_id, undo_actions,
};
use qtbridge::qobject;

/// Absolute path to Phosphor `regular/` SVGs (dev tree layout).
fn resolve_icon_root() -> String {
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
    brush_hardness: f32,
    brush_r: f32,
    brush_g: f32,
    brush_b: f32,
    fps: f32,
    composite_ms: f32,
    stroke_latency_ms: f32,
    status_text: String,
    active_tool: String,
    has_document: bool,
    layer_count: i32,
    active_layer_index: i32,
    can_undo: bool,
    can_redo: bool,
    layer_names: String,
    layer_visibility: String,
    graph_revision: i32,
    active_opacity: f32,
    icon_root: String,
    engine: SessionState,
    worker: PaintWorker,
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
            brush_hardness: engine.brush_hardness,
            brush_r: engine.brush_color[0],
            brush_g: engine.brush_color[1],
            brush_b: engine.brush_color[2],
            fps: engine.fps,
            composite_ms: 0.0,
            stroke_latency_ms: 0.0,
            status_text: engine.status_summary(),
            active_tool: engine.active_tool.clone(),
            has_document: engine.has_document,
            layer_count: 0,
            active_layer_index: -1,
            can_undo: false,
            can_redo: false,
            layer_names: String::new(),
            layer_visibility: String::new(),
            graph_revision: 0,
            active_opacity: 1.0,
            icon_root,
            engine,
            worker: PaintWorker::start(),
        }
    }

    fn sync_from_engine(&mut self) {
        self.doc_width = self.engine.size.width as i32;
        self.doc_height = self.engine.size.height as i32;
        self.zoom = self.engine.camera.zoom;
        self.pan_x = self.engine.camera.pan_x;
        self.pan_y = self.engine.camera.pan_y;
        self.brush_size = self.engine.brush_size;
        self.brush_hardness = self.engine.brush_hardness;
        self.brush_r = self.engine.brush_color[0];
        self.brush_g = self.engine.brush_color[1];
        self.brush_b = self.engine.brush_color[2];
        self.fps = self.engine.fps;
        self.composite_ms = self.engine.composite_ms;
        self.stroke_latency_ms = self.engine.stroke_latency_ms;
        self.active_tool = self.engine.active_tool.clone();
        self.has_document = self.engine.has_document;
        self.layer_count = self.engine.layer_count();
        self.active_layer_index = self.engine.active_layer_index();
        self.can_undo = self.engine.can_undo() || phototux_canvas::can_undo_stroke();
        self.can_redo = self.engine.can_redo() || phototux_canvas::can_redo_stroke();
        self.layer_names = self.engine.layer_names_joined();
        self.layer_visibility = self.engine.layer_visibility_joined();
        self.graph_revision = self.engine.graph_revision() as i32;
        self.active_opacity = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .map(|l| l.opacity)
            .unwrap_or(1.0);
        self.status_text = self.engine.status_summary();
    }

    fn emit_camera_fields(&mut self) {
        self.zoom_changed();
        self.pan_x_changed();
        self.pan_y_changed();
        self.status_text_changed();
    }

    fn emit_layer_fields(&mut self) {
        self.layer_count_changed();
        self.active_layer_index_changed();
        self.can_undo_changed();
        self.can_redo_changed();
        self.layer_names_changed();
        self.layer_visibility_changed();
        self.graph_revision_changed();
        self.active_opacity_changed();
        self.composite_ms_changed();
        self.status_text_changed();
    }

    fn emit_doc_fields(&mut self) {
        self.doc_width_changed();
        self.doc_height_changed();
        self.emit_camera_fields();
        self.brush_size_changed();
        self.active_tool_changed();
        self.has_document_changed();
        self.emit_layer_fields();
    }

    fn recomposite(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        match phototux_canvas::sync_and_composite(graph.layers()) {
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
            }
            Err(e) => {
                eprintln!("[phototux] composite: {e}");
            }
        }
    }

    fn open_gpu_document(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        match phototux_canvas::open_document(graph.size, graph.layers()) {
            Ok(ms) => self.engine.set_composite_ms(ms),
            Err(e) => eprintln!("[phototux] open_document GPU: {e}"),
        }
    }

    fn active_id(&self) -> Option<LayerId> {
        self.engine.graph.as_ref().and_then(|g| g.active_id())
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
    qproperty!(
        "brushHardness",
        Member = brush_hardness,
        Notify = brush_hardness_changed
    );
    qproperty!("brushR", Member = brush_r, Notify = brush_color_changed);
    qproperty!("brushG", Member = brush_g, Notify = brush_color_changed);
    qproperty!("brushB", Member = brush_b, Notify = brush_color_changed);
    qproperty!("fps", Member = fps, Notify = fps_changed);
    qproperty!(
        "compositeMs",
        Member = composite_ms,
        Notify = composite_ms_changed
    );
    qproperty!(
        "strokeLatencyMs",
        Member = stroke_latency_ms,
        Notify = stroke_latency_ms_changed
    );
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
    qproperty!(
        "layerCount",
        Member = layer_count,
        Notify = layer_count_changed
    );
    qproperty!(
        "activeLayerIndex",
        Member = active_layer_index,
        Notify = active_layer_index_changed
    );
    qproperty!("canUndo", Member = can_undo, Notify = can_undo_changed);
    qproperty!("canRedo", Member = can_redo, Notify = can_redo_changed);
    qproperty!(
        "layerNames",
        Member = layer_names,
        Notify = layer_names_changed
    );
    qproperty!(
        "layerVisibility",
        Member = layer_visibility,
        Notify = layer_visibility_changed
    );
    qproperty!(
        "graphRevision",
        Member = graph_revision,
        Notify = graph_revision_changed
    );
    qproperty!(
        "activeOpacity",
        Member = active_opacity,
        Notify = active_opacity_changed
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
    fn brush_hardness_changed(&mut self);
    #[qsignal]
    fn brush_color_changed(&mut self);
    #[qsignal]
    fn fps_changed(&mut self);
    #[qsignal]
    fn composite_ms_changed(&mut self);
    #[qsignal]
    fn stroke_latency_ms_changed(&mut self);
    #[qsignal]
    fn status_text_changed(&mut self);
    #[qsignal]
    fn active_tool_changed(&mut self);
    #[qsignal]
    fn has_document_changed(&mut self);
    #[qsignal]
    fn layer_count_changed(&mut self);
    #[qsignal]
    fn active_layer_index_changed(&mut self);
    #[qsignal]
    fn can_undo_changed(&mut self);
    #[qsignal]
    fn can_redo_changed(&mut self);
    #[qsignal]
    fn layer_names_changed(&mut self);
    #[qsignal]
    fn layer_visibility_changed(&mut self);
    #[qsignal]
    fn graph_revision_changed(&mut self);
    #[qsignal]
    fn active_opacity_changed(&mut self);
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
        self.engine.sync_brush_from_tool();
        self.worker.send(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_size_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn set_brush_hardness(&mut self, value: f32) {
        self.engine.set_brush_hardness(value);
        self.engine.sync_brush_from_tool();
        self.worker.send(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_hardness_changed();
    }

    #[qslot]
    fn set_brush_color(&mut self, r: f32, g: f32, b: f32) {
        self.engine.set_brush_color(r, g, b, 1.0);
        self.engine.sync_brush_from_tool();
        self.worker.send(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_color_changed();
    }

    #[qslot]
    fn set_active_tool(&mut self, tool: String) {
        let id = match tool.as_str() {
            tool_id::BRUSH | tool_id::ERASER | tool_id::PAN | tool_id::ZOOM => tool,
            _ => tool_id::BRUSH.to_owned(),
        };
        self.engine.set_active_tool(&id);
        self.engine.sync_brush_from_tool();
        self.worker.send(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.active_tool_changed();
        self.status_text_changed();
    }

    /// Poll paint worker events (call from FrameAnimation).
    #[qslot]
    fn poll_engine(&mut self) {
        let events = self.worker.poll_events();
        let mut dirty = false;
        for ev in events {
            match ev {
                EngineEvent::CompositeDone { ms } => {
                    self.engine.set_composite_ms(ms);
                    dirty = true;
                }
                EngineEvent::StrokeLatency { ms } => {
                    self.engine.set_stroke_latency_ms(ms);
                    dirty = true;
                }
                EngineEvent::StrokeEnded => {
                    self.graph_revision = self.graph_revision.wrapping_add(1);
                    self.graph_revision_changed();
                    dirty = true;
                }
                EngineEvent::Error(e) => {
                    eprintln!("[phototux] paint worker: {e}");
                }
            }
        }
        if dirty {
            self.sync_from_engine();
            self.composite_ms_changed();
            self.stroke_latency_ms_changed();
            self.can_undo_changed();
            self.can_redo_changed();
            self.status_text_changed();
        }
    }

    fn now_ms() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }

    /// Begin paint stroke at canvas-local coordinates (pixels).
    #[qslot]
    fn stroke_begin(&mut self, sx: f32, sy: f32, pressure: f32) {
        if !self.engine.has_document {
            return;
        }
        let tool = self.engine.active_tool.as_str();
        if tool != tool_id::BRUSH && tool != tool_id::ERASER {
            return;
        }
        self.engine.sync_brush_from_tool();
        self.worker.send(EngineCommand::SetBrush(self.engine.brush));
        let Some(layer) = self.active_id() else {
            return;
        };
        let (x, y) = self.engine.screen_to_document(sx, sy);
        self.worker.send(EngineCommand::BeginStroke {
            layer,
            x,
            y,
            pressure: pressure.clamp(0.05, 1.0),
            t_ms: Self::now_ms(),
        });
    }

    #[qslot]
    fn stroke_move(&mut self, sx: f32, sy: f32, pressure: f32) {
        if !self.engine.has_document {
            return;
        }
        let (x, y) = self.engine.screen_to_document(sx, sy);
        self.worker.send(EngineCommand::StrokePoint {
            x,
            y,
            pressure: pressure.clamp(0.05, 1.0),
            t_ms: Self::now_ms(),
        });
    }

    #[qslot]
    fn stroke_end(&mut self) {
        self.worker.send(EngineCommand::EndStroke);
    }

    #[qslot]
    fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.engine.set_viewport(width, height);
    }

    #[qslot]
    fn pan_by(&mut self, dx: f32, dy: f32) {
        self.engine.pan_by(dx, dy);
        self.sync_from_engine();
        self.emit_camera_fields();
    }

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

    #[qslot]
    fn apply_size_preset(&mut self, label: String) {
        if let Some(preset) = SizePreset::from_label(&label) {
            self.engine.apply_preset(preset);
            self.open_gpu_document();
            self.sync_from_engine();
            self.emit_doc_fields();
        }
    }

    #[qslot]
    fn apply_document_size(&mut self, width: i32, height: i32) {
        let w = width.max(1) as u32;
        let h = height.max(1) as u32;
        self.engine.apply_size(DocumentSize::new(w, h));
        self.open_gpu_document();
        self.sync_from_engine();
        self.emit_doc_fields();
    }

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

    // —— Layers / undo (Phase 3) ——

    #[qslot]
    fn add_layer(&mut self) {
        {
            let SessionState { graph, undo, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            undo_actions::add_layer(graph, undo, None);
        }
        self.recomposite();
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn delete_active_layer(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let ok = {
            let SessionState { graph, undo, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::delete_layer(g, undo, id))
        };
        if ok {
            self.recomposite();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn set_active_layer(&mut self, index: i32) {
        let ok = self
            .engine
            .graph
            .as_mut()
            .is_some_and(|g| g.set_active_index(index.max(0) as usize));
        if ok {
            self.sync_from_engine();
            self.active_layer_index_changed();
            self.active_opacity_changed();
            self.status_text_changed();
        }
    }

    #[qslot]
    fn set_layer_visible(&mut self, index: i32, visible: bool) {
        let id = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.layers().get(index.max(0) as usize).map(|l| l.id));
        let Some(id) = id else {
            return;
        };
        let ok = {
            let SessionState { graph, undo, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::set_visibility(g, undo, id, visible))
        };
        if ok {
            self.recomposite();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn toggle_layer_visible(&mut self, index: i32) {
        let vis = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.layers().get(index.max(0) as usize).map(|l| l.visible))
            .unwrap_or(true);
        self.set_layer_visible(index, !vis);
    }

    #[qslot]
    fn set_active_opacity(&mut self, opacity: f32) {
        let Some(id) = self.active_id() else {
            return;
        };
        let ok = {
            let SessionState { graph, undo, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::set_opacity(g, undo, id, opacity))
        };
        if ok {
            self.recomposite();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn set_active_blend(&mut self, blend: String) {
        let Some(mode) = BlendMode::from_str_label(&blend) else {
            return;
        };
        let Some(id) = self.active_id() else {
            return;
        };
        let ok = {
            let SessionState { graph, undo, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::set_blend(g, undo, id, mode))
        };
        if ok {
            self.recomposite();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn move_active_layer(&mut self, to_index: i32) {
        let Some(id) = self.active_id() else {
            return;
        };
        let ok = {
            let SessionState { graph, undo, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::move_layer(g, undo, id, to_index.max(0) as usize))
        };
        if ok {
            self.recomposite();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn undo(&mut self) {
        if phototux_canvas::can_undo_stroke() {
            if let Ok(ms) = phototux_canvas::undo_stroke() {
                self.engine.set_composite_ms(ms);
                self.sync_from_engine();
                self.emit_layer_fields();
                self.graph_revision_changed();
            }
            return;
        }
        let ok = {
            let SessionState { graph, undo, .. } = &mut self.engine;
            graph.as_mut().is_some_and(|g| undo.undo(g))
        };
        if ok {
            self.recomposite();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn redo(&mut self) {
        if phototux_canvas::can_redo_stroke() {
            if let Ok(ms) = phototux_canvas::redo_stroke() {
                self.engine.set_composite_ms(ms);
                self.sync_from_engine();
                self.emit_layer_fields();
                self.graph_revision_changed();
            }
            return;
        }
        let ok = {
            let SessionState { graph, undo, .. } = &mut self.engine;
            graph.as_mut().is_some_and(|g| undo.redo(g))
        };
        if ok {
            self.recomposite();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }
}
