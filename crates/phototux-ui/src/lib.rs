//! QML-facing session via qtbridge (ADR-003). Package name `phototux_ui` → `import phototux_ui`.

mod file_worker;

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

use file_worker::{FileCommand, FileEvent, FileWorker};
use phototux_canvas::PaintWorker;
use phototux_engine::{
    AdjustmentParams, BlendMode, CropRect, DocumentError, DocumentGraph, DocumentSize,
    EngineCommand, EngineEvent, HistoryKind, LayerId, LayerTransform, MAX_LAYERS, SelectionCombine,
    SelectionRect, SelectionShape, SelectionState, SessionState, SizePreset, TextContent,
    TransformSession, tool_id, undo_actions,
};

#[derive(Clone)]
struct SelectionSnapshot {
    state: SelectionState,
    mask: Vec<u8>,
}

#[derive(Clone)]
struct TransformSnapshot {
    size: DocumentSize,
    layers: Vec<(LayerId, Vec<u8>)>,
    graph: DocumentGraph,
}

const SELECTION_UNDO_LIMIT: usize = 64;
const TRANSFORM_UNDO_LIMIT: usize = 32;
use phototux_io::{RasterFormat, format_report};
use qtbridge::qobject;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();

pub fn mark_process_started() {
    let _ = PROCESS_START.set(Instant::now());
}

fn resolve_icon_root() -> String {
    "qrc:/qt/qml/PhotoTux/App/icons".to_owned()
}

fn local_path(value: &str) -> Result<PathBuf, String> {
    let encoded_path = if let Some(rest) = value.strip_prefix("file://") {
        if rest.starts_with('/') {
            rest
        } else {
            let (host, path) = rest
                .split_once('/')
                .ok_or_else(|| "invalid local file URL".to_owned())?;
            if host != "localhost" {
                return Err("only local file URLs are supported".to_owned());
            }
            path.strip_prefix('/').map_or(path, |stripped| stripped)
        }
    } else {
        value
    };
    let decoded = percent_encoding::percent_decode_str(encoded_path)
        .decode_utf8()
        .map_err(|_| "file path is not valid UTF-8".to_owned())?;
    if decoded.contains('\0') {
        return Err("file path contains a null byte".to_owned());
    }
    let path = if value.starts_with("file://localhost/") {
        PathBuf::from("/").join(decoded.as_ref())
    } else {
        PathBuf::from(decoded.as_ref())
    };
    Ok(path)
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
    layer_kinds: String,
    layer_mask_flags: String,
    layer_clips: String,
    mask_edit_active: bool,
    history_labels: String,
    selection_active: bool,
    selection_x: i32,
    selection_y: i32,
    selection_w: i32,
    selection_h: i32,
    selection_shape: String,
    selection_combine: String,
    selection_preview_active: bool,
    selection_preview_x: i32,
    selection_preview_y: i32,
    selection_preview_w: i32,
    selection_preview_h: i32,
    transform_active: bool,
    transform_constrain: bool,
    transform_tx: f32,
    transform_ty: f32,
    transform_sx: f32,
    transform_sy: f32,
    transform_rot: f32,
    crop_preview_active: bool,
    crop_preview_x: i32,
    crop_preview_y: i32,
    crop_preview_w: i32,
    crop_preview_h: i32,
    compatibility_report: String,
    document_path: String,
    graph_revision: i32,
    active_opacity: f32,
    icon_root: String,
    document_name: String,
    dirty: bool,
    io_busy: bool,
    io_error: String,
    startup_ms: f32,
    engine: SessionState,
    worker: PaintWorker,
    file_worker: FileWorker,
    clipboard_rgba: Option<(u32, u32, Vec<u8>)>,
    selection_undo: Vec<SelectionSnapshot>,
    selection_redo: Vec<SelectionSnapshot>,
    transform_undo: Vec<TransformSnapshot>,
    transform_redo: Vec<TransformSnapshot>,
}

impl Default for AppSession {
    fn default() -> Self {
        let started = Instant::now();
        let mut session = Self::new(resolve_icon_root());
        if let Some(path) = std::env::var_os("PHOTOTUX_DESKTOP_OPEN") {
            let path = PathBuf::from(path);
            session.io_busy = true;
            session.status_text = format!("Opening {}…", path.display());
            if let Err(error) = session.file_worker.send(FileCommand::Open(path)) {
                session.io_busy = false;
                session.io_error = format!("Open failed: {error}");
            }
        }
        eprintln!(
            "[phototux] AppSession ready {:.2} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        session
    }
}

impl AppSession {
    pub fn new(icon_root: String) -> Self {
        let engine = SessionState::default();
        let mut out = Self {
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
            layer_kinds: String::new(),
            layer_mask_flags: String::new(),
            layer_clips: String::new(),
            mask_edit_active: false,
            history_labels: String::new(),
            selection_active: false,
            selection_x: 0,
            selection_y: 0,
            selection_w: 0,
            selection_h: 0,
            selection_shape: "rect".to_owned(),
            selection_combine: SelectionCombine::Replace.as_str().to_owned(),
            selection_preview_active: false,
            selection_preview_x: 0,
            selection_preview_y: 0,
            selection_preview_w: 0,
            selection_preview_h: 0,
            transform_active: false,
            transform_constrain: false,
            transform_tx: 0.0,
            transform_ty: 0.0,
            transform_sx: 1.0,
            transform_sy: 1.0,
            transform_rot: 0.0,
            crop_preview_active: false,
            crop_preview_x: 0,
            crop_preview_y: 0,
            crop_preview_w: 0,
            crop_preview_h: 0,
            compatibility_report: String::new(),
            document_path: String::new(),
            graph_revision: 0,
            active_opacity: 1.0,
            icon_root,
            document_name: "Untitled".to_owned(),
            dirty: false,
            io_busy: false,
            io_error: String::new(),
            startup_ms: 0.0,
            engine,
            worker: PaintWorker::start(),
            file_worker: FileWorker::start(),
            clipboard_rgba: None,
            selection_undo: Vec::new(),
            selection_redo: Vec::new(),
            transform_undo: Vec::new(),
            transform_redo: Vec::new(),
        };
        if let Some(error) = out.worker.start_error() {
            out.status_text = error.to_owned();
            out.io_error = error.to_owned();
        }
        if let Some(error) = out.file_worker.start_error() {
            out.status_text = error.to_owned();
            out.io_error = error.to_owned();
        }
        out
    }

    fn sync_camera_from_engine(&mut self) {
        self.zoom = self.engine.camera.zoom;
        self.pan_x = self.engine.camera.pan_x;
        self.pan_y = self.engine.camera.pan_y;
        self.status_text = self.engine.status_summary();
    }

    fn send_paint(&mut self, command: EngineCommand) {
        if let Err(error) = self.worker.send(command) {
            self.fail_io("Paint", &error);
        }
    }

    fn report_gpu(&mut self, operation: &str, error: &str) {
        self.status_text = format!("{operation} failed: {error}");
        self.status_text_changed();
        eprintln!("[phototux] {operation}: {error}");
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
        self.can_undo = self.engine.can_undo();
        self.can_redo = self.engine.can_redo();
        self.layer_names = self.engine.layer_names_joined();
        self.layer_visibility = self.engine.layer_visibility_joined();
        self.layer_kinds = self.engine.layer_kinds_joined();
        self.layer_mask_flags = self.engine.layer_mask_flags_joined();
        self.layer_clips = self.engine.layer_clips_joined();
        if self.engine.mask_edit_layer.is_some_and(|id| {
            self.engine
                .graph
                .as_ref()
                .and_then(|graph| graph.get(id))
                .is_none_or(|layer| layer.mask.is_none())
        }) {
            self.engine.mask_edit_layer = None;
        }
        self.mask_edit_active = matches!(
            self.engine.paint_target(),
            phototux_engine::PaintTarget::LayerMask
        );
        self.history_labels = self.engine.history_labels_joined();
        self.sync_selection_fields();
        self.sync_transform_fields();
        self.document_path = self.engine.document_path.clone().unwrap_or_default();
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
        self.layer_kinds_changed();
        self.layer_mask_flags_changed();
        self.layer_clips_changed();
        self.mask_edit_active_changed();
        self.history_labels_changed();
        self.emit_selection_fields();
        self.emit_transform_fields();
        self.document_path_changed();
        self.graph_revision_changed();
        self.active_opacity_changed();
        self.composite_ms_changed();
        self.status_text_changed();
    }

    fn sync_selection_fields(&mut self) {
        self.selection_active = self.engine.selection.active;
        self.selection_combine = self.engine.selection.combine.as_str().to_owned();
        self.selection_shape = self.engine.selection.shape.as_str().to_owned();
        if let Some(b) = self.engine.selection.bounds {
            self.selection_x = b.x;
            self.selection_y = b.y;
            self.selection_w = i32::try_from(b.width).unwrap_or(i32::MAX);
            self.selection_h = i32::try_from(b.height).unwrap_or(i32::MAX);
        } else {
            self.selection_x = 0;
            self.selection_y = 0;
            self.selection_w = 0;
            self.selection_h = 0;
        }
    }

    fn emit_selection_fields(&mut self) {
        self.selection_active_changed();
        self.selection_x_changed();
        self.selection_y_changed();
        self.selection_w_changed();
        self.selection_h_changed();
        self.selection_shape_changed();
        self.selection_combine_changed();
        self.selection_preview_active_changed();
        self.selection_preview_x_changed();
        self.selection_preview_y_changed();
        self.selection_preview_w_changed();
        self.selection_preview_h_changed();
    }

    fn clear_selection_stacks(&mut self) {
        self.selection_undo.clear();
        self.selection_redo.clear();
    }

    fn clear_transform_stacks(&mut self) {
        self.transform_undo.clear();
        self.transform_redo.clear();
    }

    fn sync_transform_fields(&mut self) {
        if let Some(session) = &self.engine.transform_session {
            self.transform_active = true;
            self.transform_constrain = session.constrain_aspect;
            self.transform_tx = session.draft.translate_x;
            self.transform_ty = session.draft.translate_y;
            self.transform_sx = session.draft.scale_x;
            self.transform_sy = session.draft.scale_y;
            self.transform_rot = session.draft.rotation_deg;
        } else {
            self.transform_active = false;
            self.transform_constrain = false;
            self.transform_tx = 0.0;
            self.transform_ty = 0.0;
            self.transform_sx = 1.0;
            self.transform_sy = 1.0;
            self.transform_rot = 0.0;
        }
    }

    fn emit_transform_fields(&mut self) {
        self.transform_active_changed();
        self.transform_constrain_changed();
        self.transform_tx_changed();
        self.transform_ty_changed();
        self.transform_sx_changed();
        self.transform_sy_changed();
        self.transform_rot_changed();
        self.crop_preview_active_changed();
        self.crop_preview_x_changed();
        self.crop_preview_y_changed();
        self.crop_preview_w_changed();
        self.crop_preview_h_changed();
    }

    fn push_transform_snapshot(&mut self) {
        let Ok((size, layers)) = phototux_canvas::snapshot_document_layers() else {
            return;
        };
        let Some(graph) = self.engine.graph.clone() else {
            return;
        };
        self.transform_undo.push(TransformSnapshot {
            size,
            layers,
            graph,
        });
        if self.transform_undo.len() > TRANSFORM_UNDO_LIMIT {
            self.transform_undo.remove(0);
        }
        self.transform_redo.clear();
    }

    fn restore_transform_snapshot(&mut self, snap: TransformSnapshot) {
        let layers = snap.graph.layers().to_vec();
        match phototux_canvas::restore_document_layers(snap.size, &snap.layers, &layers) {
            Ok(ms) => {
                self.engine.size = snap.size;
                self.engine.graph = Some(snap.graph);
                self.engine.set_composite_ms(ms);
                self.engine.transform_session = None;
                self.crop_preview_active = false;
            }
            Err(error) => self.report_gpu("transform restore", &error),
        }
    }

    fn push_selection_snapshot(&mut self) {
        let mask = phototux_canvas::selection_snapshot().unwrap_or_default();
        self.selection_undo.push(SelectionSnapshot {
            state: self.engine.selection.clone(),
            mask,
        });
        if self.selection_undo.len() > SELECTION_UNDO_LIMIT {
            self.selection_undo.remove(0);
        }
        self.selection_redo.clear();
    }

    fn restore_selection_snapshot(&mut self, snap: SelectionSnapshot) {
        self.engine.selection = snap.state;
        if let Err(error) = phototux_canvas::selection_restore(&snap.mask) {
            self.report_gpu("selection restore", &error);
        }
    }

    fn emit_doc_fields(&mut self) {
        self.doc_width_changed();
        self.doc_height_changed();
        self.emit_camera_fields();
        self.brush_size_changed();
        self.active_tool_changed();
        self.has_document_changed();
        self.document_name_changed();
        self.dirty_changed();
        self.io_busy_changed();
        self.emit_layer_fields();
    }

    fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            self.dirty_changed();
        }
    }

    fn fail_io(&mut self, operation: &str, message: &str) {
        self.io_busy = false;
        self.io_error = format!("{operation} failed: {message}");
        self.io_busy_changed();
        self.io_error_changed();
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
                self.report_gpu("composite", &e);
            }
        }
    }

    fn open_gpu_document(&mut self) {
        let Some((size, layers)) = self
            .engine
            .graph
            .as_ref()
            .map(|graph| (graph.size, graph.layers().to_vec()))
        else {
            return;
        };
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        self.engine.selection.clear();
        self.engine.transform_session = None;
        self.selection_preview_active = false;
        self.crop_preview_active = false;
        match phototux_canvas::open_document(size, &layers) {
            Ok(ms) => self.engine.set_composite_ms(ms),
            Err(e) => self.report_gpu("open_document GPU", &e),
        }
    }

    fn active_id(&self) -> Option<LayerId> {
        self.engine.graph.as_ref().and_then(|g| g.active_id())
    }
}

#[cfg(test)]
mod tests {
    use super::local_path;
    use std::path::Path;

    #[test]
    fn local_file_url_decodes_percent_escapes() {
        assert_eq!(
            local_path("file:///tmp/Photo%20Tux.png").expect("local file URL"),
            Path::new("/tmp/Photo Tux.png")
        );
    }

    #[test]
    fn local_file_url_rejects_remote_hosts() {
        assert!(local_path("file://example.com/photo.png").is_err());
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
        "layerKinds",
        Member = layer_kinds,
        Notify = layer_kinds_changed
    );
    qproperty!(
        "layerMaskFlags",
        Member = layer_mask_flags,
        Notify = layer_mask_flags_changed
    );
    qproperty!(
        "layerClips",
        Member = layer_clips,
        Notify = layer_clips_changed
    );
    qproperty!(
        "maskEditActive",
        Member = mask_edit_active,
        Notify = mask_edit_active_changed
    );
    qproperty!(
        "historyLabels",
        Member = history_labels,
        Notify = history_labels_changed
    );
    qproperty!(
        "selectionActive",
        Member = selection_active,
        Notify = selection_active_changed
    );
    qproperty!(
        "selectionX",
        Member = selection_x,
        Notify = selection_x_changed
    );
    qproperty!(
        "selectionY",
        Member = selection_y,
        Notify = selection_y_changed
    );
    qproperty!(
        "selectionW",
        Member = selection_w,
        Notify = selection_w_changed
    );
    qproperty!(
        "selectionH",
        Member = selection_h,
        Notify = selection_h_changed
    );
    qproperty!(
        "selectionShape",
        Member = selection_shape,
        Notify = selection_shape_changed
    );
    qproperty!(
        "selectionCombine",
        Member = selection_combine,
        Notify = selection_combine_changed
    );
    qproperty!(
        "selectionPreviewActive",
        Member = selection_preview_active,
        Notify = selection_preview_active_changed
    );
    qproperty!(
        "selectionPreviewX",
        Member = selection_preview_x,
        Notify = selection_preview_x_changed
    );
    qproperty!(
        "selectionPreviewY",
        Member = selection_preview_y,
        Notify = selection_preview_y_changed
    );
    qproperty!(
        "selectionPreviewW",
        Member = selection_preview_w,
        Notify = selection_preview_w_changed
    );
    qproperty!(
        "selectionPreviewH",
        Member = selection_preview_h,
        Notify = selection_preview_h_changed
    );
    qproperty!(
        "transformActive",
        Member = transform_active,
        Notify = transform_active_changed
    );
    qproperty!(
        "transformConstrain",
        Member = transform_constrain,
        Notify = transform_constrain_changed
    );
    qproperty!(
        "transformTx",
        Member = transform_tx,
        Notify = transform_tx_changed
    );
    qproperty!(
        "transformTy",
        Member = transform_ty,
        Notify = transform_ty_changed
    );
    qproperty!(
        "transformSx",
        Member = transform_sx,
        Notify = transform_sx_changed
    );
    qproperty!(
        "transformSy",
        Member = transform_sy,
        Notify = transform_sy_changed
    );
    qproperty!(
        "transformRot",
        Member = transform_rot,
        Notify = transform_rot_changed
    );
    qproperty!(
        "cropPreviewActive",
        Member = crop_preview_active,
        Notify = crop_preview_active_changed
    );
    qproperty!(
        "cropPreviewX",
        Member = crop_preview_x,
        Notify = crop_preview_x_changed
    );
    qproperty!(
        "cropPreviewY",
        Member = crop_preview_y,
        Notify = crop_preview_y_changed
    );
    qproperty!(
        "cropPreviewW",
        Member = crop_preview_w,
        Notify = crop_preview_w_changed
    );
    qproperty!(
        "cropPreviewH",
        Member = crop_preview_h,
        Notify = crop_preview_h_changed
    );
    qproperty!(
        "compatibilityReport",
        Member = compatibility_report,
        Notify = compatibility_report_changed
    );
    qproperty!(
        "documentPath",
        Member = document_path,
        Notify = document_path_changed
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
    qproperty!(
        "documentName",
        Member = document_name,
        Notify = document_name_changed
    );
    qproperty!("dirty", Member = dirty, Notify = dirty_changed);
    qproperty!("ioBusy", Member = io_busy, Notify = io_busy_changed);
    qproperty!("ioError", Member = io_error, Notify = io_error_changed);
    qproperty!(
        "startupMs",
        Member = startup_ms,
        Notify = startup_ms_changed
    );

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
    fn layer_kinds_changed(&mut self);
    #[qsignal]
    fn layer_mask_flags_changed(&mut self);
    #[qsignal]
    fn layer_clips_changed(&mut self);
    #[qsignal]
    fn mask_edit_active_changed(&mut self);
    #[qsignal]
    fn history_labels_changed(&mut self);
    #[qsignal]
    fn selection_active_changed(&mut self);
    #[qsignal]
    fn selection_x_changed(&mut self);
    #[qsignal]
    fn selection_y_changed(&mut self);
    #[qsignal]
    fn selection_w_changed(&mut self);
    #[qsignal]
    fn selection_h_changed(&mut self);
    #[qsignal]
    fn selection_shape_changed(&mut self);
    #[qsignal]
    fn selection_combine_changed(&mut self);
    #[qsignal]
    fn selection_preview_active_changed(&mut self);
    #[qsignal]
    fn selection_preview_x_changed(&mut self);
    #[qsignal]
    fn selection_preview_y_changed(&mut self);
    #[qsignal]
    fn selection_preview_w_changed(&mut self);
    #[qsignal]
    fn selection_preview_h_changed(&mut self);
    #[qsignal]
    fn transform_active_changed(&mut self);
    #[qsignal]
    fn transform_constrain_changed(&mut self);
    #[qsignal]
    fn transform_tx_changed(&mut self);
    #[qsignal]
    fn transform_ty_changed(&mut self);
    #[qsignal]
    fn transform_sx_changed(&mut self);
    #[qsignal]
    fn transform_sy_changed(&mut self);
    #[qsignal]
    fn transform_rot_changed(&mut self);
    #[qsignal]
    fn crop_preview_active_changed(&mut self);
    #[qsignal]
    fn crop_preview_x_changed(&mut self);
    #[qsignal]
    fn crop_preview_y_changed(&mut self);
    #[qsignal]
    fn crop_preview_w_changed(&mut self);
    #[qsignal]
    fn crop_preview_h_changed(&mut self);
    #[qsignal]
    fn compatibility_report_changed(&mut self);
    #[qsignal]
    fn document_path_changed(&mut self);
    #[qsignal]
    fn graph_revision_changed(&mut self);
    #[qsignal]
    fn active_opacity_changed(&mut self);
    #[qsignal]
    fn icon_root_changed(&mut self);
    #[qsignal]
    fn document_name_changed(&mut self);
    #[qsignal]
    fn dirty_changed(&mut self);
    #[qsignal]
    fn io_busy_changed(&mut self);
    #[qsignal]
    fn io_error_changed(&mut self);
    #[qsignal]
    fn startup_ms_changed(&mut self);

    #[qslot]
    fn set_zoom(&mut self, value: f32) {
        self.engine.set_zoom(value);
        self.sync_camera_from_engine();
        self.emit_camera_fields();
    }

    #[qslot]
    fn set_brush_size(&mut self, value: f32) {
        self.engine.set_brush_size(value);
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_size_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn set_brush_hardness(&mut self, value: f32) {
        self.engine.set_brush_hardness(value);
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_hardness_changed();
    }

    #[qslot]
    fn set_brush_color(&mut self, r: f32, g: f32, b: f32) {
        self.engine.set_brush_color(r, g, b, 1.0);
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_color_changed();
    }

    #[qslot]
    fn set_active_tool(&mut self, tool: String) {
        let known = matches!(
            tool.as_str(),
            tool_id::BRUSH
                | tool_id::ERASER
                | tool_id::PAN
                | tool_id::ZOOM
                | tool_id::SELECT_RECT
                | tool_id::SELECT_ELLIPSE
                | tool_id::SELECT_LASSO
                | tool_id::MOVE
                | tool_id::TRANSFORM
                | tool_id::CROP
                | tool_id::FILL
                | tool_id::GRADIENT
                | tool_id::EYEDROPPER
                | tool_id::TEXT
        );
        let id = if known {
            tool
        } else {
            tool_id::BRUSH.to_owned()
        };
        self.engine.set_active_tool(&id);
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
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
                    self.engine.history.push_stroke(if self.mask_edit_active {
                        "Mask stroke"
                    } else {
                        "Brush stroke"
                    });
                    self.graph_revision = self.graph_revision.wrapping_add(1);
                    self.graph_revision_changed();
                    self.mark_dirty();
                    dirty = true;
                }
                EngineEvent::Error(e) => {
                    self.report_gpu("paint worker", &e);
                }
            }
        }
        for event in self.file_worker.poll_events() {
            match event {
                FileEvent::Opened { path, raster } => {
                    let size = DocumentSize::new(raster.width(), raster.height());
                    let layer_name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Image".to_owned());
                    let graph = DocumentGraph::new_flattened(size, layer_name.clone());
                    let Some(target_layer) = graph.active_id() else {
                        self.fail_io("Open", "decoded document has no layer");
                        continue;
                    };
                    self.clear_selection_stacks();
                    self.clear_transform_stacks();
                    match phototux_canvas::open_raster_document(
                        size,
                        graph.layers(),
                        target_layer,
                        raster.pixels(),
                    ) {
                        Ok(ms) => {
                            self.engine.replace_graph(graph);
                            self.engine.set_composite_ms(ms);
                            self.document_name = layer_name;
                            self.dirty = false;
                            self.io_busy = false;
                            self.sync_from_engine();
                            self.emit_doc_fields();
                        }
                        Err(error) => self.fail_io("Open", &error),
                    }
                }
                FileEvent::PtxOpened { path, document } => {
                    let (graph, rasters, masks) = document.into_parts();
                    self.clear_selection_stacks();
                    self.clear_transform_stacks();
                    match phototux_canvas::open_document(graph.size, graph.layers()) {
                        Ok(ms) => {
                            for (id, raster) in rasters {
                                if let Err(error) =
                                    phototux_canvas::write_layer_rgba(LayerId(id), raster.pixels())
                                {
                                    self.fail_io("Open", &error);
                                    continue;
                                }
                            }
                            for (id, mask) in masks {
                                let r8: Vec<u8> =
                                    mask.pixels().chunks_exact(4).map(|rgba| rgba[0]).collect();
                                if let Err(error) = phototux_canvas::write_mask_r8(LayerId(id), &r8)
                                {
                                    self.fail_io("Open", &error);
                                    continue;
                                }
                            }
                            self.engine.replace_graph(graph);
                            self.engine.document_path = Some(path.display().to_string());
                            self.engine.set_composite_ms(ms);
                            self.document_name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "Document.ptx".into());
                            self.dirty = false;
                            self.io_busy = false;
                            self.compatibility_report.clear();
                            self.sync_from_engine();
                            self.emit_doc_fields();
                            self.compatibility_report_changed();
                        }
                        Err(error) => self.fail_io("Open", &error),
                    }
                }
                FileEvent::PsdOpened {
                    path,
                    graph,
                    raster,
                    report,
                } => {
                    let size = graph.size;
                    let Some(target_layer) = graph.active_id() else {
                        self.fail_io("Open", "PSD import has no layer");
                        continue;
                    };
                    self.clear_selection_stacks();
                    self.clear_transform_stacks();
                    match phototux_canvas::open_raster_document(
                        size,
                        graph.layers(),
                        target_layer,
                        raster.pixels(),
                    ) {
                        Ok(ms) => {
                            self.engine.replace_graph(graph);
                            self.engine.set_composite_ms(ms);
                            self.document_name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "Imported.psd".into());
                            self.dirty = true;
                            self.io_busy = false;
                            self.compatibility_report = format_report(&report);
                            self.sync_from_engine();
                            self.emit_doc_fields();
                            self.compatibility_report_changed();
                        }
                        Err(error) => self.fail_io("Open", &error),
                    }
                }
                FileEvent::Saved { path } => {
                    self.io_busy = false;
                    self.dirty = false;
                    self.engine.document_path = Some(path.display().to_string());
                    self.document_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Document.ptx".into());
                    self.status_text = format!("Saved {}", path.display());
                    self.sync_from_engine();
                    self.emit_doc_fields();
                }
                FileEvent::Autosaved => {
                    self.status_text = "Autosave written".to_owned();
                    self.status_text_changed();
                }
                FileEvent::Exported { path } => {
                    self.io_busy = false;
                    self.status_text = format!("Exported {}", path.display());
                    self.io_busy_changed();
                    self.status_text_changed();
                }
                FileEvent::Cancelled { operation } => {
                    self.io_busy = false;
                    self.status_text = format!("{operation} cancelled");
                    self.io_busy_changed();
                    self.status_text_changed();
                }
                FileEvent::Failed { operation, message } => {
                    self.fail_io(operation, &message);
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
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        let Some(layer) = self.active_id() else {
            return;
        };
        let target = self.engine.paint_target();
        let (x, y) = self.engine.screen_to_document(sx, sy);
        self.send_paint(EngineCommand::BeginStroke {
            layer,
            target,
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
        self.send_paint(EngineCommand::StrokePoint {
            x,
            y,
            pressure: pressure.clamp(0.05, 1.0),
            t_ms: Self::now_ms(),
        });
    }

    #[qslot]
    fn stroke_end(&mut self) {
        self.send_paint(EngineCommand::EndStroke);
    }

    #[qslot]
    fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.engine.set_viewport(width, height);
    }

    #[qslot]
    fn pan_by(&mut self, dx: f32, dy: f32) {
        self.engine.pan_by(dx, dy);
        self.sync_camera_from_engine();
        self.emit_camera_fields();
    }

    #[qslot]
    fn zoom_at(&mut self, factor: f32, anchor_x: f32, anchor_y: f32) {
        self.engine.zoom_at(factor, anchor_x, anchor_y);
        self.sync_camera_from_engine();
        self.emit_camera_fields();
    }

    #[qslot]
    fn zoom_to_fit(&mut self) {
        self.engine.zoom_to_fit();
        self.sync_camera_from_engine();
        self.emit_camera_fields();
    }

    #[qslot]
    fn apply_size_preset(&mut self, label: String) {
        if let Some(preset) = SizePreset::from_label(&label) {
            self.engine.apply_preset(preset);
            self.open_gpu_document();
            self.document_name = "Untitled".to_owned();
            self.dirty = false;
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
        self.document_name = "Untitled".to_owned();
        self.dirty = false;
        self.sync_from_engine();
        self.emit_doc_fields();
    }

    #[qslot]
    fn open_raster_file(&mut self, file_url: String) {
        if self.io_busy {
            return;
        }
        let path = match local_path(&file_url) {
            Ok(path) => path,
            Err(error) => {
                self.fail_io("Open", &error);
                return;
            }
        };
        self.io_busy = true;
        self.io_error.clear();
        self.compatibility_report.clear();
        self.status_text = format!("Opening {}…", path.display());
        self.io_busy_changed();
        self.status_text_changed();
        self.compatibility_report_changed();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let command = match ext.as_str() {
            "ptx" => FileCommand::OpenPtx(path),
            "psd" => FileCommand::OpenPsd(path),
            _ => FileCommand::Open(path),
        };
        if let Err(error) = self.file_worker.send(command) {
            self.fail_io("Open", &error);
        }
    }

    #[qslot]
    fn save_document(&mut self, file_url: String) {
        if self.io_busy || !self.engine.has_document {
            return;
        }
        let path = if file_url.is_empty() {
            match self.engine.document_path.clone() {
                Some(existing) => PathBuf::from(existing),
                None => {
                    self.fail_io("Save", "use Save As for untitled documents");
                    return;
                }
            }
        } else {
            match local_path(&file_url) {
                Ok(path) => path,
                Err(error) => {
                    self.fail_io("Save", &error);
                    return;
                }
            }
        };
        let Some(graph) = self.engine.graph.clone() else {
            return;
        };
        self.io_busy = true;
        self.io_error.clear();
        self.status_text = format!("Saving {}…", path.display());
        self.io_busy_changed();
        self.status_text_changed();
        if let Err(error) = self.file_worker.send(FileCommand::SavePtx { path, graph }) {
            self.fail_io("Save", &error);
        }
    }

    #[qslot]
    fn cancel_io(&mut self) {
        self.file_worker.cancel_token().cancel();
        self.status_text = "Cancelling…".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn autosave_now(&mut self) {
        if !self.engine.has_document || self.io_busy {
            return;
        }
        let Some(graph) = self.engine.graph.clone() else {
            return;
        };
        let original = self.engine.document_path.as_ref().map(PathBuf::from);
        let _ = self
            .file_worker
            .send(FileCommand::Autosave { graph, original });
    }

    #[qslot]
    fn export_raster_file(&mut self, file_url: String) {
        if self.io_busy || !self.engine.has_document {
            return;
        }
        let path = match local_path(&file_url) {
            Ok(path) => path,
            Err(error) => {
                self.fail_io("Export", &error);
                return;
            }
        };
        let format = match RasterFormat::from_path(&path) {
            Ok(format) => format,
            Err(error) => {
                self.fail_io("Export", &error.to_string());
                return;
            }
        };
        self.io_busy = true;
        self.io_error.clear();
        self.status_text = format!("Exporting {}…", path.display());
        self.io_busy_changed();
        self.status_text_changed();
        if let Err(error) = self.file_worker.send(FileCommand::Export { path, format }) {
            self.fail_io("Export", &error);
        }
    }

    #[qslot]
    fn close_document(&mut self) {
        if self.io_busy {
            return;
        }
        let viewport_width = self.engine.viewport_w;
        let viewport_height = self.engine.viewport_h;
        phototux_canvas::close_document();
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        self.engine = SessionState::default();
        self.engine.set_viewport(viewport_width, viewport_height);
        self.document_name = "Untitled".to_owned();
        self.dirty = false;
        self.selection_preview_active = false;
        self.crop_preview_active = false;
        self.sync_from_engine();
        self.emit_doc_fields();
    }

    #[qslot]
    fn acknowledge_discard(&mut self) {
        if self.dirty {
            self.dirty = false;
            self.dirty_changed();
        }
    }

    #[qslot]
    fn report_fps(&mut self, fps: f32) {
        self.engine.set_fps(fps);
        self.fps = self.engine.fps;
        self.fps_changed();
    }

    #[qslot]
    fn report_interactive(&mut self) {
        if self.startup_ms > 0.0 {
            return;
        }
        let Some(start) = PROCESS_START.get() else {
            return;
        };
        self.startup_ms = start.elapsed().as_secs_f32() * 1000.0;
        eprintln!(
            "[phototux] first interactive frame {:.2} ms",
            self.startup_ms
        );
        self.startup_ms_changed();
    }

    #[qslot]
    fn set_status(&mut self, text: String) {
        self.status_text = text;
        self.status_text_changed();
    }

    // —— Layers / undo (Phase 3) ——

    #[qslot]
    fn add_layer(&mut self) {
        let result = {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            if graph.layer_count() >= MAX_LAYERS {
                Err(DocumentError::layer_limit(MAX_LAYERS))
            } else {
                undo_actions::add_layer(graph, history, None).map(|_| ())
            }
        };
        match result {
            Ok(()) => {
                self.recomposite();
                self.mark_dirty();
                self.sync_from_engine();
                self.emit_layer_fields();
            }
            Err(error) => self.report_gpu("add layer", &error.to_string()),
        }
    }

    #[qslot]
    fn delete_active_layer(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let ok = {
            let SessionState { graph, history, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::delete_layer(g, history, id))
        };
        if ok {
            self.recomposite();
            self.mark_dirty();
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
            self.layer_mask_flags_changed();
            self.layer_clips_changed();
            self.mask_edit_active_changed();
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
            let SessionState { graph, history, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::set_visibility(g, history, id, visible))
        };
        if ok {
            self.recomposite();
            self.mark_dirty();
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
            let SessionState { graph, history, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::set_opacity(g, history, id, opacity))
        };
        if ok {
            self.recomposite();
            self.mark_dirty();
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
            let SessionState { graph, history, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::set_blend(g, history, id, mode))
        };
        if ok {
            self.recomposite();
            self.mark_dirty();
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
            let SessionState { graph, history, .. } = &mut self.engine;
            graph
                .as_mut()
                .is_some_and(|g| undo_actions::move_layer(g, history, id, to_index.max(0) as usize))
        };
        if ok {
            self.recomposite();
            self.mark_dirty();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn undo(&mut self) {
        let kind = {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            history.undo_next(graph)
        };
        let Some(kind) = kind else {
            return;
        };
        match kind {
            HistoryKind::Stroke => match phototux_canvas::undo_stroke() {
                Ok(ms) => self.engine.set_composite_ms(ms),
                Err(error) => self.report_gpu("stroke undo", &error),
            },
            HistoryKind::Graph => {
                self.recomposite();
            }
            HistoryKind::Selection => {
                let current = SelectionSnapshot {
                    state: self.engine.selection.clone(),
                    mask: phototux_canvas::selection_snapshot().unwrap_or_default(),
                };
                if let Some(prev) = self.selection_undo.pop() {
                    self.selection_redo.push(current);
                    self.restore_selection_snapshot(prev);
                } else {
                    self.engine.selection.clear();
                    let _ = phototux_canvas::selection_clear();
                }
            }
            HistoryKind::Transform => {
                let Ok((size, layers)) = phototux_canvas::snapshot_document_layers() else {
                    return;
                };
                let Some(graph) = self.engine.graph.clone() else {
                    return;
                };
                let current = TransformSnapshot {
                    size,
                    layers,
                    graph,
                };
                if let Some(prev) = self.transform_undo.pop() {
                    self.transform_redo.push(current);
                    self.restore_transform_snapshot(prev);
                }
            }
        }
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.emit_doc_fields();
    }

    #[qslot]
    fn redo(&mut self) {
        let kind = {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            history.redo_next(graph)
        };
        let Some(kind) = kind else {
            return;
        };
        match kind {
            HistoryKind::Stroke => match phototux_canvas::redo_stroke() {
                Ok(ms) => self.engine.set_composite_ms(ms),
                Err(error) => self.report_gpu("stroke redo", &error),
            },
            HistoryKind::Graph => {
                self.recomposite();
            }
            HistoryKind::Selection => {
                let current = SelectionSnapshot {
                    state: self.engine.selection.clone(),
                    mask: phototux_canvas::selection_snapshot().unwrap_or_default(),
                };
                if let Some(next) = self.selection_redo.pop() {
                    self.selection_undo.push(current);
                    self.restore_selection_snapshot(next);
                }
            }
            HistoryKind::Transform => {
                let Ok((size, layers)) = phototux_canvas::snapshot_document_layers() else {
                    return;
                };
                let Some(graph) = self.engine.graph.clone() else {
                    return;
                };
                let current = TransformSnapshot {
                    size,
                    layers,
                    graph,
                };
                if let Some(next) = self.transform_redo.pop() {
                    self.transform_undo.push(current);
                    self.restore_transform_snapshot(next);
                }
            }
        }
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.emit_doc_fields();
    }

    fn commit_selection_shape(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        combine: &str,
        shape: SelectionShape,
        label: &str,
    ) {
        if !self.engine.has_document || width <= 0 || height <= 0 {
            return;
        }
        let mode = SelectionCombine::parse(combine);
        let rect = SelectionRect {
            x,
            y,
            width: width as u32,
            height: height as u32,
        };
        self.push_selection_snapshot();
        let gpu_result = match shape {
            SelectionShape::Rect => phototux_canvas::selection_apply_rect(rect, mode),
            SelectionShape::Ellipse => phototux_canvas::selection_apply_ellipse(rect, mode),
        };
        if let Err(error) = gpu_result {
            self.report_gpu("selection apply", &error);
        }
        match shape {
            SelectionShape::Rect => self.engine.selection.set_rect(rect, mode),
            SelectionShape::Ellipse => self.engine.selection.set_ellipse(rect, mode),
        }
        self.selection_preview_active = false;
        self.engine.history.push_selection(label);
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_selection_fields();
        self.can_undo_changed();
        self.history_labels_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn select_rect(&mut self, x: i32, y: i32, width: i32, height: i32, combine: String) {
        self.commit_selection_shape(
            x,
            y,
            width,
            height,
            &combine,
            SelectionShape::Rect,
            "Rectangular selection",
        );
    }

    #[qslot]
    fn select_ellipse(&mut self, x: i32, y: i32, width: i32, height: i32, combine: String) {
        self.commit_selection_shape(
            x,
            y,
            width,
            height,
            &combine,
            SelectionShape::Ellipse,
            "Elliptical selection",
        );
    }

    #[qslot]
    fn select_none(&mut self) {
        if !self.engine.has_document {
            return;
        }
        if !self.engine.selection.active
            && phototux_canvas::selection_snapshot()
                .map(|m| m.iter().all(|&v| v == 0))
                .unwrap_or(true)
        {
            return;
        }
        self.push_selection_snapshot();
        if let Err(error) = phototux_canvas::selection_clear() {
            self.report_gpu("deselect", &error);
        }
        self.engine.selection.clear();
        self.selection_preview_active = false;
        self.engine.history.push_selection("Deselect");
        self.sync_from_engine();
        self.emit_selection_fields();
        self.can_undo_changed();
        self.history_labels_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn select_all(&mut self) {
        if !self.engine.has_document {
            return;
        }
        self.push_selection_snapshot();
        if let Err(error) = phototux_canvas::selection_select_all() {
            self.report_gpu("select all", &error);
        }
        self.engine
            .selection
            .select_all(self.engine.size.width, self.engine.size.height);
        self.selection_preview_active = false;
        self.engine.history.push_selection("Select all");
        self.sync_from_engine();
        self.emit_selection_fields();
        self.can_undo_changed();
        self.history_labels_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn invert_selection(&mut self) {
        if !self.engine.has_document {
            return;
        }
        self.push_selection_snapshot();
        if let Err(error) = phototux_canvas::selection_invert() {
            self.report_gpu("invert selection", &error);
        }
        self.engine
            .selection
            .invert_bounds(self.engine.size.width, self.engine.size.height);
        self.selection_preview_active = false;
        self.engine.history.push_selection("Invert selection");
        self.sync_from_engine();
        self.emit_selection_fields();
        self.can_undo_changed();
        self.history_labels_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn set_selection_combine(&mut self, combine: String) {
        let mode = SelectionCombine::parse(&combine);
        self.engine.selection.combine = mode;
        self.selection_combine = mode.as_str().to_owned();
        self.selection_combine_changed();
    }

    #[qslot]
    fn set_selection_preview(&mut self, active: bool, x: i32, y: i32, width: i32, height: i32) {
        self.selection_preview_active = active;
        self.selection_preview_x = x;
        self.selection_preview_y = y;
        self.selection_preview_w = width.max(0);
        self.selection_preview_h = height.max(0);
        self.selection_preview_active_changed();
        self.selection_preview_x_changed();
        self.selection_preview_y_changed();
        self.selection_preview_w_changed();
        self.selection_preview_h_changed();
    }

    #[qslot]
    fn copy_selection(&mut self) {
        let Ok((width, height, pixels)) = phototux_canvas::read_composite_rgba() else {
            return;
        };
        self.clipboard_rgba = Some((width, height, pixels));
        self.status_text = "Copied".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn paste_as_new_layer(&mut self) {
        let Some((width, height, pixels)) = self.clipboard_rgba.clone() else {
            self.fail_io("Paste", "clipboard empty");
            return;
        };
        let result = {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            undo_actions::add_layer(graph, history, Some("Pasted".into()))
        };
        match result {
            Ok(id) => {
                if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
                    // Dimension mismatch: still keep empty layer.
                    let _ = (width, height, error);
                }
                self.recomposite();
                self.mark_dirty();
                self.sync_from_engine();
                self.emit_layer_fields();
            }
            Err(error) => self.report_gpu("paste", &error.to_string()),
        }
    }

    #[qslot]
    fn add_group_layer(&mut self) {
        let result = (|| {
            let SessionState { graph, history, .. } = &mut self.engine;
            let graph = graph.as_mut().ok_or(DocumentError::NoDocument)?;
            let id = graph.add_group_top(None)?;
            let index = graph.index_of(id).unwrap_or(0);
            let layer = graph
                .get(id)
                .cloned()
                .ok_or(DocumentError::LayerMissingAfterAdd)?;
            history.push_graph_applied(
                phototux_engine::GraphCommand::AddLayer { id, index, layer },
                "Add group",
            );
            Ok::<_, DocumentError>(())
        })();
        match result {
            Ok(()) => {
                self.recomposite();
                self.mark_dirty();
                self.sync_from_engine();
                self.emit_layer_fields();
            }
            Err(error) => self.report_gpu("add group", &error.to_string()),
        }
    }

    #[qslot]
    fn add_text_layer(&mut self, text: String) {
        let content = TextContent {
            text,
            ..TextContent::default()
        };
        let result = (|| {
            let SessionState { graph, history, .. } = &mut self.engine;
            let graph = graph.as_mut().ok_or(DocumentError::NoDocument)?;
            let id = graph.add_text_top(None, content)?;
            let index = graph.index_of(id).unwrap_or(0);
            let layer = graph
                .get(id)
                .cloned()
                .ok_or(DocumentError::LayerMissingAfterAdd)?;
            history.push_graph_applied(
                phototux_engine::GraphCommand::AddLayer { id, index, layer },
                "Add text layer",
            );
            Ok::<_, DocumentError>(())
        })();
        match result {
            Ok(()) => {
                self.recomposite();
                self.mark_dirty();
                self.sync_from_engine();
                self.emit_layer_fields();
            }
            Err(error) => self.report_gpu("add text", &error.to_string()),
        }
    }

    #[qslot]
    fn add_adjustment_layer(&mut self, kind: String) {
        let params = match kind.as_str() {
            "invert" => AdjustmentParams::Invert,
            "threshold" => AdjustmentParams::Threshold { level: 0.5 },
            "posterize" => AdjustmentParams::Posterize { levels: 8 },
            "hue" => AdjustmentParams::HueSaturation {
                hue: 0.0,
                saturation: 0.0,
                lightness: 0.0,
            },
            _ => AdjustmentParams::BrightnessContrast {
                brightness: 0.0,
                contrast: 0.0,
            },
        };
        let result = (|| {
            let SessionState { graph, history, .. } = &mut self.engine;
            let graph = graph.as_mut().ok_or(DocumentError::NoDocument)?;
            let id = graph.add_adjustment_top(None, params)?;
            let index = graph.index_of(id).unwrap_or(0);
            let layer = graph
                .get(id)
                .cloned()
                .ok_or(DocumentError::LayerMissingAfterAdd)?;
            history.push_graph_applied(
                phototux_engine::GraphCommand::AddLayer { id, index, layer },
                "Add adjustment",
            );
            Ok::<_, DocumentError>(())
        })();
        match result {
            Ok(()) => {
                self.recomposite();
                self.mark_dirty();
                self.sync_from_engine();
                self.emit_layer_fields();
            }
            Err(error) => self.report_gpu("add adjustment", &error.to_string()),
        }
    }

    #[qslot]
    fn add_mask_to_active(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let can_add = self
            .engine
            .graph
            .as_ref()
            .and_then(|graph| graph.get(id))
            .is_some_and(|layer| layer.mask.is_none());
        if !can_add {
            return;
        }
        if let Err(error) = phototux_canvas::ensure_mask(id) {
            self.report_gpu("add mask", &error);
            return;
        }
        {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            let prev = graph.get(id).and_then(|l| l.mask.clone());
            let next = Some(phototux_engine::LayerMask::default());
            if graph.set_mask(id, next.clone()).is_none() {
                return;
            }
            history.push_graph_applied(
                phototux_engine::GraphCommand::SetMask { id, prev, next },
                "Add layer mask",
            );
        }
        self.engine.mask_edit_layer = Some(id);
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn delete_mask_on_active(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let has_mask = self
            .engine
            .graph
            .as_ref()
            .and_then(|graph| graph.get(id))
            .is_some_and(|layer| layer.mask.is_some());
        if !has_mask {
            return;
        }
        if let Err(error) = phototux_canvas::remove_mask(id) {
            self.report_gpu("delete mask", &error);
            return;
        }
        {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            let prev = graph.get(id).and_then(|layer| layer.mask.clone());
            if graph.set_mask(id, None).is_none() {
                return;
            }
            history.push_graph_applied(
                phototux_engine::GraphCommand::SetMask {
                    id,
                    prev,
                    next: None,
                },
                "Delete layer mask",
            );
        }
        if self.engine.mask_edit_layer == Some(id) {
            self.engine.mask_edit_layer = None;
        }
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn set_mask_enabled_on_active(&mut self, enabled: bool) {
        let Some(id) = self.active_id() else {
            return;
        };
        let mut changed = false;
        {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            let Some(prev) = graph.get(id).and_then(|layer| layer.mask.clone()) else {
                return;
            };
            if prev.enabled == enabled {
                return;
            }
            let mut next = prev.clone();
            next.enabled = enabled;
            if graph.set_mask(id, Some(next.clone())).is_some() {
                history.push_graph_applied(
                    phototux_engine::GraphCommand::SetMask {
                        id,
                        prev: Some(prev),
                        next: Some(next),
                    },
                    if enabled {
                        "Enable layer mask"
                    } else {
                        "Disable layer mask"
                    },
                );
                changed = true;
            }
        }
        if changed {
            self.recomposite();
            self.mark_dirty();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn set_mask_edit_target(&mut self, edit_mask: bool) {
        let Some(id) = self.active_id() else {
            return;
        };
        if edit_mask
            && self
                .engine
                .graph
                .as_ref()
                .and_then(|graph| graph.get(id))
                .is_none_or(|layer| layer.mask.is_none())
        {
            return;
        }
        self.engine.mask_edit_layer = edit_mask.then_some(id);
        self.sync_from_engine();
        self.mask_edit_active_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn set_clips_to_below_on_active(&mut self, clips: bool) {
        let Some(id) = self.active_id() else {
            return;
        };
        let mut changed = false;
        {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            let Some(prev) = graph.set_clips_to_below(id, clips) else {
                return;
            };
            if prev != clips {
                history.push_graph_applied(
                    phototux_engine::GraphCommand::SetClipsToBelow {
                        id,
                        prev,
                        next: clips,
                    },
                    if clips {
                        "Create clipping mask"
                    } else {
                        "Release clipping mask"
                    },
                );
                changed = true;
            }
        }
        if changed {
            self.recomposite();
            self.mark_dirty();
            self.sync_from_engine();
            self.emit_layer_fields();
        }
    }

    #[qslot]
    fn set_guides_visible(&mut self, visible: bool) {
        self.engine.guides.show_guides = visible;
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
    }

    #[qslot]
    fn swap_fg_bg(&mut self) {
        self.engine.colors.swap();
        let fg = self.engine.colors.foreground;
        self.engine.set_brush_color(fg[0], fg[1], fg[2], fg[3]);
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_color_changed();
    }

    #[qslot]
    fn begin_transform(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let baseline = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.transform)
            .unwrap_or_default();
        self.engine.transform_session = Some(TransformSession::new(id, baseline));
        self.sync_transform_fields();
        self.emit_transform_fields();
    }

    #[qslot]
    fn update_transform_draft(
        &mut self,
        translate_x: f32,
        translate_y: f32,
        scale_x: f32,
        scale_y: f32,
        rotation_deg: f32,
        constrain: bool,
    ) {
        let Some(session) = self.engine.transform_session.as_mut() else {
            return;
        };
        let mut sx = scale_x.max(0.01);
        let mut sy = scale_y.max(0.01);
        if constrain {
            let uniform = sx.abs().max(sy.abs());
            sx = uniform.copysign(sx);
            sy = uniform.copysign(sy);
        }
        session.constrain_aspect = constrain;
        session.draft = LayerTransform {
            translate_x,
            translate_y,
            scale_x: sx,
            scale_y: sy,
            rotation_deg,
        };
        if let Some(graph) = self.engine.graph.as_mut() {
            if let Some(layer) = graph.get_mut(session.layer_id) {
                layer.transform = session.draft;
            }
            graph.revision = graph.revision.wrapping_add(1);
        }
        self.recomposite();
        self.sync_transform_fields();
        self.emit_transform_fields();
        self.graph_revision_changed();
        self.composite_ms_changed();
    }

    #[qslot]
    fn commit_transform(&mut self) {
        let Some(session) = self.engine.transform_session.take() else {
            return;
        };
        if session.draft.is_identity() {
            self.sync_transform_fields();
            self.emit_transform_fields();
            return;
        }
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        match phototux_canvas::bake_layer_transform(session.layer_id, session.draft, &layers) {
            Ok(ms) => {
                if let Some(graph) = self.engine.graph.as_mut() {
                    if let Some(layer) = graph.get_mut(session.layer_id) {
                        layer.transform = LayerTransform::identity();
                    }
                    graph.revision = graph.revision.wrapping_add(1);
                }
                self.engine.set_composite_ms(ms);
                self.engine.history.push_transform("Free Transform");
                self.mark_dirty();
            }
            Err(error) => {
                self.engine.transform_session = Some(session);
                self.report_gpu("transform bake", &error);
            }
        }
        self.sync_from_engine();
        self.emit_layer_fields();
        self.emit_transform_fields();
    }

    #[qslot]
    fn cancel_transform(&mut self) {
        let Some(session) = self.engine.transform_session.take() else {
            return;
        };
        if let Some(graph) = self.engine.graph.as_mut() {
            if let Some(layer) = graph.get_mut(session.layer_id) {
                layer.transform = session.baseline;
            }
            graph.revision = graph.revision.wrapping_add(1);
        }
        self.recomposite();
        self.sync_from_engine();
        self.emit_transform_fields();
        self.graph_revision_changed();
    }

    #[qslot]
    fn set_crop_preview(&mut self, active: bool, x: i32, y: i32, width: i32, height: i32) {
        self.crop_preview_active = active;
        self.crop_preview_x = x;
        self.crop_preview_y = y;
        self.crop_preview_w = width.max(0);
        self.crop_preview_h = height.max(0);
        self.emit_transform_fields();
    }

    #[qslot]
    fn commit_crop(&mut self, x: i32, y: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 || !self.engine.has_document {
            return;
        }
        let rect = CropRect {
            x,
            y,
            width: width as u32,
            height: height as u32,
        };
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| {
                let mut layers = g.layers().to_vec();
                for layer in &mut layers {
                    layer.transform = LayerTransform::identity();
                }
                layers
            })
            .unwrap_or_default();
        match phototux_canvas::crop_document(rect, &layers) {
            Ok((new_size, ms)) => {
                if let Some(graph) = self.engine.graph.as_mut() {
                    graph.size = new_size;
                    for layer in graph.layers_mut() {
                        layer.transform = LayerTransform::identity();
                    }
                    graph.revision = graph.revision.wrapping_add(1);
                }
                self.engine.size = new_size;
                self.engine.set_composite_ms(ms);
                self.engine.history.push_transform("Crop");
                self.engine.selection.clear();
                self.crop_preview_active = false;
                self.clear_selection_stacks();
                self.mark_dirty();
                self.engine.zoom_to_fit();
            }
            Err(error) => self.report_gpu("crop", &error),
        }
        self.sync_from_engine();
        self.emit_doc_fields();
    }

    #[qslot]
    fn cancel_crop(&mut self) {
        self.crop_preview_active = false;
        self.emit_transform_fields();
    }

    #[qslot]
    fn flip_active_layer(&mut self, horizontal: bool) {
        let Some(id) = self.active_id() else {
            return;
        };
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        match phototux_canvas::flip_layer(id, horizontal, &layers) {
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
                self.engine.history.push_transform(if horizontal {
                    "Flip Horizontal"
                } else {
                    "Flip Vertical"
                });
                self.mark_dirty();
            }
            Err(error) => self.report_gpu("flip", &error),
        }
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn rotate_canvas_90_cw(&mut self) {
        if !self.engine.has_document {
            return;
        }
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        match phototux_canvas::rotate_canvas_90_cw(&layers) {
            Ok((new_size, ms)) => {
                if let Some(graph) = self.engine.graph.as_mut() {
                    graph.size = new_size;
                    graph.revision = graph.revision.wrapping_add(1);
                }
                self.engine.size = new_size;
                self.engine.set_composite_ms(ms);
                self.engine.history.push_transform("Rotate 90° CW");
                self.engine.selection.clear();
                self.clear_selection_stacks();
                self.mark_dirty();
                self.engine.zoom_to_fit();
            }
            Err(error) => self.report_gpu("rotate canvas", &error),
        }
        self.sync_from_engine();
        self.emit_doc_fields();
    }
}
