//! QML-facing session via qtbridge (ADR-003). Package name `phototux_ui` → `import phototux_ui`.

mod file_worker;
mod prefs;

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

use file_worker::{FileCommand, FileEvent, FileWorker};
use phototux_canvas::PaintWorker;
use phototux_engine::{
    AdjustmentParams, CommandArgs, CommandEffects, CommandError, CropRect, DocumentError,
    DocumentGraph, DocumentSize, EngineCommand, EngineEvent, FilterParams, Guide, GuideOrientation,
    HistoryKind, HostHistoryAction, LayerId, LayerKind, LayerStyle, LayerTransform, PathPoint,
    SelectionCombine, SelectionRect, SelectionShape, SelectionState, SessionState, TextContent,
    TransformSession, VectorPath, bake_text_rgba8, command_id, contract_mask_r8, expand_mask_r8,
    feather_mask_r8, stroke_path_rgba8, tool_id, undo_actions,
};
use prefs::Preferences;

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
    /// Live lasso/polygon path: `x,y|x,y|...` in document pixels.
    selection_path: String,
    selection_path_active: bool,
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
    active_blend: String,
    foreground_hex: String,
    background_hex: String,
    recent_colors: String,
    viewport_width: f32,
    viewport_height: f32,
    adjustment_kind: String,
    adjustment_p0: f32,
    adjustment_p1: f32,
    adjustment_p2: f32,
    has_gaussian_blur: bool,
    gaussian_radius: f32,
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
    /// Generation pinned when a Save was submitted (Phase 2 receipt).
    pending_save_generation: Option<u64>,
    prefs: Preferences,
    panel_descriptors_json: String,
    tool_descriptors_json: String,
    preferences_open: bool,
    pref_show_guides: bool,
    pref_restore_last_tool: bool,
    panel_navigator_visible: bool,
    panel_swatches_visible: bool,
    panel_layers_visible: bool,
    panel_history_visible: bool,
    panel_properties_visible: bool,
    text_layer_active: bool,
    text_body: String,
    text_font_family: String,
    text_font_size: f32,
    text_tracking: f32,
    text_line_spacing: f32,
    text_alignment: i32,
    text_color_hex: String,
    pref_show_grid: bool,
    pref_show_rulers: bool,
    pref_snap: bool,
    guides_json: String,
    grid_spacing: f32,
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
            selection_path: String::new(),
            selection_path_active: false,
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
            active_blend: "normal".to_owned(),
            foreground_hex: "#000000".to_owned(),
            background_hex: "#FFFFFF".to_owned(),
            recent_colors: String::new(),
            viewport_width: 1.0,
            viewport_height: 1.0,
            adjustment_kind: String::new(),
            adjustment_p0: 0.0,
            adjustment_p1: 0.0,
            adjustment_p2: 1.0,
            has_gaussian_blur: false,
            gaussian_radius: 0.0,
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
            pending_save_generation: None,
            prefs: Preferences::default(),
            panel_descriptors_json: phototux_engine::panels_json(),
            tool_descriptors_json: phototux_engine::tools_json(),
            preferences_open: false,
            pref_show_guides: true,
            pref_restore_last_tool: false,
            panel_navigator_visible: true,
            panel_swatches_visible: true,
            panel_layers_visible: true,
            panel_history_visible: true,
            panel_properties_visible: true,
            text_layer_active: false,
            text_body: String::new(),
            text_font_family: "Noto Sans".into(),
            text_font_size: 24.0,
            text_tracking: 0.0,
            text_line_spacing: 1.2,
            text_alignment: 0,
            text_color_hex: "#000000".into(),
            pref_show_grid: false,
            pref_show_rulers: false,
            pref_snap: true,
            guides_json: "[]".into(),
            grid_spacing: 32.0,
        };
        out.apply_loaded_preferences();
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
        let active_layer = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)));
        self.active_opacity = active_layer.map(|l| l.opacity).unwrap_or(1.0);
        self.active_blend = active_layer
            .map(|l| l.blend.as_str().to_owned())
            .unwrap_or_else(|| "normal".to_owned());
        self.sync_color_fields();
        self.viewport_width = self.engine.viewport_w;
        self.viewport_height = self.engine.viewport_h;
        self.sync_adjustment_fields();
        self.sync_text_fields();
        self.sync_guides_fields();
        self.status_text = self.engine.status_summary();
    }

    fn sync_color_fields(&mut self) {
        use phototux_engine::ColorState;
        self.foreground_hex = ColorState::to_hex(self.engine.colors.foreground);
        self.background_hex = ColorState::to_hex(self.engine.colors.background);
        self.recent_colors = self.engine.colors.recent_hex_joined();
        let fg = self.engine.colors.foreground;
        self.brush_r = fg[0];
        self.brush_g = fg[1];
        self.brush_b = fg[2];
    }

    fn sync_adjustment_fields(&mut self) {
        let layer = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)));
        let Some(layer) = layer else {
            self.adjustment_kind.clear();
            self.adjustment_p0 = 0.0;
            self.adjustment_p1 = 0.0;
            self.adjustment_p2 = 1.0;
            self.has_gaussian_blur = false;
            self.gaussian_radius = 0.0;
            return;
        };
        match layer.adjustment.as_ref() {
            Some(AdjustmentParams::BrightnessContrast {
                brightness,
                contrast,
            }) => {
                self.adjustment_kind = "brightness".into();
                self.adjustment_p0 = *brightness;
                self.adjustment_p1 = *contrast;
                self.adjustment_p2 = 0.0;
            }
            Some(AdjustmentParams::Levels {
                black,
                white,
                gamma,
            }) => {
                self.adjustment_kind = "levels".into();
                self.adjustment_p0 = *black;
                self.adjustment_p1 = *white;
                self.adjustment_p2 = *gamma;
            }
            Some(other) => {
                self.adjustment_kind = other.kind_key().into();
                self.adjustment_p0 = 0.0;
                self.adjustment_p1 = 0.0;
                self.adjustment_p2 = 0.0;
            }
            None => {
                self.adjustment_kind.clear();
                self.adjustment_p0 = 0.0;
                self.adjustment_p1 = 0.0;
                self.adjustment_p2 = 1.0;
            }
        }
        let gaussian = layer.effects.iter().find_map(|effect| {
            if !effect.enabled {
                return None;
            }
            match effect.params {
                FilterParams::GaussianBlur { radius } => Some(radius),
                _ => None,
            }
        });
        self.has_gaussian_blur = gaussian.is_some();
        self.gaussian_radius = gaussian.unwrap_or(0.0);
    }

    fn sync_text_fields(&mut self) {
        let layer = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)));
        let Some(layer) = layer.filter(|l| l.kind == LayerKind::Text) else {
            self.text_layer_active = false;
            return;
        };
        self.text_layer_active = true;
        let content = layer.text.clone().unwrap_or_default();
        self.text_body = content.text;
        self.text_font_family = content.font_family;
        self.text_font_size = content.font_size_pt;
        self.text_tracking = content.tracking;
        self.text_line_spacing = content.line_spacing;
        self.text_alignment = i32::from(content.alignment);
        self.text_color_hex = phototux_engine::ColorState::to_hex(content.color_rgba);
    }

    fn sync_guides_fields(&mut self) {
        self.pref_show_guides = self.engine.guides.show_guides;
        self.pref_show_grid = self.engine.guides.show_grid;
        self.pref_show_rulers = self.engine.guides.show_rulers;
        self.pref_snap = self.engine.guides.snap;
        self.grid_spacing = self.engine.guides.grid_spacing;
        self.guides_json = self.engine.guides.guides_json();
    }

    fn emit_text_fields(&mut self) {
        self.text_layer_active_changed();
        self.text_body_changed();
        self.text_font_family_changed();
        self.text_font_size_changed();
        self.text_tracking_changed();
        self.text_line_spacing_changed();
        self.text_alignment_changed();
        self.text_color_hex_changed();
    }

    fn emit_guides_fields(&mut self) {
        self.pref_show_guides_changed();
        self.pref_show_grid_changed();
        self.pref_show_rulers_changed();
        self.pref_snap_changed();
        self.guides_json_changed();
        self.grid_spacing_changed();
    }

    fn emit_camera_fields(&mut self) {
        self.zoom_changed();
        self.pan_x_changed();
        self.pan_y_changed();
        self.status_text_changed();
    }

    fn emit_color_fields(&mut self) {
        self.brush_color_changed();
        self.foreground_hex_changed();
        self.background_hex_changed();
        self.recent_colors_changed();
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
        self.active_blend_changed();
        self.foreground_hex_changed();
        self.background_hex_changed();
        self.recent_colors_changed();
        self.viewport_width_changed();
        self.viewport_height_changed();
        self.brush_color_changed();
        self.adjustment_kind_changed();
        self.adjustment_p0_changed();
        self.adjustment_p1_changed();
        self.adjustment_p2_changed();
        self.has_gaussian_blur_changed();
        self.gaussian_radius_changed();
        self.composite_ms_changed();
        self.status_text_changed();
        self.emit_text_fields();
        self.emit_guides_fields();
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
        self.selection_path_changed();
        self.selection_path_active_changed();
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

    fn apply_loaded_preferences(&mut self) {
        self.prefs = Preferences::load();
        self.sync_pref_fields_from_store();
        self.engine.guides.show_guides = self.prefs.show_guides;
        self.engine.guides.show_grid = self.prefs.show_grid;
        self.engine.guides.show_rulers = self.prefs.show_rulers;
        self.engine.guides.snap = self.prefs.snap_enabled;
        if self.prefs.restore_last_tool && !self.prefs.last_tool.is_empty() {
            let _ = self.engine.invoke(
                command_id::VIEW_SET_TOOL,
                CommandArgs::Tool {
                    tool: self.prefs.last_tool.clone(),
                },
            );
        }
    }

    fn sync_pref_fields_from_store(&mut self) {
        self.pref_show_guides = self.prefs.show_guides;
        self.pref_show_grid = self.prefs.show_grid;
        self.pref_show_rulers = self.prefs.show_rulers;
        self.pref_snap = self.prefs.snap_enabled;
        self.pref_restore_last_tool = self.prefs.restore_last_tool;
        self.panel_navigator_visible = self.prefs.panel_navigator;
        self.panel_swatches_visible = self.prefs.panel_swatches;
        self.panel_layers_visible = self.prefs.panel_layers;
        self.panel_history_visible = self.prefs.panel_history;
        self.panel_properties_visible = self.prefs.panel_properties;
        self.sync_guides_fields();
    }

    fn emit_pref_fields(&mut self) {
        self.pref_show_guides_changed();
        self.pref_show_grid_changed();
        self.pref_show_rulers_changed();
        self.pref_snap_changed();
        self.pref_restore_last_tool_changed();
        self.panel_navigator_visible_changed();
        self.panel_swatches_visible_changed();
        self.panel_layers_visible_changed();
        self.panel_history_visible_changed();
        self.panel_properties_visible_changed();
        self.guides_json_changed();
        self.grid_spacing_changed();
    }

    fn persist_prefs(&mut self) {
        if let Err(error) = self.prefs.save() {
            self.status_text = format!("Preferences save failed: {error}");
            self.status_text_changed();
        }
    }

    fn invoke_command(&mut self, id: &str, args: CommandArgs) -> Result<(), CommandError> {
        let effects = self.engine.invoke(id, args)?;
        self.apply_command_effects(effects);
        Ok(())
    }

    fn apply_command_effects(&mut self, effects: CommandEffects) {
        if let Some(host) = effects.host_history {
            self.apply_host_history(host);
        }
        if effects.recomposite {
            self.recomposite();
        }
        if effects.dirty {
            self.mark_dirty();
        }
        if effects.sync_camera {
            self.sync_camera_from_engine();
            self.emit_camera_fields();
        }
        if effects.sync_layers {
            self.sync_from_engine();
            self.emit_layer_fields();
            self.active_blend_changed();
        }
        if effects.sync_doc {
            self.sync_from_engine();
            self.emit_doc_fields();
        }
        if effects.generation > 0 {
            self.graph_revision = effects.generation.min(i32::MAX as u64) as i32;
            self.graph_revision_changed();
        }
    }

    fn apply_host_history(&mut self, action: HostHistoryAction) {
        match action {
            HostHistoryAction::Undo(HistoryKind::Stroke) => match phototux_canvas::undo_stroke() {
                Ok(ms) => self.engine.set_composite_ms(ms),
                Err(error) => self.report_gpu("stroke undo", &error),
            },
            HostHistoryAction::Redo(HistoryKind::Stroke) => match phototux_canvas::redo_stroke() {
                Ok(ms) => self.engine.set_composite_ms(ms),
                Err(error) => self.report_gpu("stroke redo", &error),
            },
            HostHistoryAction::Undo(HistoryKind::Selection) => {
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
            HostHistoryAction::Redo(HistoryKind::Selection) => {
                let current = SelectionSnapshot {
                    state: self.engine.selection.clone(),
                    mask: phototux_canvas::selection_snapshot().unwrap_or_default(),
                };
                if let Some(next) = self.selection_redo.pop() {
                    self.selection_undo.push(current);
                    self.restore_selection_snapshot(next);
                }
            }
            HostHistoryAction::Undo(HistoryKind::Transform) => {
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
            HostHistoryAction::Redo(HistoryKind::Transform) => {
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
            HostHistoryAction::Undo(HistoryKind::Graph)
            | HostHistoryAction::Redo(HistoryKind::Graph) => {
                self.recomposite();
            }
        }
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.emit_doc_fields();
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
        "selectionPath",
        Member = selection_path,
        Notify = selection_path_changed
    );
    qproperty!(
        "selectionPathActive",
        Member = selection_path_active,
        Notify = selection_path_active_changed
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
    qproperty!(
        "activeBlend",
        Member = active_blend,
        Notify = active_blend_changed
    );
    qproperty!(
        "foregroundHex",
        Member = foreground_hex,
        Notify = foreground_hex_changed
    );
    qproperty!(
        "backgroundHex",
        Member = background_hex,
        Notify = background_hex_changed
    );
    qproperty!(
        "recentColors",
        Member = recent_colors,
        Notify = recent_colors_changed
    );
    qproperty!(
        "viewportWidth",
        Member = viewport_width,
        Notify = viewport_width_changed
    );
    qproperty!(
        "viewportHeight",
        Member = viewport_height,
        Notify = viewport_height_changed
    );
    qproperty!(
        "adjustmentKind",
        Member = adjustment_kind,
        Notify = adjustment_kind_changed
    );
    qproperty!(
        "adjustmentP0",
        Member = adjustment_p0,
        Notify = adjustment_p0_changed
    );
    qproperty!(
        "adjustmentP1",
        Member = adjustment_p1,
        Notify = adjustment_p1_changed
    );
    qproperty!(
        "adjustmentP2",
        Member = adjustment_p2,
        Notify = adjustment_p2_changed
    );
    qproperty!(
        "hasGaussianBlur",
        Member = has_gaussian_blur,
        Notify = has_gaussian_blur_changed
    );
    qproperty!(
        "gaussianRadius",
        Member = gaussian_radius,
        Notify = gaussian_radius_changed
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
    qproperty!(
        "panelDescriptorsJson",
        Member = panel_descriptors_json,
        Notify = panel_descriptors_json_changed
    );
    qproperty!(
        "toolDescriptorsJson",
        Member = tool_descriptors_json,
        Notify = tool_descriptors_json_changed
    );
    qproperty!(
        "preferencesOpen",
        Member = preferences_open,
        Notify = preferences_open_changed
    );
    qproperty!(
        "prefShowGuides",
        Member = pref_show_guides,
        Notify = pref_show_guides_changed
    );
    qproperty!(
        "prefShowGrid",
        Member = pref_show_grid,
        Notify = pref_show_grid_changed
    );
    qproperty!(
        "prefShowRulers",
        Member = pref_show_rulers,
        Notify = pref_show_rulers_changed
    );
    qproperty!("prefSnap", Member = pref_snap, Notify = pref_snap_changed);
    qproperty!(
        "guidesJson",
        Member = guides_json,
        Notify = guides_json_changed
    );
    qproperty!(
        "gridSpacing",
        Member = grid_spacing,
        Notify = grid_spacing_changed
    );
    qproperty!(
        "textLayerActive",
        Member = text_layer_active,
        Notify = text_layer_active_changed
    );
    qproperty!("textBody", Member = text_body, Notify = text_body_changed);
    qproperty!(
        "textFontFamily",
        Member = text_font_family,
        Notify = text_font_family_changed
    );
    qproperty!(
        "textFontSize",
        Member = text_font_size,
        Notify = text_font_size_changed
    );
    qproperty!(
        "textTracking",
        Member = text_tracking,
        Notify = text_tracking_changed
    );
    qproperty!(
        "textLineSpacing",
        Member = text_line_spacing,
        Notify = text_line_spacing_changed
    );
    qproperty!(
        "textAlignment",
        Member = text_alignment,
        Notify = text_alignment_changed
    );
    qproperty!(
        "textColorHex",
        Member = text_color_hex,
        Notify = text_color_hex_changed
    );
    qproperty!(
        "prefRestoreLastTool",
        Member = pref_restore_last_tool,
        Notify = pref_restore_last_tool_changed
    );
    qproperty!(
        "panelNavigatorVisible",
        Member = panel_navigator_visible,
        Notify = panel_navigator_visible_changed
    );
    qproperty!(
        "panelSwatchesVisible",
        Member = panel_swatches_visible,
        Notify = panel_swatches_visible_changed
    );
    qproperty!(
        "panelLayersVisible",
        Member = panel_layers_visible,
        Notify = panel_layers_visible_changed
    );
    qproperty!(
        "panelHistoryVisible",
        Member = panel_history_visible,
        Notify = panel_history_visible_changed
    );
    qproperty!(
        "panelPropertiesVisible",
        Member = panel_properties_visible,
        Notify = panel_properties_visible_changed
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
    fn selection_path_changed(&mut self);
    #[qsignal]
    fn selection_path_active_changed(&mut self);
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
    fn active_blend_changed(&mut self);
    #[qsignal]
    fn foreground_hex_changed(&mut self);
    #[qsignal]
    fn background_hex_changed(&mut self);
    #[qsignal]
    fn recent_colors_changed(&mut self);
    #[qsignal]
    fn viewport_width_changed(&mut self);
    #[qsignal]
    fn viewport_height_changed(&mut self);
    #[qsignal]
    fn adjustment_kind_changed(&mut self);
    #[qsignal]
    fn adjustment_p0_changed(&mut self);
    #[qsignal]
    fn adjustment_p1_changed(&mut self);
    #[qsignal]
    fn adjustment_p2_changed(&mut self);
    #[qsignal]
    fn has_gaussian_blur_changed(&mut self);
    #[qsignal]
    fn gaussian_radius_changed(&mut self);
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
    #[qsignal]
    fn panel_descriptors_json_changed(&mut self);
    #[qsignal]
    fn tool_descriptors_json_changed(&mut self);
    #[qsignal]
    fn preferences_open_changed(&mut self);
    #[qsignal]
    fn pref_show_guides_changed(&mut self);
    #[qsignal]
    fn pref_show_grid_changed(&mut self);
    #[qsignal]
    fn pref_show_rulers_changed(&mut self);
    #[qsignal]
    fn pref_snap_changed(&mut self);
    #[qsignal]
    fn guides_json_changed(&mut self);
    #[qsignal]
    fn grid_spacing_changed(&mut self);
    #[qsignal]
    fn text_layer_active_changed(&mut self);
    #[qsignal]
    fn text_body_changed(&mut self);
    #[qsignal]
    fn text_font_family_changed(&mut self);
    #[qsignal]
    fn text_font_size_changed(&mut self);
    #[qsignal]
    fn text_tracking_changed(&mut self);
    #[qsignal]
    fn text_line_spacing_changed(&mut self);
    #[qsignal]
    fn text_alignment_changed(&mut self);
    #[qsignal]
    fn text_color_hex_changed(&mut self);
    #[qsignal]
    fn pref_restore_last_tool_changed(&mut self);
    #[qsignal]
    fn panel_navigator_visible_changed(&mut self);
    #[qsignal]
    fn panel_swatches_visible_changed(&mut self);
    #[qsignal]
    fn panel_layers_visible_changed(&mut self);
    #[qsignal]
    fn panel_history_visible_changed(&mut self);
    #[qsignal]
    fn panel_properties_visible_changed(&mut self);

    #[qslot]
    fn assign_document_profile(&mut self, profile: String) {
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_ASSIGN_PROFILE,
            CommandArgs::AssignProfile { profile },
        ) {
            self.report_gpu("assign profile", &error.to_string());
        } else {
            self.status_text = format!(
                "Assigned profile (pixels not converted): {}",
                self.engine
                    .graph
                    .as_ref()
                    .map(|g| g.color.assigned_profile.as_str())
                    .unwrap_or("?")
            );
            self.status_text_changed();
        }
    }

    /// Convert document pixels into `profile` (destructive; DR-012).
    #[qslot]
    fn convert_document_profile(&mut self, profile: String) {
        let from = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.color.assigned_profile.clone())
            .unwrap_or_else(|| "sRGB".into());
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_CONVERT_PROFILE,
            CommandArgs::ConvertProfile {
                profile: profile.clone(),
            },
        ) {
            self.report_gpu("convert profile", &error.to_string());
            return;
        }
        let needs_rewrite = from != profile;
        if needs_rewrite {
            match phototux_canvas::read_all_layer_rgba() {
                Ok(layers) => {
                    for (id, _w, _h, mut pixels) in layers {
                        phototux_engine::convert_rgba8_profile(&mut pixels, &from, &profile);
                        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
                            self.report_gpu("convert profile upload", &error);
                            return;
                        }
                    }
                }
                Err(error) => {
                    self.report_gpu("convert profile read", &error);
                    return;
                }
            }
        }
        if let Some(graph) = self.engine.graph.as_mut() {
            graph.color.mark_converted();
            graph.bump_generation();
        }
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.status_text =
            format!("Converted pixels to {profile} (from {from}) — this rewrote layer data");
        self.status_text_changed();
    }

    #[qslot]
    fn open_preferences(&mut self) {
        self.preferences_open = true;
        self.preferences_open_changed();
    }

    #[qslot]
    fn close_preferences(&mut self) {
        self.preferences_open = false;
        self.preferences_open_changed();
    }

    #[qslot]
    fn set_pref_show_guides(&mut self, value: bool) {
        self.prefs.show_guides = value;
        self.pref_show_guides = value;
        self.engine.guides.show_guides = value;
        self.persist_prefs();
        self.pref_show_guides_changed();
    }

    #[qslot]
    fn set_pref_restore_last_tool(&mut self, value: bool) {
        self.prefs.restore_last_tool = value;
        self.pref_restore_last_tool = value;
        self.persist_prefs();
        self.pref_restore_last_tool_changed();
    }

    #[qslot]
    fn set_panel_navigator_visible(&mut self, value: bool) {
        self.prefs.panel_navigator = value;
        self.panel_navigator_visible = value;
        self.persist_prefs();
        self.panel_navigator_visible_changed();
    }

    #[qslot]
    fn set_panel_swatches_visible(&mut self, value: bool) {
        self.prefs.panel_swatches = value;
        self.panel_swatches_visible = value;
        self.persist_prefs();
        self.panel_swatches_visible_changed();
    }

    #[qslot]
    fn set_panel_layers_visible(&mut self, value: bool) {
        self.prefs.panel_layers = value;
        self.panel_layers_visible = value;
        self.persist_prefs();
        self.panel_layers_visible_changed();
    }

    #[qslot]
    fn set_panel_history_visible(&mut self, value: bool) {
        self.prefs.panel_history = value;
        self.panel_history_visible = value;
        self.persist_prefs();
        self.panel_history_visible_changed();
    }

    #[qslot]
    fn set_panel_properties_visible(&mut self, value: bool) {
        self.prefs.panel_properties = value;
        self.panel_properties_visible = value;
        self.persist_prefs();
        self.panel_properties_visible_changed();
    }

    #[qslot]
    fn reset_workspace(&mut self) {
        self.prefs.reset_workspace_essentials();
        self.sync_pref_fields_from_store();
        self.persist_prefs();
        self.emit_pref_fields();
        self.status_text = "Workspace reset to Essentials".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn set_zoom(&mut self, value: f32) {
        let _ = self.invoke_command(command_id::VIEW_ZOOM_TO, CommandArgs::Zoom { zoom: value });
    }

    #[qslot]
    fn set_pan(&mut self, world_x: f32, world_y: f32) {
        let _ = self.invoke_command(
            command_id::VIEW_PAN_TO,
            CommandArgs::Pan { world_x, world_y },
        );
    }

    #[qslot]
    fn center_view_on(&mut self, doc_x: f32, doc_y: f32) {
        let _ = self.invoke_command(
            command_id::VIEW_PAN_TO,
            CommandArgs::Pan {
                world_x: doc_x,
                world_y: doc_y,
            },
        );
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
        self.emit_color_fields();
    }

    #[qslot]
    fn set_foreground_rgb(&mut self, r: f32, g: f32, b: f32) {
        self.set_brush_color(r, g, b);
    }

    #[qslot]
    fn set_foreground_hex(&mut self, hex: String) {
        let Some(rgba) = phototux_engine::ColorState::from_hex(&hex) else {
            return;
        };
        self.set_brush_color(rgba[0], rgba[1], rgba[2]);
    }

    #[qslot]
    fn set_background_hex(&mut self, hex: String) {
        let Some(rgba) = phototux_engine::ColorState::from_hex(&hex) else {
            return;
        };
        self.engine.colors.set_background(rgba);
        self.sync_from_engine();
        self.emit_color_fields();
    }

    #[qslot]
    fn set_background_rgb(&mut self, r: f32, g: f32, b: f32) {
        self.engine.colors.set_background([
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
            1.0,
        ]);
        self.sync_from_engine();
        self.emit_color_fields();
    }

    #[qslot]
    fn pick_recent_color(&mut self, index: i32) {
        if index < 0 {
            return;
        }
        let Some(rgba) = self.engine.colors.recent.get(index as usize).copied() else {
            return;
        };
        self.set_brush_color(rgba[0], rgba[1], rgba[2]);
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
                | tool_id::SELECT_POLYGON
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
        let _ = self.invoke_command(
            command_id::VIEW_SET_TOOL,
            CommandArgs::Tool { tool: id.clone() },
        );
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.prefs.last_tool = id;
        if self.prefs.restore_last_tool {
            self.persist_prefs();
        }
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
                    let __gen = self.engine.bump_document_generation();
                    self.engine.history.push_stroke(
                        if self.mask_edit_active {
                            "Mask stroke"
                        } else {
                            "Brush stroke"
                        },
                        __gen,
                    );
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
                    layer_rasters,
                    flattened,
                    report,
                } => {
                    self.clear_selection_stacks();
                    self.clear_transform_stacks();
                    let open_result = if !layer_rasters.is_empty() {
                        phototux_canvas::open_document(graph.size, graph.layers()).and_then(|ms| {
                            for (id, raster) in &layer_rasters {
                                phototux_canvas::write_layer_rgba(*id, raster.pixels())?;
                            }
                            Ok(ms)
                        })
                    } else if let Some(raster) = flattened.as_ref() {
                        let Some(target_layer) = graph.active_id() else {
                            self.fail_io("Open", "PSD import has no layer");
                            continue;
                        };
                        phototux_canvas::open_raster_document(
                            graph.size,
                            graph.layers(),
                            target_layer,
                            raster.pixels(),
                        )
                    } else {
                        self.fail_io("Open", "PSD import produced no pixel data");
                        continue;
                    };
                    match open_result {
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
                    if let Some(pinned) = self.pending_save_generation.take() {
                        let clean = self.engine.mark_persisted(pinned);
                        self.dirty = !clean;
                    } else {
                        self.dirty = false;
                    }
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
        let _ = self.invoke_command(command_id::VIEW_ZOOM_TO_FIT, CommandArgs::None);
    }

    #[qslot]
    fn apply_size_preset(&mut self, label: String) {
        if self
            .invoke_command(
                command_id::DOCUMENT_NEW_PRESET,
                CommandArgs::NewPreset { label },
            )
            .is_ok()
        {
            self.open_gpu_document();
            self.document_name = "Untitled".to_owned();
            self.dirty = false;
            self.emit_doc_fields();
        }
    }

    #[qslot]
    fn apply_document_size(&mut self, width: i32, height: i32) {
        let w = width.max(1) as u32;
        let h = height.max(1) as u32;
        if self
            .invoke_command(
                command_id::DOCUMENT_NEW_SIZE,
                CommandArgs::NewSize {
                    width: w,
                    height: h,
                },
            )
            .is_ok()
        {
            self.open_gpu_document();
            self.document_name = "Untitled".to_owned();
            self.dirty = false;
            self.emit_doc_fields();
        }
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
        self.pending_save_generation = Some(graph.generation);
        self.io_busy = true;
        self.io_error.clear();
        self.status_text = format!("Saving {}…", path.display());
        self.io_busy_changed();
        self.status_text_changed();
        if let Err(error) = self.file_worker.send(FileCommand::SavePtx { path, graph }) {
            self.pending_save_generation = None;
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
        let is_psd = path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("psd"));
        self.io_busy = true;
        self.io_error.clear();
        self.status_text = format!("Exporting {}…", path.display());
        self.io_busy_changed();
        self.status_text_changed();
        let send_result = if is_psd {
            let Some(graph) = self.engine.graph.clone() else {
                self.fail_io("Export", "no document graph");
                return;
            };
            self.file_worker
                .send(FileCommand::ExportPsd { path, graph })
        } else {
            let format = match RasterFormat::from_path(&path) {
                Ok(format) => format,
                Err(error) => {
                    self.fail_io("Export", &error.to_string());
                    return;
                }
            };
            self.file_worker.send(FileCommand::Export { path, format })
        };
        if let Err(error) = send_result {
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
        if let Err(error) = self.invoke_command(command_id::LAYER_CREATE, CommandArgs::None) {
            self.report_gpu("add layer", &error.to_string());
        }
    }

    #[qslot]
    fn delete_active_layer(&mut self) {
        let _ = self.invoke_command(command_id::LAYER_DELETE, CommandArgs::None);
    }

    #[qslot]
    fn set_active_layer(&mut self, index: i32) {
        let _ = self.invoke_command(command_id::LAYER_SET_ACTIVE, CommandArgs::LayerIndex(index));
    }

    #[qslot]
    fn set_layer_visible(&mut self, index: i32, visible: bool) {
        let _ = self.invoke_command(
            command_id::LAYER_SET_VISIBILITY,
            CommandArgs::SetVisibility { index, visible },
        );
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
        let _ = self.invoke_command(
            command_id::LAYER_SET_OPACITY,
            CommandArgs::SetOpacity { opacity },
        );
    }

    #[qslot]
    fn set_active_blend(&mut self, blend: String) {
        let _ = self.invoke_command(command_id::LAYER_SET_BLEND, CommandArgs::SetBlend { blend });
    }

    #[qslot]
    fn move_active_layer(&mut self, to_index: i32) {
        let _ = self.invoke_command(command_id::LAYER_REORDER, CommandArgs::Reorder { to_index });
    }

    #[qslot]
    fn undo(&mut self) {
        let _ = self.invoke_command(command_id::HISTORY_UNDO, CommandArgs::None);
    }

    #[qslot]
    fn redo(&mut self) {
        let _ = self.invoke_command(command_id::HISTORY_REDO, CommandArgs::None);
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
        if matches!(shape, SelectionShape::Mask) {
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
            SelectionShape::Mask => Ok(()),
        };
        if let Err(error) = gpu_result {
            self.report_gpu("selection apply", &error);
        }
        match shape {
            SelectionShape::Rect => self.engine.selection.set_rect(rect, mode),
            SelectionShape::Ellipse => self.engine.selection.set_ellipse(rect, mode),
            SelectionShape::Mask => {}
        }
        self.selection_preview_active = false;
        self.clear_selection_path();
        let __gen = self.engine.bump_document_generation();
        self.engine.history.push_selection(label, __gen);
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_selection_fields();
        self.can_undo_changed();
        self.history_labels_changed();
        self.status_text_changed();
    }

    fn clear_selection_path(&mut self) {
        if !self.selection_path_active && self.selection_path.is_empty() {
            return;
        }
        self.selection_path.clear();
        self.selection_path_active = false;
        self.selection_path_changed();
        self.selection_path_active_changed();
    }

    fn parse_selection_path(points: &str) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for part in points.split('|') {
            let mut xy = part.split(',');
            let (Some(xs), Some(ys)) = (xy.next(), xy.next()) else {
                continue;
            };
            let Ok(x) = xs.trim().parse::<f32>() else {
                continue;
            };
            let Ok(y) = ys.trim().parse::<f32>() else {
                continue;
            };
            out.push((x, y));
        }
        out
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
    fn set_selection_path(&mut self, points: String) {
        self.selection_path = points;
        self.selection_path_active = !self.selection_path.is_empty();
        self.selection_path_changed();
        self.selection_path_active_changed();
    }

    #[qslot]
    fn select_polygon(&mut self, points: String, combine: String) {
        if !self.engine.has_document {
            return;
        }
        let parsed = Self::parse_selection_path(&points);
        if parsed.len() < 3 {
            self.clear_selection_path();
            return;
        }
        let mode = SelectionCombine::parse(&combine);
        let Some(bounds) = SelectionState::polygon_bounds(&parsed) else {
            self.clear_selection_path();
            return;
        };
        self.push_selection_snapshot();
        if let Err(error) = phototux_canvas::selection_apply_polygon(&parsed, mode) {
            self.report_gpu("polygon selection", &error);
        }
        self.engine.selection.set_mask_polygon(bounds, mode);
        self.selection_preview_active = false;
        self.clear_selection_path();
        let label = if self.engine.active_tool.contains("lasso") {
            "Lasso selection"
        } else {
            "Polygonal selection"
        };
        let __gen = self.engine.bump_document_generation();
        self.engine.history.push_selection(label, __gen);
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_selection_fields();
        self.can_undo_changed();
        self.history_labels_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn cancel_selection_path(&mut self) {
        self.clear_selection_path();
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
        self.clear_selection_path();
        let __gen = self.engine.bump_document_generation();
        self.engine.history.push_selection("Deselect", __gen);
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
        let __gen = self.engine.bump_document_generation();
        self.engine.history.push_selection("Select all", __gen);
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
        let __gen = self.engine.bump_document_generation();
        self.engine
            .history
            .push_selection("Invert selection", __gen);
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
                {
                    graph.bump_generation();
                    graph.generation
                },
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
                {
                    graph.bump_generation();
                    graph.generation
                },
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

    /// Bake active text layer to raster pixels (CPU glyph bake → GPU upload).
    #[qslot]
    fn bake_text_layer(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let Some(layer) = graph.get(id) else {
            return;
        };
        if layer.kind != LayerKind::Text {
            self.status_text = "Bake Text requires an active text layer".to_owned();
            self.status_text_changed();
            return;
        }
        let Some(content) = layer.text.clone() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let pixels = match bake_text_rgba8(&content, w, h) {
            Ok(p) => p,
            Err(error) => {
                self.report_gpu("bake text", &error);
                return;
            }
        };
        if let Some(layer) = self.engine.graph.as_mut().and_then(|g| g.get_mut(id)) {
            layer.kind = LayerKind::Raster;
            layer.text = None;
            if layer.asset_key.is_none() {
                layer.asset_key = Some(format!("layer-{}", id.0));
            }
        }
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
            self.report_gpu("bake text upload", &error);
            return;
        }
        let _ = self.engine.bump_document_generation();
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.status_text = "Text baked to pixels".to_owned();
        self.status_text_changed();
    }

    /// Morphological / feather modify of the live selection mask (`op`: feather|expand|contract).
    #[qslot]
    fn modify_selection(&mut self, op: String, radius: i32) {
        if radius < 0 {
            return;
        }
        let Ok(mask) = phototux_canvas::selection_snapshot() else {
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let radius = radius as u32;
        let next = match op.as_str() {
            "feather" => feather_mask_r8(w, h, &mask, radius),
            "expand" => expand_mask_r8(w, h, &mask, radius),
            "contract" => contract_mask_r8(w, h, &mask, radius),
            _ => {
                self.status_text = format!("Unknown selection op: {op}");
                self.status_text_changed();
                return;
            }
        };
        match next {
            Ok(bytes) => {
                if let Err(error) = phototux_canvas::selection_restore(&bytes) {
                    self.report_gpu("modify selection", &error);
                    return;
                }
                self.engine.selection.feather = if op == "feather" {
                    radius as f32
                } else {
                    self.engine.selection.feather
                };
                self.sync_from_engine();
                self.status_text = format!("Selection {op} ({radius}px)");
                self.status_text_changed();
            }
            Err(error) => self.report_gpu("modify selection", &error),
        }
    }

    #[qslot]
    fn add_drop_shadow_style(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(layer) = self.engine.graph.as_mut().and_then(|g| g.get_mut(id)) else {
            return;
        };
        if layer.kind != LayerKind::Raster {
            self.status_text = "Drop Shadow requires a raster layer".to_owned();
            self.status_text_changed();
            return;
        }
        layer.styles.push(LayerStyle::drop_shadow_default());
        let _ = self.engine.bump_document_generation();
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.status_text = "Drop Shadow style added".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn add_stroke_style(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(layer) = self.engine.graph.as_mut().and_then(|g| g.get_mut(id)) else {
            return;
        };
        if layer.kind != LayerKind::Raster {
            self.status_text = "Stroke style requires a raster layer".to_owned();
            self.status_text_changed();
            return;
        }
        layer.styles.push(LayerStyle::stroke_default());
        let _ = self.engine.bump_document_generation();
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.status_text = "Stroke style added".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn stroke_active_path_to_layer(&mut self) {
        let Some(graph) = self.engine.graph.as_mut() else {
            return;
        };
        if graph.paths.paths.is_empty() {
            let w = graph.size.width as f32;
            let h = graph.size.height as f32;
            graph.paths.add(VectorPath::polyline(
                "Path 1",
                vec![
                    PathPoint {
                        x: w * 0.2,
                        y: h * 0.3,
                    },
                    PathPoint {
                        x: w * 0.8,
                        y: h * 0.7,
                    },
                ],
                false,
            ));
        }
        let idx = graph.paths.active.unwrap_or(0);
        let Some(path) = graph.paths.paths.get(idx).cloned() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let pixels = match stroke_path_rgba8(w, h, &path, [0, 0, 0, 255], 2.0) {
            Ok(p) => p,
            Err(error) => {
                self.report_gpu("stroke path", &error);
                return;
            }
        };
        let layer_name = path.name.clone();
        let result = {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            undo_actions::add_layer(graph, history, Some(layer_name))
        };
        match result {
            Ok(id) => {
                if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
                    self.report_gpu("stroke path upload", &error);
                    return;
                }
                self.recomposite();
                self.mark_dirty();
                self.sync_from_engine();
                self.emit_layer_fields();
                self.status_text = "Path stroked to new layer".to_owned();
                self.status_text_changed();
            }
            Err(error) => self.report_gpu("stroke path", &error.to_string()),
        }
    }

    #[qslot]
    fn add_adjustment_layer(&mut self, kind: String) {
        let (name, params) = match kind.as_str() {
            "levels" => (
                "Levels",
                AdjustmentParams::Levels {
                    black: 0.0,
                    white: 1.0,
                    gamma: 1.0,
                },
            ),
            "invert" => ("Invert", AdjustmentParams::Invert),
            "threshold" => ("Threshold", AdjustmentParams::Threshold { level: 0.5 }),
            "posterize" => ("Posterize", AdjustmentParams::Posterize { levels: 8 }),
            "hue" => (
                "Hue/Saturation",
                AdjustmentParams::HueSaturation {
                    hue: 0.0,
                    saturation: 0.0,
                    lightness: 0.0,
                },
            ),
            _ => (
                "Brightness/Contrast",
                AdjustmentParams::BrightnessContrast {
                    brightness: 0.0,
                    contrast: 0.0,
                },
            ),
        };
        let result = (|| {
            let SessionState { graph, history, .. } = &mut self.engine;
            let graph = graph.as_mut().ok_or(DocumentError::NoDocument)?;
            let id = graph.add_adjustment_top(Some(name.into()), params)?;
            let index = graph.index_of(id).unwrap_or(0);
            let layer = graph
                .get(id)
                .cloned()
                .ok_or(DocumentError::LayerMissingAfterAdd)?;
            history.push_graph_applied(
                phototux_engine::GraphCommand::AddLayer { id, index, layer },
                "Add adjustment",
                {
                    graph.bump_generation();
                    graph.generation
                },
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
    fn set_adjustment_params(&mut self, p0: f32, p1: f32, p2: f32) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(prev) = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .and_then(|l| l.adjustment.clone())
        else {
            return;
        };
        let next = match &prev {
            AdjustmentParams::BrightnessContrast { .. } => AdjustmentParams::BrightnessContrast {
                brightness: p0,
                contrast: p1,
            },
            AdjustmentParams::Levels { .. } => AdjustmentParams::Levels {
                black: p0,
                white: p1,
                gamma: p2,
            },
            other => other.clone(),
        };
        let next = next.clamped();
        if next == prev {
            return;
        }
        {
            let SessionState { graph, history, .. } = &mut self.engine;
            let Some(graph) = graph.as_mut() else {
                return;
            };
            if graph.set_adjustment(id, Some(next.clone())).is_none() {
                return;
            }
            history.push_graph_applied(
                phototux_engine::GraphCommand::SetAdjustment {
                    id,
                    prev: Some(prev),
                    next: Some(next),
                },
                "Adjustment",
                {
                    graph.bump_generation();
                    graph.generation
                },
            );
        }
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn add_gaussian_blur(&mut self) {
        self.add_named_filter("gaussian");
    }

    #[qslot]
    fn add_motion_blur(&mut self) {
        self.add_named_filter("motion");
    }

    #[qslot]
    fn add_emboss_filter(&mut self) {
        self.add_named_filter("emboss");
    }

    fn add_named_filter(&mut self, kind: &str) {
        let Some(id) = self.active_id() else {
            return;
        };
        let is_raster = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .is_some_and(|l| l.kind == LayerKind::Raster);
        if !is_raster {
            self.status_text = format!("{kind} filter requires an active raster layer");
            self.status_text_changed();
            return;
        }
        let Some((prev, _)) = (match kind {
            "gaussian" => self
                .engine
                .graph
                .as_mut()
                .and_then(|g| g.add_gaussian_blur(id, 4.0)),
            "motion" => self
                .engine
                .graph
                .as_mut()
                .and_then(|g| g.add_motion_blur(id, 8.0, 0.0)),
            "emboss" => self
                .engine
                .graph
                .as_mut()
                .and_then(|g| g.add_emboss(id, 1.0, 135.0)),
            _ => None,
        }) else {
            return;
        };
        let next = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.effects.clone())
            .unwrap_or_default();
        let label = match kind {
            "gaussian" => "Gaussian Blur",
            "motion" => "Motion Blur",
            "emboss" => "Emboss",
            other => other,
        };
        {
            let generation = self.engine.bump_document_generation();
            self.engine.history.push_graph_applied(
                phototux_engine::GraphCommand::SetEffects { id, prev, next },
                label,
                generation,
            );
        }
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn set_gaussian_radius(&mut self, radius: f32) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(prev) = self
            .engine
            .graph
            .as_mut()
            .and_then(|g| g.set_gaussian_radius(id, radius))
        else {
            return;
        };
        let next = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.effects.clone())
            .unwrap_or_default();
        if next == prev {
            return;
        }
        {
            let generation = self.engine.bump_document_generation();
            self.engine.history.push_graph_applied(
                phototux_engine::GraphCommand::SetEffects { id, prev, next },
                "Blur radius",
                generation,
            );
        }
        self.recomposite();
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
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
                {
                    graph.bump_generation();
                    graph.generation
                },
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
                {
                    graph.bump_generation();
                    graph.generation
                },
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
                    {
                        graph.bump_generation();
                        graph.generation
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
                    {
                        graph.bump_generation();
                        graph.generation
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
        self.prefs.show_guides = visible;
        self.pref_show_guides = visible;
        self.persist_prefs();
        self.emit_guides_fields();
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
    }

    #[qslot]
    fn set_grid_visible(&mut self, visible: bool) {
        self.engine.guides.show_grid = visible;
        self.prefs.show_grid = visible;
        self.pref_show_grid = visible;
        self.persist_prefs();
        self.emit_guides_fields();
    }

    #[qslot]
    fn set_rulers_visible(&mut self, visible: bool) {
        self.engine.guides.show_rulers = visible;
        self.prefs.show_rulers = visible;
        self.pref_show_rulers = visible;
        self.persist_prefs();
        self.emit_guides_fields();
    }

    #[qslot]
    fn set_snap_enabled(&mut self, enabled: bool) {
        self.engine.guides.snap = enabled;
        self.prefs.snap_enabled = enabled;
        self.pref_snap = enabled;
        self.persist_prefs();
        self.emit_guides_fields();
    }

    #[qslot]
    fn set_grid_spacing(&mut self, spacing: f32) {
        self.engine.guides.grid_spacing = spacing.clamp(4.0, 512.0);
        self.grid_spacing = self.engine.guides.grid_spacing;
        self.grid_spacing_changed();
    }

    #[qslot]
    fn add_guide(&mut self, orientation: String, position: f32) {
        let Some(orient) = GuideOrientation::parse(&orientation) else {
            self.status_text = format!("Unknown guide orientation: {orientation}");
            self.status_text_changed();
            return;
        };
        let position = self.engine.guides.snap_value(position, orient);
        self.engine.guides.add_guide(Guide {
            orientation: orient,
            position,
        });
        self.sync_guides_fields();
        self.emit_guides_fields();
        self.status_text = format!("Guide added at {position:.0}px");
        self.status_text_changed();
    }

    #[qslot]
    fn clear_guides(&mut self) {
        self.engine.guides.clear_guides();
        self.sync_guides_fields();
        self.emit_guides_fields();
        self.status_text = "Guides cleared".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn snap_document_value(&mut self, value: f32, orientation: String) -> f32 {
        let orient = GuideOrientation::parse(&orientation).unwrap_or(GuideOrientation::Vertical);
        self.engine.guides.snap_value(value, orient)
    }

    #[qslot]
    fn update_active_text(
        &mut self,
        body: String,
        font_family: String,
        font_size: f32,
        tracking: f32,
        line_spacing: f32,
        alignment: i32,
        color_hex: String,
    ) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(layer) = self.engine.graph.as_mut().and_then(|g| g.get_mut(id)) else {
            return;
        };
        if layer.kind != LayerKind::Text {
            return;
        }
        let rgba =
            phototux_engine::ColorState::from_hex(&color_hex).unwrap_or([0.0, 0.0, 0.0, 1.0]);
        layer.text = Some(TextContent {
            text: body,
            font_family,
            font_size_pt: font_size.clamp(4.0, 512.0),
            color_rgba: rgba,
            alignment: u8::try_from(alignment.clamp(0, 2)).unwrap_or(0),
            tracking,
            line_spacing: line_spacing.clamp(0.5, 4.0),
        });
        let _ = self.engine.bump_document_generation();
        self.mark_dirty();
        self.sync_text_fields();
        self.emit_text_fields();
    }

    #[qslot]
    fn swap_fg_bg(&mut self) {
        self.engine.colors.swap();
        let fg = self.engine.colors.foreground;
        // Sync brush without re-pushing recent (swap already has both colors).
        self.engine.brush_color = fg;
        self.engine.brush.color = fg;
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.emit_color_fields();
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
                let __gen = self.engine.bump_document_generation();
                self.engine.history.push_transform("Free Transform", __gen);
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
                let __gen = self.engine.bump_document_generation();
                self.engine.history.push_transform("Crop", __gen);
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
                let __gen = self.engine.bump_document_generation();
                self.engine.history.push_transform(
                    if horizontal {
                        "Flip Horizontal"
                    } else {
                        "Flip Vertical"
                    },
                    __gen,
                );
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
                let __gen = self.engine.bump_document_generation();
                self.engine.history.push_transform("Rotate 90° CW", __gen);
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

    fn active_raster_paintable(&self) -> Option<LayerId> {
        let id = self.active_id()?;
        let layer = self.engine.graph.as_ref()?.get(id)?;
        if layer.kind != phototux_engine::LayerKind::Raster || layer.paint_blocked() {
            return None;
        }
        Some(id)
    }

    #[qslot]
    fn fill_active_layer(&mut self) {
        let Some(id) = self.active_raster_paintable() else {
            self.status_text = "Fill requires an unlocked raster layer".to_owned();
            self.status_text_changed();
            return;
        };
        let fg = self.engine.colors.foreground;
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        let use_selection = self.engine.selection.active;
        match phototux_canvas::fill_layer(id, fg, &layers, use_selection) {
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
                let __gen = self.engine.bump_document_generation();
                self.engine.history.push_transform("Fill", __gen);
                self.mark_dirty();
            }
            Err(error) => self.report_gpu("fill", &error),
        }
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn commit_linear_gradient(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let Some(id) = self.active_raster_paintable() else {
            self.status_text = "Gradient requires an unlocked raster layer".to_owned();
            self.status_text_changed();
            return;
        };
        let c0 = self.engine.colors.foreground;
        let c1 = self.engine.colors.background;
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        let use_selection = self.engine.selection.active;
        match phototux_canvas::apply_linear_gradient(
            id,
            [x0, y0],
            [x1, y1],
            c0,
            c1,
            &layers,
            use_selection,
        ) {
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
                let __gen = self.engine.bump_document_generation();
                self.engine.history.push_transform("Gradient", __gen);
                self.mark_dirty();
            }
            Err(error) => self.report_gpu("gradient", &error),
        }
        self.sync_from_engine();
        self.emit_layer_fields();
    }

    #[qslot]
    fn sample_color_at(&mut self, doc_x: f32, doc_y: f32) {
        let x = doc_x.round() as i32;
        let y = doc_y.round() as i32;
        let rgb = match self.engine.colors.sample_source {
            phototux_engine::SampleSource::CurrentLayer => {
                let Some(id) = self.active_id() else {
                    return;
                };
                phototux_canvas::sample_layer_at(id, x, y)
            }
            phototux_engine::SampleSource::AllLayers => phototux_canvas::sample_composite_at(x, y),
        };
        match rgb {
            Ok([r, g, b]) => {
                self.set_brush_color(r, g, b);
                self.status_text = format!(
                    "Sampled {}",
                    phototux_engine::ColorState::to_hex([r, g, b, 1.0])
                );
                self.status_text_changed();
            }
            Err(error) => self.report_gpu("eyedropper", &error),
        }
    }
}
