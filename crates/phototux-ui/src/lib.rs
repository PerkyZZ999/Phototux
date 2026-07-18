//! QML-facing session via qtbridge (ADR-003). Package name `phototux_ui` → `import phototux_ui`.

mod display_icc;
mod file_worker;
mod fonts;
mod prefs;

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

use file_worker::{FileCommand, FileEvent, FileWorker};
use phototux_canvas::PaintWorker;
use phototux_engine::{
    AdjustmentParams, CommandArgs, CommandEffects, CommandError, CropRect, DocumentGraph,
    DocumentRegistry, DocumentSize, EngineCommand, EngineEvent, FilterParams, Guide,
    GuideOrientation, HistoryKind, HostFollowUp, HostHistoryAction, LayerId, LayerKind,
    LayerTransform, OpenDocumentId, PathPoint, SelectionCombine, SelectionRect, SelectionShape,
    SelectionState, SessionState, ShapeBooleanPartner, ShapeContent, ShapeGradient, TextContent,
    TransformSession, VectorPath, WorkspaceState, bake_text_rgba8, command_id, contract_mask_r8,
    ellipse_path, expand_mask_r8, feather_mask_r8, fill_gradient_even_odd, polygon_path,
    rasterize_shape_rgba8, rect_path, stroke_path_rgba8, tool_id,
};
use prefs::Preferences;

fn parse_modify_arg(arg: &str) -> (String, i32) {
    let mut parts = arg.split(':');
    let op = parts.next().unwrap_or("feather").to_owned();
    let radius = parts
        .next()
        .and_then(|r| r.parse::<i32>().ok())
        .unwrap_or(4);
    (op, radius)
}

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
use phototux_io::{
    PtxDocument, RasterFormat, discard_recovery, format_report, list_recoverable, load_recovery,
    write_stroke_journal,
};
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

fn r8_to_gray_rgba(r8: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(r8.len().saturating_mul(4));
    for &v in r8 {
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    rgba
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
    brush_texture_strength: f32,
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
    layer_selection: String,
    mask_edit_active: bool,
    mask_density: f32,
    mask_feather: f32,
    mask_inverted: bool,
    mask_linked: bool,
    mask_contrast: f32,
    mask_shift: f32,
    /// JSON `[x,y,w,h]` doc-space dirty rect, or empty when view-invalidated / none.
    dirty_rect_json: String,
    overlay_view_generation: i32,
    /// Distinct from focus/object selection: pixel selection channel active.
    pixel_selection_active: bool,
    /// Object-selection labels (layer names), distinct from pixel selection (DR-011).
    object_selection_label: String,
    /// Last polite announce string for status / a11y.
    last_announce: String,
    /// `layer` or `mask` (PaintTarget).
    edit_target: String,
    edit_target_label: String,
    active_layer_kind: String,
    history_labels: String,
    history_entry_ids: String,
    history_kinds: String,
    brush_preset_names: String,
    soft_proof_profile: String,
    soft_proof_active: bool,
    has_embedded_icc: bool,
    /// Host display ICC name (colord / env / xdg / sRGB tag).
    display_profile_name: String,
    /// Soft-proof tag for "use display profile" (`display:…`).
    display_profile_tag: String,
    /// Runtime GPU device/surface loss; document graph remains authoritative.
    gpu_lost: bool,
    accessibility_tree_json: String,
    /// AT-SPI host projection of the semantic tree (role/state mapping).
    atspi_projection_json: String,
    /// JSON array of [`phototux_io::RecoveryEntry`] for the restore chooser.
    recovery_entries_json: String,
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
    /// Multi-select: opacity values disagree.
    inspector_opacity_mixed: bool,
    /// Multi-select: blend modes disagree.
    inspector_blend_mixed: bool,
    /// Progressive disclosure for advanced Properties section.
    properties_advanced_open: bool,
    /// JSON map of preference key → winning source (builtin/user/workspace/document).
    pref_effective_json: String,
    pref_safe_start_next: bool,
    pref_history_retention: i32,
    foreground_hex: String,
    background_hex: String,
    fill_color_hex: String,
    recent_colors: String,
    viewport_width: f32,
    viewport_height: f32,
    adjustment_kind: String,
    adjustment_p0: f32,
    adjustment_p1: f32,
    adjustment_p2: f32,
    has_gaussian_blur: bool,
    gaussian_radius: f32,
    effects_joined: String,
    icon_root: String,
    document_name: String,
    dirty: bool,
    io_busy: bool,
    io_error: String,
    startup_ms: f32,
    engine: SessionState,
    /// Inactive open documents (active session is [`Self::engine`]).
    doc_registry: DocumentRegistry,
    active_doc_id: Option<OpenDocumentId>,
    document_tabs_json: String,
    worker: PaintWorker,
    file_worker: FileWorker,
    /// RGBA image clipboard (app-local; may also push OS image).
    clipboard_rgba: Option<(u32, u32, Vec<u8>)>,
    /// Selection coverage clipboard (R8, document-sized).
    clipboard_selection_r8: Option<(u32, u32, Vec<u8>)>,
    /// Layer mask clipboard (R8, document-sized).
    clipboard_mask_r8: Option<(u32, u32, Vec<u8>)>,
    selection_undo: Vec<SelectionSnapshot>,
    selection_redo: Vec<SelectionSnapshot>,
    transform_undo: Vec<TransformSnapshot>,
    transform_redo: Vec<TransformSnapshot>,
    /// Generation pinned when a Save was submitted (Phase 2 receipt).
    pending_save_generation: Option<u64>,
    prefs: Preferences,
    workspace: WorkspaceState,
    panel_descriptors_json: String,
    workspace_presets_json: String,
    workspace_focus_json: String,
    active_workspace_preset_id: String,
    dock_topology_json: String,
    panel_visibility_json: String,
    tool_descriptors_json: String,
    actions_json: String,
    shortcuts_json: String,
    action_shortcuts_json: String,
    /// When true, global shortcut resolve yields (text fields / IME).
    shortcut_input_yield: bool,
    preferences_open: bool,
    filter_gallery_open: bool,
    filter_preview_kind: String,
    filter_preview_p0: f32,
    filter_preview_p1: f32,
    filter_preview_p2: f32,
    filter_preview_active: bool,
    path_closed: bool,
    path_anchor_count: i32,
    path_edit_selected: i32,
    text_frame_w: f32,
    text_frame_h: f32,
    text_wrap: bool,
    pref_show_guides: bool,
    pref_restore_last_tool: bool,
    pref_ui_density: String,
    pref_high_contrast: bool,
    pref_reduced_motion: bool,
    panel_navigator_visible: bool,
    panel_swatches_visible: bool,
    panel_layers_visible: bool,
    panel_history_visible: bool,
    panel_properties_visible: bool,
    text_layer_active: bool,
    text_body: String,
    text_font_family: String,
    /// JSON array of discovered font family names (`fc-list`).
    available_fonts_json: String,
    /// Active text layer origin in document pixels (from layer transform).
    text_origin_x: f32,
    text_origin_y: f32,
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
            if let Err(error) = session.file_worker.send(command) {
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
            brush_texture_strength: engine.brush.texture_strength,
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
            layer_selection: String::new(),
            mask_edit_active: false,
            mask_density: 1.0,
            mask_feather: 0.0,
            mask_inverted: false,
            mask_linked: true,
            mask_contrast: 0.0,
            mask_shift: 0.0,
            dirty_rect_json: String::new(),
            overlay_view_generation: 0,
            pixel_selection_active: false,
            object_selection_label: String::new(),
            last_announce: String::new(),
            edit_target: "layer".to_owned(),
            edit_target_label: "Layer pixels".to_owned(),
            active_layer_kind: String::new(),
            history_labels: String::new(),
            history_entry_ids: String::new(),
            history_kinds: String::new(),
            brush_preset_names: String::new(),
            soft_proof_profile: String::new(),
            soft_proof_active: false,
            has_embedded_icc: false,
            display_profile_name: String::new(),
            display_profile_tag: String::new(),
            gpu_lost: false,
            accessibility_tree_json: "[]".into(),
            atspi_projection_json: "[]".into(),
            recovery_entries_json: "[]".into(),
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
            inspector_opacity_mixed: false,
            inspector_blend_mixed: false,
            properties_advanced_open: false,
            pref_effective_json: String::new(),
            pref_safe_start_next: false,
            pref_history_retention: 128,
            foreground_hex: "#000000".to_owned(),
            background_hex: "#FFFFFF".to_owned(),
            fill_color_hex: "#738CBF".to_owned(),
            recent_colors: String::new(),
            viewport_width: 1.0,
            viewport_height: 1.0,
            adjustment_kind: String::new(),
            adjustment_p0: 0.0,
            adjustment_p1: 0.0,
            adjustment_p2: 1.0,
            has_gaussian_blur: false,
            gaussian_radius: 0.0,
            effects_joined: String::new(),
            icon_root,
            document_name: "Untitled".to_owned(),
            dirty: false,
            io_busy: false,
            io_error: String::new(),
            startup_ms: 0.0,
            engine,
            doc_registry: DocumentRegistry::new(),
            active_doc_id: None,
            document_tabs_json: "[]".into(),
            worker: PaintWorker::start(),
            file_worker: FileWorker::start(),
            clipboard_rgba: None,
            clipboard_selection_r8: None,
            clipboard_mask_r8: None,
            selection_undo: Vec::new(),
            selection_redo: Vec::new(),
            transform_undo: Vec::new(),
            transform_redo: Vec::new(),
            pending_save_generation: None,
            prefs: Preferences::default(),
            workspace: WorkspaceState::essentials(),
            panel_descriptors_json: phototux_engine::panels_json(),
            workspace_presets_json: phototux_engine::workspace_presets_json(),
            workspace_focus_json:
                serde_json::to_string(&phototux_engine::WorkspaceFocus::default())
                    .unwrap_or_else(|_| "{}".into()),
            active_workspace_preset_id: "workspace.preset.essentials".into(),
            dock_topology_json: phototux_engine::DockTopology::essentials()
                .to_json()
                .unwrap_or_else(|_| "{}".into()),
            panel_visibility_json: WorkspaceState::essentials().visibility_json(),
            tool_descriptors_json: phototux_engine::tools_json(),
            actions_json: phototux_engine::actions_json(),
            shortcuts_json: phototux_engine::shortcuts_json(),
            action_shortcuts_json: phototux_engine::action_shortcuts_json(),
            shortcut_input_yield: false,
            preferences_open: false,
            filter_gallery_open: false,
            filter_preview_kind: "gaussian".into(),
            filter_preview_p0: 4.0,
            filter_preview_p1: 0.0,
            filter_preview_p2: 0.0,
            filter_preview_active: false,
            path_closed: false,
            path_anchor_count: 0,
            path_edit_selected: -1,
            text_frame_w: 0.0,
            text_frame_h: 0.0,
            text_wrap: false,
            pref_show_guides: true,
            pref_restore_last_tool: false,
            pref_ui_density: "dense".into(),
            pref_high_contrast: false,
            pref_reduced_motion: false,
            panel_navigator_visible: true,
            panel_swatches_visible: true,
            panel_layers_visible: true,
            panel_history_visible: true,
            panel_properties_visible: true,
            text_layer_active: false,
            text_body: String::new(),
            text_font_family: "Noto Sans".into(),
            available_fonts_json: fonts::discover_font_families_json(),
            text_origin_x: 0.0,
            text_origin_y: 0.0,
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
        // Field-only init: qtbridge attaches the QObject proxy *after* Default returns.
        // Emitting `*_changed()` here panics with "No proxy" and aborts the process.
        out.apply_loaded_preferences();
        out.sync_recovery_list_fields();
        let display = display_icc::discover_display_profile();
        out.display_profile_tag = display.soft_proof_tag();
        out.display_profile_name = display.name;
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

    fn apply_opened_ptx(&mut self, path: PathBuf, document: PtxDocument) {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Document.ptx".into());
        if let Err(error) = self.prepare_new_document_tab(&title) {
            self.fail_io("Open", &error);
            return;
        }
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
                        return;
                    }
                }
                for (id, mask) in masks {
                    let r8: Vec<u8> = mask.pixels().chunks_exact(4).map(|rgba| rgba[0]).collect();
                    if let Err(error) = phototux_canvas::write_mask_r8(LayerId(id), &r8) {
                        self.fail_io("Open", &error);
                        return;
                    }
                }
                self.engine.replace_graph(graph);
                self.engine.document_path = Some(path.display().to_string());
                self.engine.set_composite_ms(ms);
                self.document_name = title;
                self.dirty = true;
                self.io_busy = false;
                self.compatibility_report.clear();
                self.publish_pixel_snapshot_from_gpu();
                self.sync_from_engine();
                self.emit_doc_fields();
                self.compatibility_report_changed();
                self.refresh_document_tabs_json();
                if let Ok(export_path) = std::env::var("PHOTOTUX_DESKTOP_EXPORT") {
                    let url = if export_path.starts_with("file:") {
                        export_path
                    } else {
                        format!("file://{export_path}")
                    };
                    self.export_raster_file(url);
                }
            }
            Err(error) => self.fail_io("Open", &error),
        }
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
        let lower = error.to_ascii_lowercase();
        if lower.contains("device lost")
            || lower.contains("surface lost")
            || phototux_canvas::gpu_is_lost()
        {
            self.enter_gpu_lost();
            return;
        }
        self.status_text = format!("{operation} failed: {error}");
        self.status_text_changed();
        eprintln!("[phototux] {operation}: {error}");
    }

    fn enter_gpu_lost(&mut self) {
        if !self.gpu_lost {
            self.gpu_lost = true;
            self.gpu_lost_changed();
        }
        self.engine
            .announce("Graphics device lost — document preserved");
        self.status_text = "Graphics device lost — document preserved".into();
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
        eprintln!("[phototux] GPU lost — document authority preserved");
    }

    fn sync_from_engine(&mut self) {
        self.doc_width = self.engine.size.width as i32;
        self.doc_height = self.engine.size.height as i32;
        self.zoom = self.engine.camera.zoom;
        self.pan_x = self.engine.camera.pan_x;
        self.pan_y = self.engine.camera.pan_y;
        self.brush_size = self.engine.brush_size;
        self.brush_hardness = self.engine.brush_hardness;
        self.brush_texture_strength = self.engine.brush.texture_strength;
        self.brush_r = self.engine.brush_color[0];
        self.brush_g = self.engine.brush_color[1];
        self.brush_b = self.engine.brush_color[2];
        self.fps = self.engine.fps;
        self.composite_ms = self.engine.composite_ms;
        self.stroke_latency_ms = self.engine.stroke_latency_ms;
        self.dirty_rect_json = match self.engine.dirty_rect {
            Some([x, y, w, h]) => format!("[{x},{y},{w},{h}]"),
            None => String::new(),
        };
        self.overlay_view_generation =
            i32::try_from(self.engine.overlay_view_generation.min(i32::MAX as u64)).unwrap_or(0);
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
        self.layer_selection = self.engine.layer_selection_joined();
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
        let mask = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .and_then(|l| l.mask.as_ref());
        if let Some(m) = mask {
            self.mask_density = m.density;
            self.mask_feather = m.feather;
            self.mask_inverted = m.inverted;
            self.mask_linked = m.linked;
            self.mask_contrast = m.contrast;
            self.mask_shift = m.shift;
        } else {
            self.mask_density = 1.0;
            self.mask_feather = 0.0;
            self.mask_inverted = false;
            self.mask_linked = true;
            self.mask_contrast = 0.0;
            self.mask_shift = 0.0;
        }
        self.edit_target = self.engine.edit_target_id().to_owned();
        self.edit_target_label = self.engine.edit_target_label().to_owned();
        let idx = self.active_layer_index;
        self.active_layer_kind = if idx >= 0 {
            self.layer_kinds
                .split('|')
                .nth(idx as usize)
                .unwrap_or("")
                .to_owned()
        } else {
            String::new()
        };
        self.history_labels = self.engine.history_labels_joined();
        self.history_entry_ids = self
            .engine
            .history
            .entry_ids_newest_first()
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("|");
        self.history_kinds = self.engine.history.kinds_newest_first().join("|");
        self.brush_preset_names = self.engine.brush_presets.names_joined();
        if let Some(graph) = self.engine.graph.as_ref() {
            self.soft_proof_profile = graph.color.soft_proof_profile.clone();
            self.soft_proof_active = graph.color.soft_proof_active();
            self.has_embedded_icc = graph.color.has_embedded_icc();
        } else {
            self.soft_proof_profile.clear();
            self.soft_proof_active = false;
            self.has_embedded_icc = false;
        }
        let accessibility_tree_json = self.build_accessibility_tree_json();
        if accessibility_tree_json != self.accessibility_tree_json {
            self.accessibility_tree_json = accessibility_tree_json;
            self.atspi_projection_json =
                phototux_engine::project_semantic_tree_json(&self.accessibility_tree_json);
        }
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
        self.fill_color_hex = active_layer
            .and_then(|l| l.fill.as_ref())
            .map(|f| phototux_engine::ColorState::to_hex(f.color_rgba))
            .unwrap_or_else(|| "#738CBF".to_owned());
        self.sync_inspector_mixed_fields();
        self.sync_color_fields();
        self.viewport_width = self.engine.viewport_w;
        self.viewport_height = self.engine.viewport_h;
        self.sync_adjustment_fields();
        self.sync_text_fields();
        self.sync_path_edit_fields();
        self.sync_filter_preview_fields();
        self.sync_guides_fields();
        self.refresh_pref_effective_json();
        self.pref_effective_json_changed();
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
            Some(AdjustmentParams::Exposure { stops, gamma }) => {
                self.adjustment_kind = "exposure".into();
                self.adjustment_p0 = *stops;
                self.adjustment_p1 = *gamma;
                self.adjustment_p2 = 0.0;
            }
            Some(AdjustmentParams::HueSaturation {
                hue,
                saturation,
                lightness,
            }) => {
                self.adjustment_kind = "hue".into();
                self.adjustment_p0 = *hue;
                self.adjustment_p1 = *saturation;
                self.adjustment_p2 = *lightness;
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
        self.effects_joined = self.engine.active_effects_joined();
    }

    fn sync_text_fields(&mut self) {
        let layer = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)));
        let Some(layer) = layer.filter(|l| l.kind == LayerKind::Text) else {
            self.text_layer_active = false;
            self.text_origin_x = 0.0;
            self.text_origin_y = 0.0;
            return;
        };
        self.text_layer_active = true;
        self.text_origin_x = layer.transform.translate_x;
        self.text_origin_y = layer.transform.translate_y;
        let content = layer.text.clone().unwrap_or_default();
        self.text_body = content.text;
        self.text_font_family = content.font_family;
        self.text_font_size = content.font_size_pt;
        self.text_tracking = content.tracking;
        self.text_line_spacing = content.line_spacing;
        self.text_alignment = i32::from(content.alignment);
        self.text_color_hex = phototux_engine::ColorState::to_hex(content.color_rgba);
        self.text_frame_w = content.frame_w;
        self.text_frame_h = content.frame_h;
        self.text_wrap = content.wrap;
    }

    fn sync_path_edit_fields(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            self.path_closed = false;
            self.path_anchor_count = 0;
            self.path_edit_selected = -1;
            return;
        };
        let path = graph.active_id().and_then(|id| {
            let layer = graph.get(id)?;
            if layer.kind == LayerKind::Shape {
                return layer.shape.as_ref().map(|s| &s.path);
            }
            None
        });
        let path = path.or_else(|| {
            let idx = graph.paths.active?;
            graph.paths.paths.get(idx)
        });
        match path {
            Some(path) => {
                self.path_closed = path.closed;
                self.path_anchor_count = i32::try_from(path.anchors.len()).unwrap_or(i32::MAX);
                self.path_edit_selected = self
                    .engine
                    .path_edit_anchor
                    .and_then(|i| i32::try_from(i).ok())
                    .unwrap_or(-1);
            }
            None => {
                self.path_closed = false;
                self.path_anchor_count = 0;
                self.path_edit_selected = -1;
            }
        }
    }

    fn sync_filter_preview_fields(&mut self) {
        match &self.engine.filter_preview {
            Some(preview) => {
                self.filter_preview_active = true;
                self.filter_preview_kind = preview.kind.clone();
                self.filter_preview_p0 = preview.p0;
                self.filter_preview_p1 = preview.p1;
                self.filter_preview_p2 = preview.p2;
            }
            None => {
                self.filter_preview_active = false;
            }
        }
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
        self.text_frame_w_changed();
        self.text_frame_h_changed();
        self.text_wrap_changed();
        self.text_origin_x_changed();
        self.text_origin_y_changed();
    }

    fn emit_path_edit_fields(&mut self) {
        self.path_closed_changed();
        self.path_anchor_count_changed();
        self.path_edit_selected_changed();
    }

    fn emit_filter_preview_fields(&mut self) {
        self.filter_preview_active_changed();
        self.filter_preview_kind_changed();
        self.filter_preview_p0_changed();
        self.filter_preview_p1_changed();
        self.filter_preview_p2_changed();
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

    fn sync_inspector_mixed_fields(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            self.inspector_opacity_mixed = false;
            self.inspector_blend_mixed = false;
            return;
        };
        let ids = if self.engine.selected_layer_ids.is_empty() {
            graph.active_id().into_iter().collect::<Vec<_>>()
        } else {
            self.engine.selected_layer_ids.clone()
        };
        let opacities: Vec<f32> = ids
            .iter()
            .filter_map(|id| graph.get(*id).map(|l| l.opacity))
            .collect();
        let blends: Vec<&str> = ids
            .iter()
            .filter_map(|id| graph.get(*id).map(|l| l.blend.as_str()))
            .collect();
        self.inspector_opacity_mixed = phototux_engine::values_are_mixed(&opacities);
        self.inspector_blend_mixed = phototux_engine::values_are_mixed(&blends);
    }

    fn refresh_pref_effective_json(&mut self) {
        // Document soft-proof profile beats user default (none). Guides: session mirrors user prefs.
        let (soft_proof, soft_src) = phototux_engine::resolve_layered(
            String::new(),
            Some(String::new()),
            None,
            self.engine
                .graph
                .as_ref()
                .map(|g| g.color.soft_proof_profile.clone())
                .filter(|s| !s.is_empty()),
        );
        let (density, dens_src) = phototux_engine::resolve_layered(
            "dense".to_owned(),
            Some(self.prefs.ui_density.clone()),
            None,
            None,
        );
        let (guides, guides_src) =
            phototux_engine::resolve_layered(true, Some(self.prefs.show_guides), None, None);
        let map = serde_json::json!({
            "soft_proof_profile": { "value": soft_proof, "source": soft_src.as_str() },
            "ui_density": { "value": density, "source": dens_src.as_str() },
            "show_guides": { "value": guides, "source": guides_src.as_str() },
        });
        self.pref_effective_json = map.to_string();
    }

    fn emit_layer_fields(&mut self) {
        self.sync_inspector_mixed_fields();
        self.layer_count_changed();
        self.active_layer_index_changed();
        self.can_undo_changed();
        self.can_redo_changed();
        self.layer_names_changed();
        self.layer_visibility_changed();
        self.layer_kinds_changed();
        self.layer_mask_flags_changed();
        self.layer_clips_changed();
        self.layer_selection_changed();
        self.mask_edit_active_changed();
        self.mask_density_changed();
        self.mask_feather_changed();
        self.mask_contrast_changed();
        self.mask_shift_changed();
        self.mask_inverted_changed();
        self.mask_linked_changed();
        self.edit_target_changed();
        self.edit_target_label_changed();
        self.active_layer_kind_changed();
        self.history_labels_changed();
        self.history_entry_ids_changed();
        self.history_kinds_changed();
        self.brush_preset_names_changed();
        self.soft_proof_profile_changed();
        self.inspector_opacity_mixed_changed();
        self.inspector_blend_mixed_changed();
        self.soft_proof_active_changed();
        self.has_embedded_icc_changed();
        self.accessibility_tree_json_changed();
        self.atspi_projection_json_changed();
        self.emit_selection_fields();
        self.emit_transform_fields();
        self.document_path_changed();
        self.graph_revision_changed();
        self.active_opacity_changed();
        self.active_blend_changed();
        self.foreground_hex_changed();
        self.background_hex_changed();
        self.fill_color_hex_changed();
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
        self.effects_joined_changed();
        self.composite_ms_changed();
        self.status_text_changed();
        self.emit_text_fields();
        self.emit_path_edit_fields();
        self.emit_filter_preview_fields();
        self.emit_guides_fields();
        self.dirty_rect_json_changed();
        self.overlay_view_generation_changed();
    }

    fn build_accessibility_tree_json(&self) -> String {
        let mut nodes = Vec::new();
        nodes.push(serde_json::json!({
            "id": "chrome.toolbar",
            "role": "toolbar",
            "name": "Tools",
            "state": { "enabled": true },
        }));
        nodes.push(serde_json::json!({
            "id": "chrome.canvas",
            "role": "image",
            "name": if self.has_document {
                format!("Canvas {}×{}", self.engine.size.width, self.engine.size.height)
            } else {
                "Empty canvas".into()
            },
            "state": {
                "busy": self.io_busy,
                "editTarget": self.edit_target,
                "pixelSelection": self.pixel_selection_active,
                "objectSelection": self.object_selection_label,
            },
        }));
        for panel in phototux_engine::default_panels() {
            nodes.push(serde_json::json!({
                "id": panel.id,
                "role": "panel",
                "name": panel.title,
                "state": {
                    "visible": self.workspace.is_visible(&panel.id),
                    "docked": self.workspace.dock.is_docked(&panel.id),
                },
            }));
        }
        serde_json::to_string(&nodes).unwrap_or_else(|_| "[]".into())
    }

    fn sync_selection_fields(&mut self) {
        self.selection_active = self.engine.selection.active;
        self.pixel_selection_active = self.engine.selection.active;
        self.object_selection_label = self.engine.object_selection_names_joined();
        self.last_announce = self.engine.last_announce.clone();
        self.selection_combine = self.engine.selection.combine.as_str().to_owned();
        self.selection_shape = self.engine.selection.shape.as_str().to_owned();
        if let Some(b) = self.engine.selection.bounds {
            self.selection_x = b.x;
            self.selection_y = b.y;
            self.selection_w = i32::try_from(b.width).unwrap_or(i32::MAX);
            self.selection_h = i32::try_from(b.height).unwrap_or(i32::MAX);
            self.engine
                .mark_dirty_rect(b.x, b.y, self.selection_w, self.selection_h);
        } else {
            self.selection_x = 0;
            self.selection_y = 0;
            self.selection_w = 0;
            self.selection_h = 0;
        }
    }

    fn emit_selection_fields(&mut self) {
        self.selection_active_changed();
        self.pixel_selection_active_changed();
        self.object_selection_label_changed();
        self.last_announce_changed();
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
            self.refresh_document_tabs_json();
        }
    }

    fn apply_loaded_preferences(&mut self) {
        self.prefs = Preferences::load();
        self.workspace = WorkspaceState::from_visibility_map(self.prefs.panel_visibility.clone());
        let _ = self.workspace.set_dock(self.prefs.load_dock_topology());
        self.sync_pref_fields_from_store();
        self.refresh_shortcut_maps();
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

    fn refresh_shortcut_maps(&mut self) {
        let (chords, actions) = phototux_engine::effective_shortcuts_json(&self.prefs.keymap);
        self.shortcuts_json = chords;
        self.action_shortcuts_json = actions;
    }

    fn emit_shortcut_maps(&mut self) {
        self.shortcuts_json_changed();
        self.action_shortcuts_json_changed();
    }

    fn sync_pref_fields_from_store(&mut self) {
        self.pref_show_guides = self.prefs.show_guides;
        self.pref_show_grid = self.prefs.show_grid;
        self.pref_show_rulers = self.prefs.show_rulers;
        self.pref_snap = self.prefs.snap_enabled;
        self.pref_restore_last_tool = self.prefs.restore_last_tool;
        self.pref_ui_density = self.prefs.ui_density.clone();
        self.pref_high_contrast = self.prefs.high_contrast;
        self.pref_reduced_motion = self.prefs.reduced_motion;
        self.pref_safe_start_next = self.prefs.safe_start_next;
        self.prefs.history_retention_limit =
            crate::prefs::clamp_history_retention(self.prefs.history_retention_limit);
        self.pref_history_retention = self.prefs.history_retention_limit as i32;
        self.engine
            .history
            .set_limit(self.prefs.history_retention_limit as usize);
        self.refresh_pref_effective_json();
        if let Ok(lib) =
            phototux_engine::BrushPresetLibrary::from_json(&self.prefs.brush_presets_json)
        {
            self.engine.brush_presets = lib;
        }
        self.sync_panel_visibility_from_workspace();
        self.sync_guides_fields();
    }

    fn sync_panel_visibility_from_workspace(&mut self) {
        self.panel_navigator_visible = self.workspace.is_visible("panel.navigator");
        self.panel_swatches_visible = self.workspace.is_visible("panel.swatches");
        self.panel_layers_visible = self.workspace.is_visible("panel.layers");
        self.panel_history_visible = self.workspace.is_visible("panel.history");
        self.panel_properties_visible = self.workspace.is_visible("panel.properties");
        self.panel_visibility_json = self.workspace.visibility_json();
        self.dock_topology_json = self
            .workspace
            .dock
            .to_json()
            .unwrap_or_else(|_| "{}".into());
        self.workspace_focus_json =
            serde_json::to_string(&self.workspace.focus).unwrap_or_else(|_| "{}".into());
        self.active_workspace_preset_id = self.workspace.active_preset_id.clone();
    }

    fn persist_workspace_visibility(&mut self) {
        self.prefs.apply_workspace(&self.workspace);
        self.sync_panel_visibility_from_workspace();
        self.persist_prefs();
        self.panel_navigator_visible_changed();
        self.panel_swatches_visible_changed();
        self.panel_layers_visible_changed();
        self.panel_history_visible_changed();
        self.panel_properties_visible_changed();
        self.panel_visibility_json_changed();
        self.dock_topology_json_changed();
        self.workspace_focus_json_changed();
        self.active_workspace_preset_id_changed();
    }

    fn emit_pref_fields(&mut self) {
        self.pref_show_guides_changed();
        self.pref_show_grid_changed();
        self.pref_show_rulers_changed();
        self.pref_snap_changed();
        self.pref_restore_last_tool_changed();
        self.pref_ui_density_changed();
        self.pref_high_contrast_changed();
        self.pref_reduced_motion_changed();
        self.pref_safe_start_next_changed();
        self.pref_history_retention_changed();
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

    fn active_layer_has_mask(&self) -> bool {
        let idx = self.active_layer_index;
        if idx < 0 {
            return false;
        }
        self.layer_mask_flags
            .split('|')
            .nth(idx as usize)
            .is_some_and(|f| f != "0" && !f.is_empty())
    }

    fn action_enablement(&self, tag: &str) -> bool {
        let busy = self.io_busy;
        match tag {
            "always" => true,
            "io_idle" => !busy,
            "has_document" => self.has_document && !busy,
            "has_document_io_idle" => self.has_document && !busy,
            "can_undo" => self.can_undo && !busy,
            "can_redo" => self.can_redo && !busy,
            "selection_active" => self.has_document && self.selection_active && !busy,
            "has_mask" => self.has_document && self.active_layer_has_mask() && !busy,
            "no_mask" => self.has_document && !self.active_layer_has_mask() && !busy,
            "has_multiple_layers" => self.has_document && self.layer_count > 1 && !busy,
            _ => self.has_document && !busy,
        }
    }

    fn active_lock_flags(&self) -> phototux_engine::LockFlags {
        self.active_id()
            .and_then(|id| self.engine.graph.as_ref()?.get(id))
            .map(|layer| layer.locks)
            .unwrap_or_default()
    }

    fn command_args_for_action(
        &self,
        command_id: &str,
        arg: Option<&str>,
    ) -> Result<CommandArgs, CommandError> {
        use phototux_engine::command_id as cid;
        match command_id {
            cid::HISTORY_UNDO
            | cid::HISTORY_REDO
            | cid::LAYER_CREATE
            | cid::LAYER_DELETE
            | cid::LAYER_GROUP
            | cid::LAYER_UNGROUP
            | cid::VIEW_ZOOM_TO_FIT
            | cid::STYLE_ADD_DROP_SHADOW
            | cid::STYLE_ADD_STROKE
            | cid::STYLE_ADD_OUTER_GLOW
            | cid::STYLE_ADD_COLOR_OVERLAY
            | cid::MASK_APPLY
            | cid::DOCUMENT_ROTATE_90
            | cid::APP_SHOW_PREFERENCES
            | cid::APP_SHOW_FILTER_GALLERY
            | cid::FILTER_COMMIT
            | cid::FILTER_CANCEL_PREVIEW
            | cid::WORKSPACE_RESET
            | cid::MASK_CREATE_VECTOR
            | cid::SELECTION_TO_MASK
            | cid::MASK_TO_SELECTION => Ok(CommandArgs::None),
            cid::LAYER_CREATE_FILL => Ok(CommandArgs::FillCreate {
                color_rgba: [
                    self.engine.colors.foreground[0],
                    self.engine.colors.foreground[1],
                    self.engine.colors.foreground[2],
                    1.0,
                ],
            }),
            cid::WORKSPACE_TOGGLE_PANEL => Ok(CommandArgs::TogglePanel {
                panel_id: arg.unwrap_or("panel.layers").to_owned(),
            }),
            cid::WORKSPACE_APPLY_PRESET => Ok(CommandArgs::ApplyWorkspacePreset {
                preset_id: arg.unwrap_or("workspace.preset.essentials").to_owned(),
            }),
            cid::DOCUMENT_ASSIGN_PROFILE => Ok(CommandArgs::AssignProfile {
                profile: arg.unwrap_or("sRGB").to_owned(),
            }),
            cid::DOCUMENT_CONVERT_PROFILE => Ok(CommandArgs::ConvertProfile {
                profile: arg.unwrap_or("sRGB").to_owned(),
            }),
            cid::DOCUMENT_SET_SOFT_PROOF => {
                let raw = arg.unwrap_or(":relative");
                let (profile, intent) = match raw.split_once(':') {
                    Some((p, i)) => (p.to_owned(), i.to_owned()),
                    None => (raw.to_owned(), "relative".to_owned()),
                };
                Ok(CommandArgs::SoftProof { profile, intent })
            }
            cid::DOCUMENT_SET_ICC => {
                if arg == Some("clear") {
                    Ok(CommandArgs::SetIcc { bytes: None })
                } else {
                    Err(CommandError::InvalidArgument(
                        "document.set-icc requires clear or host embed",
                    ))
                }
            }
            cid::FILTER_ADD_ADJUSTMENT => Ok(CommandArgs::FilterAdjustment {
                kind: arg.unwrap_or("brightness").to_owned(),
            }),
            cid::FILTER_ADD_EFFECT => Ok(CommandArgs::FilterEffect {
                kind: arg.unwrap_or("gaussian").to_owned(),
            }),
            cid::FILTER_PREVIEW => Ok(CommandArgs::FilterPreview {
                kind: arg.unwrap_or("gaussian").to_owned(),
            }),
            cid::SHAPE_BOOLEAN => Ok(CommandArgs::ShapeBoolean {
                op: arg.unwrap_or("union").to_owned(),
            }),
            cid::RASTER_FLIP => Ok(CommandArgs::RasterFlip {
                horizontal: arg != Some("v"),
            }),
            cid::LAYER_SET_LOCKS => {
                let mut locks = self.active_lock_flags();
                match arg {
                    Some("pixels") => locks.pixels = !locks.pixels,
                    Some("position") => locks.position = !locks.position,
                    Some("all") => locks.all = !locks.all,
                    Some("alpha") => locks.alpha = !locks.alpha,
                    _ => locks.all = !locks.all,
                }
                Ok(CommandArgs::SetLocks {
                    pixels: locks.pixels || locks.all,
                    position: locks.position || locks.all,
                    all: locks.all,
                    alpha: locks.alpha || locks.all,
                })
            }
            _ => {
                if arg.is_none() {
                    Ok(CommandArgs::None)
                } else {
                    Err(CommandError::InvalidArgument(
                        "unsupported command args for action",
                    ))
                }
            }
        }
    }

    fn dispatch_host_op(&mut self, op: &str, arg: Option<&str>) {
        match op {
            "document.new" => {
                // QML must open new-doc dialog via destructive flow; signal via status.
                self.status_text = "host:document.new".into();
                self.status_text_changed();
            }
            "document.open" => {
                self.status_text = "host:document.open".into();
                self.status_text_changed();
            }
            "document.save" => {
                if !self.document_path.is_empty() {
                    self.save_document(String::new());
                } else {
                    self.status_text = "host:document.save_as".into();
                    self.status_text_changed();
                }
            }
            "document.save_as" => {
                self.status_text = "host:document.save_as".into();
                self.status_text_changed();
            }
            "document.export" => {
                self.status_text = "host:document.export".into();
                self.status_text_changed();
            }
            "document.close" => {
                self.status_text = "host:document.close".into();
                self.status_text_changed();
            }
            "app.quit" => {
                self.status_text = "host:app.quit".into();
                self.status_text_changed();
            }
            "help.about" => {
                self.status_text = "host:help.about".into();
                self.status_text_changed();
            }
            "prefs.open" => self.open_preferences(),
            "document.embed_icc" => {
                self.status_text = "host:document.embed_icc".into();
                self.status_text_changed();
            }
            "app.recover_gpu" => self.recover_gpu(),
            "palette.open" => {
                self.status_text = "host:palette.open".into();
                self.status_text_changed();
            }
            "clipboard.copy" => self.copy_selection(),
            "clipboard.copy_selection_mask" => self.copy_selection_mask(),
            "clipboard.copy_layer_mask" => self.copy_layer_mask(),
            "clipboard.paste_layer" => self.paste_as_new_layer(),
            "clipboard.paste_selection" => self.paste_selection_mask(),
            "clipboard.paste_mask" => self.paste_layer_mask(),
            "selection.select_all" => self.select_all(),
            "selection.deselect" => self.select_none(),
            "selection.invert" => self.invert_selection(),
            "selection.modify" => {
                let (op_name, radius) = parse_modify_arg(arg.unwrap_or("feather:4"));
                self.modify_selection(op_name, radius);
            }
            "raster.flip" => self.flip_active_layer(arg != Some("v")),
            "document.rotate_90" => self.rotate_canvas_90_cw(),
            "text.bake" => self.bake_text_layer(),
            "shape.create" => self.add_shape_layer(arg.unwrap_or("rect").to_owned()),
            "shape.rasterize" => self.rasterize_shape_layer(),
            "path.stroke" => self.stroke_active_path_to_layer(),
            "mask.create" => self.add_mask_to_active(),
            "mask.delete" => self.delete_mask_on_active(),
            "mask.toggle_enabled" => {
                let enabled = self
                    .engine
                    .graph
                    .as_ref()
                    .and_then(|g| {
                        let id = g.active_id()?;
                        g.get(id)?.mask.as_ref().map(|m| m.enabled)
                    })
                    .unwrap_or(false);
                self.set_mask_enabled_on_active(!enabled);
            }
            "layer.toggle_clip" => {
                let clips = self
                    .engine
                    .graph
                    .as_ref()
                    .and_then(|g| {
                        let id = g.active_id()?;
                        Some(g.get(id)?.clips_to_below)
                    })
                    .unwrap_or(false);
                self.set_clips_to_below_on_active(!clips);
            }
            "view.toggle_guides" => {
                let v = !self.engine.guides.show_guides;
                self.set_guides_visible(v);
            }
            "view.toggle_grid" => {
                let v = !self.engine.guides.show_grid;
                self.set_grid_visible(v);
            }
            "view.toggle_rulers" => {
                let v = !self.engine.guides.show_rulers;
                self.set_rulers_visible(v);
            }
            "view.toggle_snap" => {
                let v = !self.engine.guides.snap;
                self.set_snap_enabled(v);
            }
            "view.guide_v" => {
                let x = self.engine.size.width as f32 / 2.0;
                self.add_guide("v".into(), x);
            }
            "view.guide_h" => {
                let y = self.engine.size.height as f32 / 2.0;
                self.add_guide("h".into(), y);
            }
            "view.clear_guides" => self.clear_guides(),
            "workspace.reset" => self.reset_workspace(),
            op if op.starts_with("panel.toggle:") => {
                let panel = op.trim_start_matches("panel.toggle:");
                match panel {
                    "navigator" => {
                        self.set_panel_navigator_visible(!self.panel_navigator_visible);
                    }
                    "swatches" => {
                        self.set_panel_swatches_visible(!self.panel_swatches_visible);
                    }
                    "layers" => self.set_panel_layers_visible(!self.panel_layers_visible),
                    "history" => self.set_panel_history_visible(!self.panel_history_visible),
                    "properties" => {
                        self.set_panel_properties_visible(!self.panel_properties_visible);
                    }
                    _ => {}
                }
            }
            _ => {
                self.status_text = format!("Unknown host op: {op}");
                self.status_text_changed();
            }
        }
    }

    fn apply_command_effects(&mut self, effects: CommandEffects) {
        if let Some(host) = effects.host_history {
            self.apply_host_history(host);
        }
        match effects.host_follow_up {
            HostFollowUp::None => {}
            HostFollowUp::ConvertPixels { from, to } => {
                self.apply_convert_pixels(&from, &to);
            }
            HostFollowUp::ShowPreferences => self.open_preferences(),
            HostFollowUp::ResetWorkspace => self.reset_workspace(),
            HostFollowUp::TogglePanel { panel_id } => self.toggle_panel_by_id(&panel_id),
            HostFollowUp::ApplyWorkspacePreset { preset_id } => {
                self.apply_workspace_preset(preset_id);
            }
            HostFollowUp::SelectionToMask => self.apply_selection_to_mask_host(),
            HostFollowUp::MaskToSelection => self.apply_mask_to_selection_host(),
            HostFollowUp::ApplyMask => self.apply_mask_host(),
            HostFollowUp::HistoryJump { steps } => {
                for _ in 0..steps {
                    let _ = self.invoke_command(command_id::HISTORY_UNDO, CommandArgs::None);
                }
            }
            HostFollowUp::ShapeBoolean { op, a, b, result } => {
                self.apply_shape_boolean_host(op, a, b, result);
            }
            HostFollowUp::RasterizeShape { id } => {
                self.rasterize_shape_layer_id(id);
            }
            HostFollowUp::ShowFilterGallery => self.open_filter_gallery(),
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
        if effects.sync_selection {
            self.sync_from_engine();
            self.emit_selection_fields();
            self.can_undo_changed();
            self.history_labels_changed();
            self.status_text_changed();
        }
        if effects.generation > 0 {
            self.graph_revision = effects.generation.min(i32::MAX as u64) as i32;
            self.graph_revision_changed();
        }
    }

    fn apply_convert_pixels(&mut self, from: &str, to: &str) {
        match phototux_canvas::read_all_layer_rgba() {
            Ok(layers) => {
                for (id, _w, _h, mut pixels) in layers {
                    phototux_engine::convert_rgba8_profile(&mut pixels, from, to);
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
        if let Some(graph) = self.engine.graph.as_mut() {
            graph.color.mark_converted();
            graph.bump_generation();
        }
        self.status_text =
            format!("Converted pixels to {to} (from {from}) — this rewrote layer data");
        self.status_text_changed();
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
        // Multi-line integrity diagnostics (e.g. .ptx CRC) go straight into the dialog.
        self.io_error = if message.contains('\n') {
            format!("{operation} failed:\n{message}")
        } else {
            format!("{operation} failed: {message}")
        };
        self.io_busy_changed();
        self.io_error_changed();
        if message.contains(".ptx integrity") || message.contains("CRC32") {
            self.compatibility_report = message.to_owned();
            self.compatibility_report_changed();
        }
    }

    fn recomposite(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let mut layers = graph.layers().to_vec();
        if let Some(preview) = &self.engine.filter_preview {
            if let Some(effect) = preview.to_effect() {
                if let Some(layer) = layers.iter_mut().find(|l| l.id == preview.layer_id) {
                    layer.effects.push(effect);
                }
            }
        }
        match phototux_canvas::sync_and_composite(&layers) {
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
                self.engine.clear_dirty_rect();
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
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
                self.publish_pixel_snapshot_from_gpu();
            }
            Err(e) => self.report_gpu("open_document GPU", &e),
        }
        self.refresh_document_tabs_json();
    }

    fn publish_pixel_snapshot_from_gpu(&mut self) {
        if let Ok((_w, _h, rgba)) = phototux_canvas::read_composite_rgba() {
            let _ = self.engine.publish_pixel_snapshot_rgba(rgba);
        }
    }

    fn refresh_document_tabs_json(&mut self) {
        self.document_tabs_json = self.doc_registry.tabs_json(&self.document_name, self.dirty);
        self.document_tabs_json_changed();
    }

    /// Park the active document so another tab can become active.
    fn park_current_document(&mut self) -> Result<(), String> {
        let Some(id) = self.active_doc_id else {
            return Ok(());
        };
        if !self.engine.has_document {
            self.active_doc_id = None;
            self.doc_registry.set_active_id(None);
            return Ok(());
        }
        self.publish_pixel_snapshot_from_gpu();
        let layer_pixels: Vec<_> = phototux_canvas::read_all_layer_rgba()
            .unwrap_or_default()
            .into_iter()
            .map(|(layer_id, _w, _h, pixels)| (layer_id, pixels))
            .collect();
        let viewport_width = self.engine.viewport_w;
        let viewport_height = self.engine.viewport_h;
        let title = self.document_name.clone();
        let dirty = self.dirty;
        let session = std::mem::take(&mut self.engine);
        phototux_canvas::close_document();
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        self.doc_registry
            .park_active(id, title, session, layer_pixels, dirty);
        self.active_doc_id = None;
        self.engine = SessionState::default();
        self.engine.set_viewport(viewport_width, viewport_height);
        self.dirty = false;
        Ok(())
    }

    fn prepare_new_document_tab(&mut self, title: &str) -> Result<(), String> {
        // Refuse before parking so a full registry cannot leave the session with
        // no active document after a failed begin_active.
        if !self.doc_registry.can_open_another() {
            let limit = phototux_engine::max_open_documents();
            return Err(format!(
                "document limit reached ({limit}); close a tab first"
            ));
        }
        if self.engine.has_document {
            self.park_current_document()?;
        }
        let id = self.doc_registry.begin_active(title)?;
        self.active_doc_id = Some(id);
        Ok(())
    }

    fn activate_document_id(&mut self, id: OpenDocumentId) -> Result<(), String> {
        if self.active_doc_id == Some(id) {
            return Ok(());
        }
        if self.engine.has_document {
            self.park_current_document()?;
        }
        let parked = self
            .doc_registry
            .take_parked(id)
            .ok_or_else(|| format!("unknown document tab {}", id.0))?;
        let viewport_width = self.engine.viewport_w;
        let viewport_height = self.engine.viewport_h;
        self.engine = parked.session;
        self.engine.set_viewport(viewport_width, viewport_height);
        self.document_name = parked.title;
        self.dirty = parked.dirty;
        self.active_doc_id = Some(id);
        self.doc_registry.set_active_id(Some(id));
        if let Some(graph) = self.engine.graph.as_ref() {
            let size = graph.size;
            let layers = graph.layers().to_vec();
            match phototux_canvas::recover_gpu_document(size, &layers, &parked.layer_pixels) {
                Ok(ms) => {
                    self.engine.set_composite_ms(ms);
                    self.publish_pixel_snapshot_from_gpu();
                }
                Err(error) => self.report_gpu("activate document GPU", &error),
            }
        }
        self.sync_from_engine();
        self.emit_doc_fields();
        self.refresh_document_tabs_json();
        Ok(())
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
    qproperty!(
        "brushTextureStrength",
        Member = brush_texture_strength,
        Notify = brush_texture_strength_changed
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
        "layerSelection",
        Member = layer_selection,
        Notify = layer_selection_changed
    );
    qproperty!(
        "maskEditActive",
        Member = mask_edit_active,
        Notify = mask_edit_active_changed
    );
    qproperty!(
        "maskDensity",
        Member = mask_density,
        Notify = mask_density_changed
    );
    qproperty!(
        "maskFeather",
        Member = mask_feather,
        Notify = mask_feather_changed
    );
    qproperty!(
        "maskContrast",
        Member = mask_contrast,
        Notify = mask_contrast_changed
    );
    qproperty!(
        "maskShift",
        Member = mask_shift,
        Notify = mask_shift_changed
    );
    qproperty!(
        "dirtyRectJson",
        Member = dirty_rect_json,
        Notify = dirty_rect_json_changed
    );
    qproperty!(
        "overlayViewGeneration",
        Member = overlay_view_generation,
        Notify = overlay_view_generation_changed
    );
    qproperty!(
        "maskInverted",
        Member = mask_inverted,
        Notify = mask_inverted_changed
    );
    qproperty!(
        "maskLinked",
        Member = mask_linked,
        Notify = mask_linked_changed
    );
    qproperty!(
        "pixelSelectionActive",
        Member = pixel_selection_active,
        Notify = pixel_selection_active_changed
    );
    qproperty!(
        "objectSelectionLabel",
        Member = object_selection_label,
        Notify = object_selection_label_changed
    );
    qproperty!(
        "lastAnnounce",
        Member = last_announce,
        Notify = last_announce_changed
    );
    qproperty!(
        "editTarget",
        Member = edit_target,
        Notify = edit_target_changed
    );
    qproperty!(
        "editTargetLabel",
        Member = edit_target_label,
        Notify = edit_target_label_changed
    );
    qproperty!(
        "activeLayerKind",
        Member = active_layer_kind,
        Notify = active_layer_kind_changed
    );
    qproperty!(
        "historyLabels",
        Member = history_labels,
        Notify = history_labels_changed
    );
    qproperty!(
        "historyEntryIds",
        Member = history_entry_ids,
        Notify = history_entry_ids_changed
    );
    qproperty!(
        "historyKinds",
        Member = history_kinds,
        Notify = history_kinds_changed
    );
    qproperty!(
        "brushPresetNames",
        Member = brush_preset_names,
        Notify = brush_preset_names_changed
    );
    qproperty!(
        "softProofProfile",
        Member = soft_proof_profile,
        Notify = soft_proof_profile_changed
    );
    qproperty!(
        "softProofActive",
        Member = soft_proof_active,
        Notify = soft_proof_active_changed
    );
    qproperty!(
        "hasEmbeddedIcc",
        Member = has_embedded_icc,
        Notify = has_embedded_icc_changed
    );
    qproperty!(
        "displayProfileName",
        Member = display_profile_name,
        Notify = display_profile_name_changed
    );
    qproperty!(
        "displayProfileTag",
        Member = display_profile_tag,
        Notify = display_profile_tag_changed
    );
    qproperty!("gpuLost", Member = gpu_lost, Notify = gpu_lost_changed);
    qproperty!(
        "accessibilityTreeJson",
        Member = accessibility_tree_json,
        Notify = accessibility_tree_json_changed
    );
    qproperty!(
        "atspiProjectionJson",
        Member = atspi_projection_json,
        Notify = atspi_projection_json_changed
    );
    qproperty!(
        "recoveryEntriesJson",
        Member = recovery_entries_json,
        Notify = recovery_entries_json_changed
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
        "inspectorOpacityMixed",
        Member = inspector_opacity_mixed,
        Notify = inspector_opacity_mixed_changed
    );
    qproperty!(
        "inspectorBlendMixed",
        Member = inspector_blend_mixed,
        Notify = inspector_blend_mixed_changed
    );
    qproperty!(
        "propertiesAdvancedOpen",
        Member = properties_advanced_open,
        Notify = properties_advanced_open_changed
    );
    qproperty!(
        "prefEffectiveJson",
        Member = pref_effective_json,
        Notify = pref_effective_json_changed
    );
    qproperty!(
        "prefSafeStartNext",
        Member = pref_safe_start_next,
        Notify = pref_safe_start_next_changed
    );
    qproperty!(
        "prefHistoryRetention",
        Member = pref_history_retention,
        Notify = pref_history_retention_changed
    );
    qproperty!(
        "foregroundHex",
        Member = foreground_hex,
        Notify = foreground_hex_changed
    );
    qproperty!(
        "fillColorHex",
        Member = fill_color_hex,
        Notify = fill_color_hex_changed
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
    qproperty!(
        "effectsJoined",
        Member = effects_joined,
        Notify = effects_joined_changed
    );
    qproperty!("iconRoot", Member = icon_root, Notify = icon_root_changed);
    qproperty!(
        "documentName",
        Member = document_name,
        Notify = document_name_changed
    );
    qproperty!(
        "documentTabsJson",
        Member = document_tabs_json,
        Notify = document_tabs_json_changed
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
        "workspacePresetsJson",
        Member = workspace_presets_json,
        Notify = workspace_presets_json_changed
    );
    qproperty!(
        "workspaceFocusJson",
        Member = workspace_focus_json,
        Notify = workspace_focus_json_changed
    );
    qproperty!(
        "activeWorkspacePresetId",
        Member = active_workspace_preset_id,
        Notify = active_workspace_preset_id_changed
    );
    qproperty!(
        "dockTopologyJson",
        Member = dock_topology_json,
        Notify = dock_topology_json_changed
    );
    qproperty!(
        "panelVisibilityJson",
        Member = panel_visibility_json,
        Notify = panel_visibility_json_changed
    );
    qproperty!(
        "toolDescriptorsJson",
        Member = tool_descriptors_json,
        Notify = tool_descriptors_json_changed
    );
    qproperty!(
        "actionsJson",
        Member = actions_json,
        Notify = actions_json_changed
    );
    qproperty!(
        "shortcutsJson",
        Member = shortcuts_json,
        Notify = shortcuts_json_changed
    );
    qproperty!(
        "actionShortcutsJson",
        Member = action_shortcuts_json,
        Notify = action_shortcuts_json_changed
    );
    qproperty!(
        "shortcutInputYield",
        Member = shortcut_input_yield,
        Notify = shortcut_input_yield_changed
    );
    qproperty!(
        "preferencesOpen",
        Member = preferences_open,
        Notify = preferences_open_changed
    );
    qproperty!(
        "filterGalleryOpen",
        Member = filter_gallery_open,
        Notify = filter_gallery_open_changed
    );
    qproperty!(
        "filterPreviewActive",
        Member = filter_preview_active,
        Notify = filter_preview_active_changed
    );
    qproperty!(
        "filterPreviewKind",
        Member = filter_preview_kind,
        Notify = filter_preview_kind_changed
    );
    qproperty!(
        "filterPreviewP0",
        Member = filter_preview_p0,
        Notify = filter_preview_p0_changed
    );
    qproperty!(
        "filterPreviewP1",
        Member = filter_preview_p1,
        Notify = filter_preview_p1_changed
    );
    qproperty!(
        "filterPreviewP2",
        Member = filter_preview_p2,
        Notify = filter_preview_p2_changed
    );
    qproperty!(
        "pathClosed",
        Member = path_closed,
        Notify = path_closed_changed
    );
    qproperty!(
        "pathAnchorCount",
        Member = path_anchor_count,
        Notify = path_anchor_count_changed
    );
    qproperty!(
        "pathEditSelected",
        Member = path_edit_selected,
        Notify = path_edit_selected_changed
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
        "prefUiDensity",
        Member = pref_ui_density,
        Notify = pref_ui_density_changed
    );
    qproperty!(
        "prefHighContrast",
        Member = pref_high_contrast,
        Notify = pref_high_contrast_changed
    );
    qproperty!(
        "prefReducedMotion",
        Member = pref_reduced_motion,
        Notify = pref_reduced_motion_changed
    );
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
        "availableFontsJson",
        Member = available_fonts_json,
        Notify = available_fonts_json_changed
    );
    qproperty!(
        "textOriginX",
        Member = text_origin_x,
        Notify = text_origin_x_changed
    );
    qproperty!(
        "textOriginY",
        Member = text_origin_y,
        Notify = text_origin_y_changed
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
        "textFrameW",
        Member = text_frame_w,
        Notify = text_frame_w_changed
    );
    qproperty!(
        "textFrameH",
        Member = text_frame_h,
        Notify = text_frame_h_changed
    );
    qproperty!("textWrap", Member = text_wrap, Notify = text_wrap_changed);
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
    fn brush_texture_strength_changed(&mut self);
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
    fn layer_selection_changed(&mut self);
    #[qsignal]
    fn mask_edit_active_changed(&mut self);
    #[qsignal]
    fn mask_density_changed(&mut self);
    #[qsignal]
    fn mask_feather_changed(&mut self);
    #[qsignal]
    fn mask_contrast_changed(&mut self);
    #[qsignal]
    fn mask_shift_changed(&mut self);
    #[qsignal]
    fn dirty_rect_json_changed(&mut self);
    #[qsignal]
    fn overlay_view_generation_changed(&mut self);
    #[qsignal]
    fn mask_inverted_changed(&mut self);
    #[qsignal]
    fn mask_linked_changed(&mut self);
    #[qsignal]
    fn pixel_selection_active_changed(&mut self);
    #[qsignal]
    fn object_selection_label_changed(&mut self);
    #[qsignal]
    fn last_announce_changed(&mut self);
    #[qsignal]
    fn edit_target_changed(&mut self);
    #[qsignal]
    fn edit_target_label_changed(&mut self);
    #[qsignal]
    fn active_layer_kind_changed(&mut self);
    #[qsignal]
    fn history_labels_changed(&mut self);
    #[qsignal]
    fn history_entry_ids_changed(&mut self);
    #[qsignal]
    fn history_kinds_changed(&mut self);
    #[qsignal]
    fn brush_preset_names_changed(&mut self);
    #[qsignal]
    fn soft_proof_profile_changed(&mut self);
    #[qsignal]
    fn soft_proof_active_changed(&mut self);
    #[qsignal]
    fn has_embedded_icc_changed(&mut self);
    #[qsignal]
    fn display_profile_name_changed(&mut self);
    #[qsignal]
    fn display_profile_tag_changed(&mut self);
    #[qsignal]
    fn gpu_lost_changed(&mut self);
    #[qsignal]
    fn accessibility_tree_json_changed(&mut self);
    #[qsignal]
    fn atspi_projection_json_changed(&mut self);
    #[qsignal]
    fn recovery_entries_json_changed(&mut self);
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
    fn inspector_opacity_mixed_changed(&mut self);
    #[qsignal]
    fn inspector_blend_mixed_changed(&mut self);
    #[qsignal]
    fn properties_advanced_open_changed(&mut self);
    #[qsignal]
    fn pref_effective_json_changed(&mut self);
    #[qsignal]
    fn pref_safe_start_next_changed(&mut self);
    #[qsignal]
    fn pref_history_retention_changed(&mut self);
    #[qsignal]
    fn foreground_hex_changed(&mut self);
    #[qsignal]
    fn fill_color_hex_changed(&mut self);
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
    fn effects_joined_changed(&mut self);
    #[qsignal]
    fn icon_root_changed(&mut self);
    #[qsignal]
    fn document_name_changed(&mut self);
    #[qsignal]
    fn document_tabs_json_changed(&mut self);
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
    fn workspace_presets_json_changed(&mut self);
    #[qsignal]
    fn workspace_focus_json_changed(&mut self);
    #[qsignal]
    fn active_workspace_preset_id_changed(&mut self);
    #[qsignal]
    fn dock_topology_json_changed(&mut self);
    #[qsignal]
    fn panel_visibility_json_changed(&mut self);
    #[qsignal]
    fn tool_descriptors_json_changed(&mut self);
    #[qsignal]
    fn actions_json_changed(&mut self);
    #[qsignal]
    fn shortcuts_json_changed(&mut self);
    #[qsignal]
    fn action_shortcuts_json_changed(&mut self);
    #[qsignal]
    fn shortcut_input_yield_changed(&mut self);
    #[qsignal]
    fn preferences_open_changed(&mut self);
    #[qsignal]
    fn filter_gallery_open_changed(&mut self);
    #[qsignal]
    fn filter_preview_active_changed(&mut self);
    #[qsignal]
    fn filter_preview_kind_changed(&mut self);
    #[qsignal]
    fn filter_preview_p0_changed(&mut self);
    #[qsignal]
    fn filter_preview_p1_changed(&mut self);
    #[qsignal]
    fn filter_preview_p2_changed(&mut self);
    #[qsignal]
    fn path_closed_changed(&mut self);
    #[qsignal]
    fn path_anchor_count_changed(&mut self);
    #[qsignal]
    fn path_edit_selected_changed(&mut self);
    #[qsignal]
    fn pref_show_guides_changed(&mut self);
    #[qsignal]
    fn pref_show_grid_changed(&mut self);
    #[qsignal]
    fn pref_show_rulers_changed(&mut self);
    #[qsignal]
    fn pref_snap_changed(&mut self);
    #[qsignal]
    fn pref_ui_density_changed(&mut self);
    #[qsignal]
    fn pref_high_contrast_changed(&mut self);
    #[qsignal]
    fn pref_reduced_motion_changed(&mut self);
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
    fn available_fonts_json_changed(&mut self);
    #[qsignal]
    fn text_origin_x_changed(&mut self);
    #[qsignal]
    fn text_origin_y_changed(&mut self);
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
    fn text_frame_w_changed(&mut self);
    #[qsignal]
    fn text_frame_h_changed(&mut self);
    #[qsignal]
    fn text_wrap_changed(&mut self);
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
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_CONVERT_PROFILE,
            CommandArgs::ConvertProfile { profile },
        ) {
            self.report_gpu("convert profile", &error.to_string());
        }
    }

    /// Rebuild GPU document resources after device/surface loss (engine graph unchanged).
    #[qslot]
    fn recover_gpu(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            self.status_text = "Recover failed: no document".into();
            self.status_text_changed();
            return;
        };
        let doc_gen = graph.generation;
        let size = graph.size;
        let layers = graph.layers().to_vec();
        // Best-effort pixel snapshot before teardown; soft loss may still allow readback.
        let layer_pixels: Vec<_> = phototux_canvas::read_all_layer_rgba()
            .unwrap_or_default()
            .into_iter()
            .map(|(id, _w, _h, pixels)| (id, pixels))
            .collect();
        match phototux_canvas::recover_gpu_document(size, &layers, &layer_pixels) {
            Ok(ms) => {
                self.gpu_lost = false;
                self.gpu_lost_changed();
                self.engine.set_composite_ms(ms);
                let gen_after = self
                    .engine
                    .graph
                    .as_ref()
                    .map(|g| g.generation)
                    .unwrap_or(0);
                debug_assert_eq!(
                    doc_gen, gen_after,
                    "loss/recover must not bump document generation"
                );
                self.engine.announce("Graphics recovered — canvas restored");
                self.status_text = "Graphics recovered — canvas restored".into();
                self.status_text_changed();
                self.last_announce = self.engine.last_announce.clone();
                self.last_announce_changed();
            }
            Err(error) => {
                self.report_gpu("recover GPU", &error);
            }
        }
    }

    /// Load an ICC/ICM file and embed it on the document.
    #[qslot]
    fn embed_icc_from_file(&mut self, file_url: String) {
        let path = match local_path(&file_url) {
            Ok(path) => path,
            Err(error) => {
                self.status_text = format!("Embed ICC failed: {error}");
                self.status_text_changed();
                return;
            }
        };
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(error) => {
                self.status_text = format!("Embed ICC failed: {error}");
                self.status_text_changed();
                return;
            }
        };
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_SET_ICC,
            CommandArgs::SetIcc { bytes: Some(bytes) },
        ) {
            self.status_text = format!("Embed ICC failed: {error}");
            self.status_text_changed();
        }
    }

    #[qslot]
    fn clear_embedded_icc(&mut self) {
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_SET_ICC,
            CommandArgs::SetIcc { bytes: None },
        ) {
            self.status_text = format!("Clear ICC failed: {error}");
            self.status_text_changed();
        }
    }

    #[qslot]
    fn invoke_action(&mut self, id: String) {
        let Some(action) = phototux_engine::action_by_id(&id) else {
            self.status_text = format!("Unknown action: {id}");
            self.status_text_changed();
            return;
        };
        if !self.action_enablement(&action.enablement) {
            let label = action.label.replace('&', "");
            self.status_text = format!(
                "Action unavailable: {label} ({})",
                match action.enablement.as_str() {
                    "has_document" | "has_document_io_idle" => "no document open",
                    "can_undo" => "nothing to undo",
                    "can_redo" => "nothing to redo",
                    "io_idle" => "busy",
                    other => other,
                }
            );
            self.status_text_changed();
            return;
        }
        if let Some(host) = action.host_op.as_deref() {
            self.dispatch_host_op(host, action.arg.as_deref());
            return;
        }
        if let Some(cid) = action.command_id.as_deref() {
            match self.command_args_for_action(cid, action.arg.as_deref()) {
                Ok(args) => {
                    if let Err(error) = self.invoke_command(cid, args) {
                        self.report_action_error(&error.to_string());
                    }
                }
                Err(error) => self.report_action_error(&error.to_string()),
            }
        }
    }

    /// Surface action/command failures in the status footer and Properties announce.
    fn report_action_error(&mut self, error: &str) {
        let lower = error.to_ascii_lowercase();
        if lower.contains("rejected") || lower.contains("invalid argument") {
            self.status_text = error.to_owned();
            self.status_text_changed();
            self.engine.announce(error);
            self.last_announce = self.engine.last_announce.clone();
            self.last_announce_changed();
            return;
        }
        self.report_gpu("action", error);
    }

    #[qslot]
    fn action_enabled(&mut self, id: String) -> bool {
        phototux_engine::action_by_id(&id)
            .map(|a| self.action_enablement(&a.enablement))
            .unwrap_or(false)
    }

    #[qslot]
    fn shortcut_action(&mut self, chord: String) -> String {
        let action_map = phototux_engine::effective_action_shortcuts(&self.prefs.keymap);
        let map = phototux_engine::chord_map_from_action_shortcuts(&action_map);
        phototux_engine::resolve_shortcut(&map, &chord)
            .unwrap_or("")
            .to_owned()
    }

    #[qslot]
    fn handle_shortcut(&mut self, chord: String) -> bool {
        if self.preferences_open || self.shortcut_input_yield {
            return false;
        }
        let action_map = phototux_engine::effective_action_shortcuts(&self.prefs.keymap);
        let map = phototux_engine::chord_map_from_action_shortcuts(&action_map);
        let Some(action_id) = phototux_engine::resolve_shortcut(&map, &chord) else {
            return false;
        };
        let id = action_id.to_owned();
        self.invoke_action(id);
        true
    }

    #[qslot]
    fn set_shortcut_input_yield(&mut self, yield_input: bool) {
        if self.shortcut_input_yield == yield_input {
            return;
        }
        self.shortcut_input_yield = yield_input;
        self.shortcut_input_yield_changed();
    }

    #[qslot]
    fn shortcut_conflict_for(&mut self, action_id: String, chord: String) -> String {
        let effective = phototux_engine::effective_action_shortcuts(&self.prefs.keymap);
        phototux_engine::shortcut_conflict(&action_id, &chord, &effective).unwrap_or_default()
    }

    #[qslot]
    fn set_action_shortcut(&mut self, action_id: String, chord: String) {
        if phototux_engine::action_by_id(&action_id).is_none() {
            return;
        }
        let normalized = phototux_engine::normalize_shortcut(&chord);
        let defaults = phototux_engine::default_action_shortcuts();
        if normalized.is_empty() {
            self.prefs.keymap.remove(&action_id);
        } else if defaults.get(&action_id).is_some_and(|d| d == &normalized) {
            // Same as default — drop override.
            self.prefs.keymap.remove(&action_id);
        } else {
            let effective = phototux_engine::effective_action_shortcuts(&self.prefs.keymap);
            if let Some(other) =
                phototux_engine::shortcut_conflict(&action_id, &normalized, &effective)
            {
                // Steal chord: clear the other binding (as override to empty) or remove their override
                // and set empty override so default doesn't restore a clash.
                if defaults.contains_key(&other) {
                    self.prefs.keymap.insert(other, String::new());
                } else {
                    self.prefs.keymap.remove(&other);
                }
            }
            self.prefs.keymap.insert(action_id, normalized);
        }
        // Drop empty-string overrides that match “unbound” for actions without defaults.
        self.prefs.keymap.retain(|id, chord| {
            if !chord.is_empty() {
                return true;
            }
            // Keep empty override only when action has a default (means unbound).
            defaults.contains_key(id)
        });
        self.refresh_shortcut_maps();
        self.emit_shortcut_maps();
        self.persist_prefs();
    }

    #[qslot]
    fn reset_keymap(&mut self) {
        self.prefs.keymap.clear();
        self.refresh_shortcut_maps();
        self.emit_shortcut_maps();
        self.persist_prefs();
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
    fn open_filter_gallery(&mut self) {
        self.filter_gallery_open = true;
        self.filter_gallery_open_changed();
    }

    #[qslot]
    fn close_filter_gallery(&mut self) {
        if self.engine.filter_preview.is_some() {
            let _ = self.invoke_command(command_id::FILTER_CANCEL_PREVIEW, CommandArgs::None);
        }
        self.filter_gallery_open = false;
        self.filter_gallery_open_changed();
    }

    #[qslot]
    fn filter_gallery_preview(&mut self, kind: String) {
        if let Err(error) = self.invoke_command(
            command_id::FILTER_PREVIEW,
            CommandArgs::FilterPreview { kind },
        ) {
            self.status_text = error.to_string();
            self.status_text_changed();
        }
        self.sync_filter_preview_fields();
        self.emit_filter_preview_fields();
    }

    #[qslot]
    fn filter_gallery_set_params(&mut self, p0: f32, p1: f32, p2: f32) {
        if let Err(error) = self.invoke_command(
            command_id::FILTER_SET_PREVIEW_PARAMS,
            CommandArgs::FilterPreviewParams { p0, p1, p2 },
        ) {
            self.status_text = error.to_string();
            self.status_text_changed();
        }
        self.sync_filter_preview_fields();
        self.emit_filter_preview_fields();
    }

    #[qslot]
    fn filter_gallery_apply(&mut self) {
        if let Err(error) = self.invoke_command(command_id::FILTER_COMMIT, CommandArgs::None) {
            self.status_text = error.to_string();
            self.status_text_changed();
            return;
        }
        self.filter_gallery_open = false;
        self.filter_gallery_open_changed();
        self.sync_filter_preview_fields();
        self.emit_filter_preview_fields();
    }

    #[qslot]
    fn filter_gallery_cancel(&mut self) {
        self.close_filter_gallery();
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
    fn set_pref_ui_density(&mut self, value: String) {
        let density = if value == "comfortable" {
            "comfortable"
        } else {
            "dense"
        };
        self.prefs.ui_density = density.to_owned();
        self.pref_ui_density = density.to_owned();
        self.persist_prefs();
        self.pref_ui_density_changed();
    }

    #[qslot]
    fn set_pref_high_contrast(&mut self, value: bool) {
        self.prefs.high_contrast = value;
        self.pref_high_contrast = value;
        self.persist_prefs();
        self.pref_high_contrast_changed();
    }

    #[qslot]
    fn set_pref_reduced_motion(&mut self, value: bool) {
        self.prefs.reduced_motion = value;
        self.pref_reduced_motion = value;
        self.persist_prefs();
        self.pref_reduced_motion_changed();
    }

    #[qslot]
    fn set_pref_safe_start_next(&mut self, value: bool) {
        self.prefs.safe_start_next = value;
        self.pref_safe_start_next = value;
        self.persist_prefs();
        self.pref_safe_start_next_changed();
        if value {
            self.engine
                .announce("Safe start armed — next launch uses essentials chrome");
            self.status_text = self.engine.last_announce.clone();
            self.status_text_changed();
            self.last_announce = self.engine.last_announce.clone();
            self.last_announce_changed();
        }
    }

    #[qslot]
    fn set_pref_history_retention(&mut self, value: i32) {
        let clamped = crate::prefs::clamp_history_retention(value.max(0) as u32);
        self.prefs.history_retention_limit = clamped;
        self.pref_history_retention = clamped as i32;
        self.engine.history.set_limit(clamped as usize);
        self.persist_prefs();
        self.pref_history_retention_changed();
        self.can_undo = self.engine.history.can_undo();
        self.can_redo = self.engine.history.can_redo();
        self.history_labels = self.engine.history_labels_joined();
        self.history_entry_ids = self
            .engine
            .history
            .entry_ids_newest_first()
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("|");
        self.history_kinds = self.engine.history.kinds_newest_first().join("|");
        self.can_undo_changed();
        self.can_redo_changed();
        self.history_labels_changed();
        self.history_entry_ids_changed();
        self.history_kinds_changed();
    }

    #[qslot]
    fn set_properties_advanced_open(&mut self, open: bool) {
        self.properties_advanced_open = open;
        self.properties_advanced_open_changed();
    }

    #[qslot]
    fn set_panel_navigator_visible(&mut self, value: bool) {
        let _ = self.workspace.set_visible("panel.navigator", value);
        self.persist_workspace_visibility();
    }

    #[qslot]
    fn set_panel_swatches_visible(&mut self, value: bool) {
        let _ = self.workspace.set_visible("panel.swatches", value);
        self.persist_workspace_visibility();
    }

    #[qslot]
    fn set_panel_layers_visible(&mut self, value: bool) {
        let _ = self.workspace.set_visible("panel.layers", value);
        self.persist_workspace_visibility();
    }

    #[qslot]
    fn set_panel_history_visible(&mut self, value: bool) {
        let _ = self.workspace.set_visible("panel.history", value);
        self.persist_workspace_visibility();
    }

    #[qslot]
    fn set_panel_properties_visible(&mut self, value: bool) {
        let _ = self.workspace.set_visible("panel.properties", value);
        self.persist_workspace_visibility();
    }

    /// Set visibility by panel descriptor id (prefs / descriptor-driven chrome).
    #[qslot]
    fn set_panel_visible(&mut self, panel_id: String, value: bool) {
        if !self.workspace.set_visible(&panel_id, value) {
            self.status_text = format!("Unknown panel: {panel_id}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Replace dock topology from JSON (layout-only; never dirties document).
    #[qslot]
    fn set_dock_topology_json(&mut self, json: String) {
        match phototux_engine::DockTopology::from_json(&json) {
            Ok(dock) => {
                if let Err(reason) = self.workspace.set_dock(dock) {
                    self.status_text = format!("Dock topology rejected: {reason}");
                    self.status_text_changed();
                    return;
                }
                self.persist_workspace_visibility();
            }
            Err(reason) => {
                self.status_text = format!("Dock topology invalid: {reason}");
                self.status_text_changed();
            }
        }
    }

    /// Move a panel within the right stack (`delta` −1 up / +1 down).
    #[qslot]
    fn move_panel_in_stack(&mut self, panel_id: String, delta: i32) {
        if let Err(reason) = self.workspace.move_panel_in_stack(&panel_id, delta) {
            self.status_text = format!("Panel move failed: {reason}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Reorder right stack by indices (DnD commit).
    #[qslot]
    fn reorder_panel_in_stack(&mut self, from: i32, to: i32) {
        if from < 0 || to < 0 {
            return;
        }
        if let Err(reason) = self
            .workspace
            .reorder_panel_in_stack(from as usize, to as usize)
        {
            self.status_text = format!("Panel reorder failed: {reason}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Tear a docked panel into a floating window.
    #[qslot]
    fn tear_off_panel(&mut self, panel_id: String, x: i32, y: i32, width: i32, height: i32) {
        let w = width.max(200) as u32;
        let h = height.max(120) as u32;
        if let Err(reason) = self.workspace.tear_off_panel(&panel_id, x, y, w, h, "") {
            self.status_text = format!("Tear-off failed: {reason}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Return a floating panel to the right dock stack.
    #[qslot]
    fn redock_panel(&mut self, panel_id: String) {
        if let Err(reason) = self.workspace.redock_panel(&panel_id) {
            self.status_text = format!("Redock failed: {reason}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Persist floating window geometry (move/resize).
    #[qslot]
    fn set_floating_panel_geometry(
        &mut self,
        panel_id: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        let w = width.max(200) as u32;
        let h = height.max(120) as u32;
        if let Err(reason) = self.workspace.set_floating_geometry(&panel_id, x, y, w, h) {
            self.status_text = format!("Float geometry failed: {reason}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Clamp floating windows to the given screen rect (logical pixels).
    #[qslot]
    fn clamp_floating_panels(&mut self, x: i32, y: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let before = self.workspace.revision;
        self.workspace.clamp_floating(phototux_engine::ScreenRect {
            x,
            y,
            width: width as u32,
            height: height as u32,
        });
        if self.workspace.revision != before {
            self.persist_workspace_visibility();
        }
    }

    /// Toggle auto-hide for a docked panel (edge strip).
    #[qslot]
    fn toggle_panel_auto_hide(&mut self, panel_id: String) {
        if let Err(reason) = self.workspace.toggle_auto_hide(&panel_id) {
            self.status_text = format!("Auto-hide failed: {reason}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Pin (reveal) an auto-hidden panel.
    #[qslot]
    fn pin_panel(&mut self, panel_id: String) {
        if let Err(reason) = self.workspace.pin_panel(&panel_id) {
            self.status_text = format!("Pin failed: {reason}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    fn toggle_panel_by_id(&mut self, panel_id: &str) {
        if !self.workspace.toggle(panel_id) {
            self.status_text = format!("Unknown panel: {panel_id}");
            self.status_text_changed();
            return;
        }
        self.persist_workspace_visibility();
    }

    #[qslot]
    fn reset_workspace(&mut self) {
        self.workspace.reset_essentials();
        self.prefs.reset_workspace_essentials();
        self.sync_pref_fields_from_store();
        self.persist_prefs();
        self.emit_pref_fields();
        self.pref_effective_json_changed();
        self.status_text = "Workspace reset to Essentials".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn apply_workspace_preset(&mut self, preset_id: String) {
        let Some(preset) = phototux_engine::workspace_preset_by_id(&preset_id) else {
            self.status_text = format!("Unknown workspace preset: {preset_id}");
            self.status_text_changed();
            return;
        };
        self.workspace.apply_preset(&preset);
        self.persist_workspace_visibility();
        self.status_text = format!("Workspace: {}", preset.title);
        self.status_text_changed();
    }

    #[qslot]
    fn restore_last_saved_workspace(&mut self) {
        let Some(ws) = self.prefs.load_last_saved_workspace() else {
            self.status_text = "No last-saved workspace layout".to_owned();
            self.status_text_changed();
            return;
        };
        self.workspace = ws;
        self.persist_workspace_visibility();
        self.status_text = "Workspace restored from last saved".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn set_workspace_focus_path(&mut self, path: String) {
        self.workspace.set_focus_path(path);
        self.sync_panel_visibility_from_workspace();
        self.workspace_focus_json_changed();
    }

    #[qslot]
    fn set_workspace_panel_context(&mut self, panel_id: String) {
        self.workspace.set_panel_context(panel_id);
        self.sync_panel_visibility_from_workspace();
        self.workspace_focus_json_changed();
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
    fn set_brush_texture_strength(&mut self, value: f32) {
        let s = value.clamp(0.0, 1.0);
        self.engine.brush.texture_strength = s;
        self.engine.brush.texture = if s > 0.001 {
            phototux_engine::BrushTextureKind::Noise
        } else {
            phototux_engine::BrushTextureKind::None
        };
        self.brush_texture_strength = s;
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.brush_texture_strength_changed();
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
        // Leaving transform/crop must end the in-progress session (strip, palette, shortcuts).
        if id != tool_id::TRANSFORM && self.engine.transform_session.is_some() {
            self.cancel_transform();
        }
        if id != tool_id::CROP && self.crop_preview_active {
            self.cancel_crop();
        }
        let _ = self.invoke_command(
            command_id::VIEW_SET_TOOL,
            CommandArgs::Tool { tool: id.clone() },
        );
        self.engine.sync_brush_from_tool();
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.prefs.last_tool = id.clone();
        if self.prefs.restore_last_tool {
            self.persist_prefs();
        }
        // VIEW_SET_TOOL is view-only (no sync_doc); mirror engine into QML props here.
        self.active_tool = id;
        self.status_text = self.engine.status_summary();
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
                    // Telemetry only — do not full-sync (status/AT thrash freezes AT-SPI).
                    self.engine.set_composite_ms(ms);
                    self.composite_ms = ms;
                    self.composite_ms_changed();
                }
                EngineEvent::StrokeLatency { ms } => {
                    self.engine.set_stroke_latency_ms(ms);
                    self.stroke_latency_ms = ms;
                    self.stroke_latency_ms_changed();
                }
                EngineEvent::StrokeJournaled(entry) => {
                    // Best-effort recovery hook; never fail the stroke on journal I/O.
                    let _ = write_stroke_journal(&entry);
                }
                EngineEvent::StrokeEnded => {
                    let label = if self.mask_edit_active {
                        "Mask stroke"
                    } else {
                        "Brush stroke"
                    };
                    let _ = self.invoke_command(
                        command_id::RASTER_PAINT_STROKE,
                        CommandArgs::RasterPaintStroke {
                            label: label.to_owned(),
                        },
                    );
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
                    if let Err(error) = self.prepare_new_document_tab(&layer_name) {
                        self.fail_io("Open", &error);
                        continue;
                    }
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
                            self.publish_pixel_snapshot_from_gpu();
                            self.sync_from_engine();
                            self.emit_doc_fields();
                            self.refresh_document_tabs_json();
                        }
                        Err(error) => self.fail_io("Open", &error),
                    }
                }
                FileEvent::PtxOpened { path, document } => {
                    self.apply_opened_ptx(path, document);
                    self.dirty = false;
                    self.dirty_changed();
                }
                FileEvent::PsdOpened {
                    path,
                    graph,
                    layer_rasters,
                    flattened,
                    report,
                } => {
                    let title = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Imported.psd".into());
                    if let Err(error) = self.prepare_new_document_tab(&title) {
                        self.fail_io("Open", &error);
                        continue;
                    }
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
                            self.document_name = title;
                            self.dirty = true;
                            self.io_busy = false;
                            self.compatibility_report = format_report(&report);
                            self.publish_pixel_snapshot_from_gpu();
                            self.sync_from_engine();
                            self.emit_doc_fields();
                            self.compatibility_report_changed();
                            self.refresh_document_tabs_json();
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
            let prev_status = self.status_text.clone();
            let prev_a11y = self.accessibility_tree_json.clone();
            self.sync_from_engine();
            self.can_undo_changed();
            self.can_redo_changed();
            self.dirty_changed();
            if self.status_text != prev_status {
                self.status_text_changed();
            }
            if self.accessibility_tree_json != prev_a11y {
                self.accessibility_tree_json_changed();
                self.atspi_projection_json_changed();
            }
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
        let _ = self.invoke_command(command_id::VIEW_PAN_BY, CommandArgs::PanBy { dx, dy });
    }

    #[qslot]
    fn zoom_at(&mut self, factor: f32, anchor_x: f32, anchor_y: f32) {
        let _ = self.invoke_command(
            command_id::VIEW_ZOOM_AT,
            CommandArgs::ZoomAt {
                factor,
                anchor_x,
                anchor_y,
            },
        );
    }

    #[qslot]
    fn zoom_to_fit(&mut self) {
        let _ = self.invoke_command(command_id::VIEW_ZOOM_TO_FIT, CommandArgs::None);
    }

    #[qslot]
    fn apply_size_preset(&mut self, label: String) {
        if let Err(error) = self.prepare_new_document_tab("Untitled") {
            self.status_text = error;
            self.status_text_changed();
            return;
        }
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
            self.refresh_document_tabs_json();
        }
    }

    #[qslot]
    fn apply_document_size(&mut self, width: i32, height: i32) {
        if let Err(error) = self.prepare_new_document_tab("Untitled") {
            self.status_text = error;
            self.status_text_changed();
            return;
        }
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
            self.refresh_document_tabs_json();
        }
    }

    #[qslot]
    fn activate_document_tab(&mut self, id: i32) {
        if id < 0 {
            return;
        }
        if let Err(error) = self.activate_document_id(OpenDocumentId(id as u64)) {
            self.status_text = error;
            self.status_text_changed();
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
    fn sync_recovery_list_fields(&mut self) {
        let entries = list_recoverable().unwrap_or_default();
        self.recovery_entries_json =
            serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into());
    }

    #[qslot]
    fn refresh_recovery_list(&mut self) {
        self.sync_recovery_list_fields();
        self.emit_recovery_list();
    }

    fn emit_recovery_list(&mut self) {
        self.recovery_entries_json_changed();
    }

    #[qslot]
    fn restore_recovery(&mut self, document_id: String) {
        let Ok(entries) = list_recoverable() else {
            self.fail_io("Recover", "could not list recovery entries");
            return;
        };
        let Some(entry) = entries.into_iter().find(|e| e.document_id == document_id) else {
            self.fail_io("Recover", "recovery entry not found");
            return;
        };
        match load_recovery(&entry) {
            Ok(document) => {
                let path = entry
                    .original_path
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(&entry.snapshot_path));
                self.apply_opened_ptx(path, document);
                let _ = discard_recovery(&entry);
                self.sync_recovery_list_fields();
                self.emit_recovery_list();
                self.status_text = "Recovered unsaved document".to_owned();
                self.status_text_changed();
            }
            Err(error) => self.fail_io("Recover", &error.to_string()),
        }
    }

    #[qslot]
    fn discard_recovery_entry(&mut self, document_id: String) {
        let Ok(entries) = list_recoverable() else {
            return;
        };
        if let Some(entry) = entries.into_iter().find(|e| e.document_id == document_id) {
            let _ = discard_recovery(&entry);
        }
        self.sync_recovery_list_fields();
        self.emit_recovery_list();
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
            let icc = self
                .engine
                .graph
                .as_ref()
                .and_then(|g| g.color.embedded_icc.clone());
            self.file_worker
                .send(FileCommand::Export { path, format, icc })
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
        self.active_doc_id = None;
        self.doc_registry.set_active_id(None);
        self.selection_preview_active = false;
        self.crop_preview_active = false;
        let next = self.doc_registry.parked_ids().next();
        if let Some(next_id) = next {
            if let Err(error) = self.activate_document_id(next_id) {
                self.status_text = error;
                self.status_text_changed();
                self.document_name = "Untitled".to_owned();
                self.dirty = false;
                self.sync_from_engine();
                self.emit_doc_fields();
                self.refresh_document_tabs_json();
            }
        } else {
            self.document_name = "Untitled".to_owned();
            self.dirty = false;
            self.sync_from_engine();
            self.emit_doc_fields();
            self.refresh_document_tabs_json();
        }
    }

    #[qslot]
    fn clear_host_status_marker(&mut self) {
        if !self.status_text.starts_with("host:") {
            return;
        }
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
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

    /// Layers panel click with modifiers (`ctrl` toggle, `shift` range).
    #[qslot]
    fn select_layer_click(&mut self, index: i32, ctrl: bool, shift: bool) {
        if index < 0 {
            return;
        }
        self.engine.select_layer_click(index as usize, ctrl, shift);
        self.sync_from_engine();
        self.emit_layer_fields();
        self.sync_selection_fields();
        self.emit_selection_fields();
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
        self.selection_preview_active = false;
        self.clear_selection_path();
        let _ = self.invoke_command(
            command_id::SELECTION_REPLACE,
            CommandArgs::SelectionReplace {
                shape,
                combine: mode,
                rect,
                polygon: Vec::new(),
                label: label.to_owned(),
            },
        );
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
        if SelectionState::polygon_bounds(&parsed).is_none() {
            self.clear_selection_path();
            return;
        }
        self.push_selection_snapshot();
        if let Err(error) = phototux_canvas::selection_apply_polygon(&parsed, mode) {
            self.report_gpu("polygon selection", &error);
        }
        self.selection_preview_active = false;
        self.clear_selection_path();
        let label = if self.engine.active_tool.contains("lasso") {
            "Lasso selection"
        } else {
            "Polygonal selection"
        };
        let _ = self.invoke_command(
            command_id::SELECTION_REPLACE,
            CommandArgs::SelectionReplace {
                shape: SelectionShape::Mask,
                combine: mode,
                rect: SelectionRect {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                },
                polygon: parsed,
                label: label.to_owned(),
            },
        );
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
        self.selection_preview_active = false;
        self.clear_selection_path();
        let _ = self.invoke_command(command_id::SELECTION_DESELECT, CommandArgs::None);
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
        self.selection_preview_active = false;
        let _ = self.invoke_command(command_id::SELECTION_SELECT_ALL, CommandArgs::None);
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
        self.selection_preview_active = false;
        let _ = self.invoke_command(command_id::SELECTION_INVERT, CommandArgs::None);
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
        // Prefer selection coverage when a pixel selection is active (handbook §21).
        // Also capture masked pixels so Paste as New Layer has an RGBA payload.
        if self.engine.selection.active {
            self.copy_active_selection_payload();
            return;
        }
        let Ok((width, height, pixels)) = phototux_canvas::read_composite_rgba() else {
            return;
        };
        // Hostile-input bound: refuse payloads over 64 MiB RGBA.
        const MAX_CLIPBOARD_BYTES: usize = 64 * 1024 * 1024;
        if pixels.len() > MAX_CLIPBOARD_BYTES {
            self.status_text = "Copy refused: clipboard size limit".to_owned();
            self.status_text_changed();
            return;
        }
        let os_ok = Self::push_os_clipboard_rgba(width, height, &pixels).is_ok();
        self.clipboard_rgba = Some((width, height, pixels));
        self.status_text = if os_ok {
            "Copied (app + system clipboard)".to_owned()
        } else {
            "Copied (app clipboard only)".to_owned()
        };
        self.status_text_changed();
    }

    /// Copy selection coverage plus active-layer (or composite) pixels masked by it.
    fn copy_active_selection_payload(&mut self) {
        let Ok(r8) = phototux_canvas::selection_snapshot() else {
            self.status_text = "Copy selection failed: no mask".to_owned();
            self.status_text_changed();
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let width = graph.size.width;
        let height = graph.size.height;
        const MAX_CLIPBOARD_BYTES: usize = 64 * 1024 * 1024;
        let px_count = (width as usize).saturating_mul(height as usize);
        if r8.len() != px_count {
            self.status_text = "Copy selection failed: size mismatch".to_owned();
            self.status_text_changed();
            return;
        }
        if r8.len() > MAX_CLIPBOARD_BYTES {
            self.status_text = "Copy refused: clipboard size limit".to_owned();
            self.status_text_changed();
            return;
        }

        let rgba_result = self
            .active_id()
            .and_then(|id| phototux_canvas::read_layer_rgba(id).ok())
            .or_else(|| phototux_canvas::read_composite_rgba().ok());
        let Some((rw, rh, mut pixels)) = rgba_result else {
            // Coverage-only fallback so Paste as Selection still works.
            self.copy_selection_mask();
            return;
        };
        if rw != width || rh != height || pixels.len() != px_count.saturating_mul(4) {
            self.copy_selection_mask();
            return;
        }
        if pixels.len() > MAX_CLIPBOARD_BYTES {
            self.status_text = "Copy refused: clipboard size limit".to_owned();
            self.status_text_changed();
            return;
        }

        for (i, &cov) in r8.iter().enumerate() {
            let a = u16::from(pixels[i * 4 + 3]);
            pixels[i * 4 + 3] = ((a * u16::from(cov)) / 255) as u8;
        }

        let os_ok = Self::push_os_clipboard_rgba(width, height, &pixels).is_ok();
        self.clipboard_selection_r8 = Some((width, height, r8));
        self.clipboard_rgba = Some((width, height, pixels));
        self.engine.announce("Selection copied");
        self.status_text = if os_ok {
            "Copied selection pixels (app + system clipboard)".to_owned()
        } else {
            "Copied selection pixels (app clipboard)".to_owned()
        };
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    #[qslot]
    fn copy_selection_mask(&mut self) {
        let Ok(r8) = phototux_canvas::selection_snapshot() else {
            self.status_text = "Copy selection failed: no mask".to_owned();
            self.status_text_changed();
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let width = graph.size.width;
        let height = graph.size.height;
        const MAX_CLIPBOARD_BYTES: usize = 64 * 1024 * 1024;
        if r8.len() > MAX_CLIPBOARD_BYTES {
            self.status_text = "Copy refused: clipboard size limit".to_owned();
            self.status_text_changed();
            return;
        }
        if r8.len() != (width as usize) * (height as usize) {
            self.status_text = "Copy selection failed: size mismatch".to_owned();
            self.status_text_changed();
            return;
        }
        // OS preview: grayscale RGBA so external apps see coverage.
        let rgba = r8_to_gray_rgba(&r8);
        let os_ok = Self::push_os_clipboard_rgba(width, height, &rgba).is_ok();
        self.clipboard_selection_r8 = Some((width, height, r8));
        self.engine.announce("Selection copied");
        self.status_text = if os_ok {
            "Copied selection (app + system grayscale)".to_owned()
        } else {
            "Copied selection (app clipboard)".to_owned()
        };
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    #[qslot]
    fn copy_layer_mask(&mut self) {
        let Some(id) = self.active_id() else {
            self.status_text = "Copy mask failed: no active layer".to_owned();
            self.status_text_changed();
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        if !graph
            .layers()
            .iter()
            .any(|l| l.id == id && l.mask.is_some())
        {
            self.status_text = "Copy mask failed: layer has no mask".to_owned();
            self.status_text_changed();
            return;
        }
        let width = graph.size.width;
        let height = graph.size.height;
        let Ok(r8) = phototux_canvas::read_mask_r8(id) else {
            self.status_text = "Copy mask failed: layer has no mask".to_owned();
            self.status_text_changed();
            return;
        };
        const MAX_CLIPBOARD_BYTES: usize = 64 * 1024 * 1024;
        if r8.len() > MAX_CLIPBOARD_BYTES {
            self.status_text = "Copy refused: clipboard size limit".to_owned();
            self.status_text_changed();
            return;
        }
        let rgba = r8_to_gray_rgba(&r8);
        let os_ok = Self::push_os_clipboard_rgba(width, height, &rgba).is_ok();
        self.clipboard_mask_r8 = Some((width, height, r8));
        self.engine.announce("Layer mask copied");
        self.status_text = if os_ok {
            "Copied layer mask (app + system grayscale)".to_owned()
        } else {
            "Copied layer mask (app clipboard)".to_owned()
        };
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    #[qslot]
    fn paste_selection_mask(&mut self) {
        let Some((width, height, r8)) = self
            .clipboard_selection_r8
            .clone()
            .or_else(|| self.clipboard_mask_r8.clone())
        else {
            self.fail_io("Paste selection", "clipboard has no selection/mask payload");
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        if width != graph.size.width || height != graph.size.height {
            self.fail_io(
                "Paste selection",
                "clipboard mask size does not match document",
            );
            return;
        }
        if let Err(error) = phototux_canvas::selection_restore(&r8) {
            self.report_gpu("paste selection", &error);
            return;
        }
        self.engine.selection.active = true;
        self.sync_from_engine();
        self.emit_doc_fields();
        self.engine.announce("Selection pasted");
        self.status_text = "Pasted selection from clipboard".to_owned();
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    #[qslot]
    fn paste_layer_mask(&mut self) {
        let Some((width, height, r8)) = self
            .clipboard_mask_r8
            .clone()
            .or_else(|| self.clipboard_selection_r8.clone())
        else {
            self.fail_io("Paste mask", "clipboard has no mask/selection payload");
            return;
        };
        let Some(id) = self.active_id() else {
            self.fail_io("Paste mask", "no active layer");
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        if width != graph.size.width || height != graph.size.height {
            self.fail_io("Paste mask", "clipboard mask size does not match document");
            return;
        }
        let needs_mask = graph
            .layers()
            .iter()
            .find(|l| l.id == id)
            .is_some_and(|l| l.mask.is_none());
        if needs_mask {
            if let Err(error) = self.invoke_command(command_id::MASK_CREATE, CommandArgs::None) {
                self.report_gpu("paste mask create", &error.to_string());
                return;
            }
        }
        if let Err(error) = phototux_canvas::ensure_mask(id) {
            self.report_gpu("paste mask ensure", &error);
            return;
        }
        if let Err(error) = phototux_canvas::write_mask_r8(id, &r8) {
            self.report_gpu("paste mask upload", &error);
            return;
        }
        self.recomposite();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.engine.announce("Mask pasted onto active layer");
        self.status_text = "Pasted mask onto active layer".to_owned();
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    fn push_os_clipboard_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard
            .set_image(arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: std::borrow::Cow::Borrowed(pixels),
            })
            .map_err(|e| e.to_string())
    }

    fn pull_os_clipboard_rgba() -> Option<(u32, u32, Vec<u8>)> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let img = clipboard.get_image().ok()?;
        let width = u32::try_from(img.width).ok()?;
        let height = u32::try_from(img.height).ok()?;
        const MAX_CLIPBOARD_BYTES: usize = 64 * 1024 * 1024;
        if img.bytes.len() > MAX_CLIPBOARD_BYTES {
            return None;
        }
        Some((width, height, img.bytes.into_owned()))
    }

    #[qslot]
    fn jump_history_entry(&mut self, entry_id: i64) {
        if entry_id < 0 {
            return;
        }
        let _ = self.invoke_command(
            command_id::HISTORY_JUMP,
            CommandArgs::HistoryJump {
                entry_id: entry_id as u64,
            },
        );
    }

    #[qslot]
    fn set_soft_proof(&mut self, profile: String, intent: String) {
        let _ = self.invoke_command(
            command_id::DOCUMENT_SET_SOFT_PROOF,
            CommandArgs::SoftProof { profile, intent },
        );
    }

    /// Soft-proof using the discovered host display profile tag.
    #[qslot]
    fn use_display_soft_proof(&mut self) {
        if self.display_profile_tag.is_empty() {
            let display = display_icc::discover_display_profile();
            self.display_profile_name = display.name.clone();
            self.display_profile_tag = display.soft_proof_tag();
            self.display_profile_name_changed();
            self.display_profile_tag_changed();
        }
        let tag = self.display_profile_tag.clone();
        self.set_soft_proof(tag, "relative".into());
        self.status_text = format!(
            "Soft-proof: display profile ({})",
            self.display_profile_name
        );
        self.status_text_changed();
    }

    #[qslot]
    fn apply_brush_preset(&mut self, index: i32) {
        let Some(preset) = self
            .engine
            .brush_presets
            .apply_index(index.max(0) as usize)
            .cloned()
        else {
            return;
        };
        self.engine.set_brush_size(preset.size);
        self.engine.set_brush_hardness(preset.hardness);
        self.engine.set_brush_color(
            preset.color[0],
            preset.color[1],
            preset.color[2],
            preset.color[3],
        );
        self.engine.brush.opacity = preset.opacity.clamp(0.0, 1.0);
        self.engine.brush.flow = preset.flow.clamp(0.0, 1.0);
        self.engine.brush.spacing_ratio = preset.spacing.clamp(0.05, 2.0);
        self.engine.brush.scatter = preset.scatter.clamp(0.0, 1.0);
        self.engine.brush.size_pressure = preset.size_pressure;
        self.engine.brush.opacity_pressure = preset.opacity_pressure;
        self.engine.brush.texture =
            phototux_engine::BrushTextureKind::from_str_key(&preset.texture);
        self.engine.brush.texture_strength = preset.texture_strength.clamp(0.0, 1.0);
        self.send_paint(EngineCommand::SetBrush(self.engine.brush));
        self.sync_from_engine();
        self.brush_size_changed();
        self.brush_hardness_changed();
        self.brush_texture_strength_changed();
        self.brush_color_changed();
        self.status_text = format!("Brush preset: {}", preset.name);
        self.status_text_changed();
    }

    #[qslot]
    fn save_current_brush_preset(&mut self, name: String) {
        let name = if name.trim().is_empty() {
            "Custom".to_owned()
        } else {
            name
        };
        let preset = phototux_engine::BrushPreset {
            name,
            size: self.engine.brush_size,
            hardness: self.engine.brush_hardness,
            opacity: self.engine.brush.opacity,
            flow: self.engine.brush.flow,
            spacing: self.engine.brush.spacing_ratio,
            smoothing: 0.0,
            size_pressure: self.engine.brush.size_pressure,
            opacity_pressure: self.engine.brush.opacity_pressure,
            scatter: self.engine.brush.scatter,
            texture: self.engine.brush.texture.as_str().to_owned(),
            texture_strength: self.engine.brush.texture_strength,
            color: self.engine.brush_color,
        };
        self.engine.brush_presets.upsert(preset);
        self.prefs.brush_presets_json = self.engine.brush_presets.to_json().unwrap_or_default();
        let _ = self.prefs.save();
        self.brush_preset_names = self.engine.brush_presets.names_joined();
        self.brush_preset_names_changed();
    }

    #[qslot]
    fn paste_as_new_layer(&mut self) {
        let payload = self
            .clipboard_rgba
            .clone()
            .or_else(Self::pull_os_clipboard_rgba);
        let Some((width, height, pixels)) = payload else {
            self.fail_io("Paste", "clipboard empty");
            return;
        };
        let _ = (width, height);
        match self.invoke_command(
            command_id::CLIPBOARD_PASTE_LAYER,
            CommandArgs::PasteLayer {
                name: "Pasted".into(),
            },
        ) {
            Ok(()) => {
                if let Some(id) = self.engine.graph.as_ref().and_then(|g| g.active_id()) {
                    if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
                        self.report_gpu("paste upload", &error);
                        return;
                    }
                    self.recomposite();
                    self.status_text = "Pasted layer".to_owned();
                    self.status_text_changed();
                }
            }
            Err(error) => self.report_gpu("paste", &error.to_string()),
        }
    }

    #[qslot]
    fn add_group_layer(&mut self) {
        if let Err(error) = self.invoke_command(command_id::LAYER_GROUP, CommandArgs::None) {
            self.report_gpu("add group", &error.to_string());
        }
    }

    #[qslot]
    fn add_fill_layer(&mut self) {
        let color = self.engine.colors.foreground;
        if let Err(error) = self.invoke_command(
            command_id::LAYER_CREATE_FILL,
            CommandArgs::FillCreate {
                color_rgba: [color[0], color[1], color[2], 1.0],
            },
        ) {
            self.report_gpu("add fill", &error.to_string());
        }
    }

    #[qslot]
    fn set_active_fill_hex(&mut self, hex: String) {
        let Some(rgba) = phototux_engine::ColorState::from_hex(&hex) else {
            return;
        };
        let _ = self.invoke_command(
            command_id::LAYER_SET_FILL_COLOR,
            CommandArgs::FillColor { color_rgba: rgba },
        );
    }

    #[qslot]
    fn add_text_layer(&mut self, text: String) {
        if let Err(error) =
            self.invoke_command(command_id::TEXT_CREATE, CommandArgs::TextCreate { text })
        {
            self.report_gpu("add text", &error.to_string());
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
        if let Err(error) = self.invoke_command(command_id::TEXT_BAKE, CommandArgs::None) {
            self.report_gpu("bake text", &error.to_string());
            return;
        }
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
            self.report_gpu("bake text upload", &error);
            return;
        }
        self.recomposite();
        self.engine
            .announce("Text baked to pixels — editable text discarded");
        self.status_text = "Text baked to pixels — editable text discarded".to_owned();
        self.status_text_changed();
    }

    /// Create a shape layer (`kind`: rect|ellipse|polygon|gradient|line|live).
    #[qslot]
    fn add_shape_layer(&mut self, kind: String) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let w = graph.size.width as f32;
        let h = graph.size.height as f32;
        let doc_w = graph.size.width;
        let doc_h = graph.size.height;
        let (path, kind_key, filled, gradient) = match kind.as_str() {
            "ellipse" => (
                ellipse_path("Ellipse", w * 0.5, h * 0.5, w * 0.2, h * 0.15),
                "ellipse",
                true,
                None,
            ),
            "polygon" => (
                polygon_path("Polygon", w * 0.5, h * 0.5, w.min(h) * 0.22, 6),
                "polygon",
                true,
                None,
            ),
            "gradient" => (
                rect_path("Gradient", w * 0.25, h * 0.25, w * 0.5, h * 0.4),
                "rect",
                true,
                Some(ShapeGradient {
                    x0: w * 0.25,
                    y0: h * 0.25,
                    x1: w * 0.75,
                    y1: h * 0.65,
                    c0_rgba: [0.2, 0.45, 0.9, 1.0],
                    c1_rgba: [0.95, 0.35, 0.2, 1.0],
                }),
            ),
            "line" => (
                VectorPath::polyline(
                    "Line",
                    vec![
                        PathPoint {
                            x: w * 0.2,
                            y: h * 0.5,
                        },
                        PathPoint {
                            x: w * 0.8,
                            y: h * 0.5,
                        },
                    ],
                    false,
                ),
                "line",
                false,
                None,
            ),
            _ => (
                rect_path("Rectangle", w * 0.25, h * 0.25, w * 0.5, h * 0.4),
                "rect",
                true,
                None,
            ),
        };
        let live_vector = matches!(kind.as_str(), "live" | "polygon" | "gradient");
        let content = ShapeContent {
            path,
            filled,
            stroked: true,
            kind: kind_key.into(),
            live_vector,
            gradient,
            ..ShapeContent::default()
        };
        match self.invoke_command(
            command_id::SHAPE_CREATE,
            CommandArgs::ShapeCreate {
                content: Box::new(content.clone()),
            },
        ) {
            Ok(()) => {
                let Some(id) = self.engine.graph.as_ref().and_then(|g| g.active_id()) else {
                    return;
                };
                match Self::shape_pixels(&content, doc_w, doc_h) {
                    Ok(pixels) => {
                        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
                            self.report_gpu("shape upload", &error);
                            return;
                        }
                    }
                    Err(error) => {
                        self.report_gpu("shape raster", &error);
                        return;
                    }
                }
                self.recomposite();
            }
            Err(error) => self.report_gpu("add shape", &error.to_string()),
        }
    }

    fn shape_pixels(content: &ShapeContent, w: u32, h: u32) -> Result<Vec<u8>, String> {
        let stroke = content.stroked.then(|| {
            [
                (content.stroke_rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                (content.stroke_rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                (content.stroke_rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                (content.stroke_rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
            ]
        });
        let mut base = if let Some(grad) = content.gradient.as_ref() {
            let pixels = (w as usize)
                .checked_mul(h as usize)
                .and_then(|n| n.checked_mul(4))
                .ok_or_else(|| "dimensions overflow".to_owned())?;
            let mut out = vec![0_u8; pixels];
            if content.filled {
                let c0 = [
                    (grad.c0_rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (grad.c0_rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (grad.c0_rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (grad.c0_rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
                ];
                let c1 = [
                    (grad.c1_rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (grad.c1_rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (grad.c1_rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (grad.c1_rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
                ];
                fill_gradient_even_odd(
                    &mut out,
                    w,
                    h,
                    &content.path,
                    grad.x0,
                    grad.y0,
                    grad.x1,
                    grad.y1,
                    c0,
                    c1,
                );
            }
            if let Some(stroke) = stroke {
                let stroked = stroke_path_rgba8(w, h, &content.path, stroke, content.stroke_width)?;
                for (d, s) in out.chunks_exact_mut(4).zip(stroked.chunks_exact(4)) {
                    if s[3] > 0 {
                        d.copy_from_slice(s);
                    }
                }
            }
            out
        } else {
            let fill = content.filled.then(|| {
                [
                    (content.fill_rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (content.fill_rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (content.fill_rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (content.fill_rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
                ]
            });
            rasterize_shape_rgba8(w, h, &content.path, fill, stroke, content.stroke_width)?
        };
        if let Some(partner) = content.boolean_partner.as_ref() {
            let op = phototux_engine::BooleanOp::parse(&partner.op)
                .unwrap_or(phototux_engine::BooleanOp::Union);
            let fill_b = [
                (partner.fill_rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                (partner.fill_rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                (partner.fill_rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                (partner.fill_rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
            ];
            let b = rasterize_shape_rgba8(w, h, &partner.path, Some(fill_b), None, 0.0)?;
            base = phototux_engine::boolean_rgba8(&base, &b, op)?;
        }
        Ok(base)
    }

    fn apply_shape_boolean_host(
        &mut self,
        op: phototux_engine::BooleanOp,
        a: LayerId,
        b: LayerId,
        result: LayerId,
    ) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let Some(content_a) = graph.get(a).and_then(|l| l.shape.clone()) else {
            self.report_gpu("shape boolean", "shape A missing");
            return;
        };
        let Some(content_b) = graph.get(b).and_then(|l| l.shape.clone()) else {
            self.report_gpu("shape boolean", "shape B missing");
            return;
        };
        // Vector-preserving spine: keep ShapeContent with boolean partner when both are shapes.
        let mut preserved = content_a.clone();
        preserved.live_vector = true;
        preserved.boolean_partner = Some(ShapeBooleanPartner {
            op: op.as_str().to_owned(),
            path: content_b.path.clone(),
            fill_rgba: content_b.fill_rgba,
        });
        if let Some(graph) = self.engine.graph.as_mut() {
            if let Some(layer) = graph.get_mut(result) {
                layer.kind = LayerKind::Shape;
                layer.shape = Some(preserved.clone());
            }
        }
        let combined = match Self::shape_pixels(&preserved, w, h) {
            Ok(p) => p,
            Err(_error) => {
                // Fallback: pure raster boolean bake.
                let pixels_a = match Self::shape_pixels(&content_a, w, h) {
                    Ok(p) => p,
                    Err(e) => {
                        self.report_gpu("shape boolean", &e);
                        return;
                    }
                };
                let pixels_b = match Self::shape_pixels(&content_b, w, h) {
                    Ok(p) => p,
                    Err(e) => {
                        self.report_gpu("shape boolean", &e);
                        return;
                    }
                };
                match phototux_engine::boolean_rgba8(&pixels_a, &pixels_b, op) {
                    Ok(p) => {
                        self.status_text = format!(
                            "Boolean {} (raster bake; vector path unavailable)",
                            op.as_str()
                        );
                        self.status_text_changed();
                        p
                    }
                    Err(e) => {
                        self.report_gpu("shape boolean", &e);
                        return;
                    }
                }
            }
        };
        if let Err(error) = phototux_canvas::write_layer_rgba(result, &combined) {
            self.report_gpu("shape boolean upload", &error);
            return;
        }
        self.recomposite();
        if !self.status_text.contains("raster bake") {
            self.status_text = format!("Boolean {} (vector-preserving)", op.as_str());
            self.status_text_changed();
        }
    }

    /// Re-upload shape layer pixels after path edit (keeps `LayerKind::Shape`).
    fn rasterize_shape_layer_id(&mut self, id: LayerId) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let Some(layer) = graph.get(id) else {
            return;
        };
        let Some(content) = layer.shape.clone() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let pixels = match Self::shape_pixels(&content, w, h) {
            Ok(p) => p,
            Err(error) => {
                self.report_gpu("shape path upload", &error);
                return;
            }
        };
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
            self.report_gpu("shape path upload", &error);
            return;
        }
        self.recomposite();
    }

    /// Bake active shape layer to raster (clears shape payload).
    #[qslot]
    fn rasterize_shape_layer(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let Some(layer) = graph.get(id) else {
            return;
        };
        if layer.kind != LayerKind::Shape {
            self.status_text = "Rasterize Shape requires an active shape layer".to_owned();
            self.status_text_changed();
            return;
        }
        let Some(content) = layer.shape.clone() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let pixels = match Self::shape_pixels(&content, w, h) {
            Ok(p) => p,
            Err(error) => {
                self.report_gpu("rasterize shape", &error);
                return;
            }
        };
        if let Err(error) = self.invoke_command(command_id::SHAPE_RASTERIZE, CommandArgs::None) {
            self.report_gpu("rasterize shape", &error.to_string());
            return;
        }
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
            self.report_gpu("rasterize shape upload", &error);
            return;
        }
        self.recomposite();
        self.status_text = "Shape rasterized to pixels".to_owned();
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
        let radius_u = radius as u32;
        let next = match op.as_str() {
            "feather" => feather_mask_r8(w, h, &mask, radius_u),
            "expand" => expand_mask_r8(w, h, &mask, radius_u),
            "contract" => contract_mask_r8(w, h, &mask, radius_u),
            _ => {
                self.status_text = format!("Unknown selection op: {op}");
                self.status_text_changed();
                return;
            }
        };
        match next {
            Ok(bytes) => {
                self.push_selection_snapshot();
                if let Err(error) = phototux_canvas::selection_restore(&bytes) {
                    self.report_gpu("modify selection", &error);
                    return;
                }
                let _ = self.invoke_command(
                    command_id::SELECTION_MODIFY,
                    CommandArgs::SelectionModify {
                        op: op.clone(),
                        radius: radius_u,
                    },
                );
                self.status_text = format!("Selection {op} ({radius_u}px)");
                self.status_text_changed();
            }
            Err(error) => self.report_gpu("modify selection", &error),
        }
    }

    #[qslot]
    fn add_drop_shadow_style(&mut self) {
        if let Err(error) =
            self.invoke_command(command_id::STYLE_ADD_DROP_SHADOW, CommandArgs::None)
        {
            self.status_text = error.to_string();
            self.status_text_changed();
            return;
        }
        self.status_text = "Drop Shadow style added".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn add_stroke_style(&mut self) {
        if let Err(error) = self.invoke_command(command_id::STYLE_ADD_STROKE, CommandArgs::None) {
            self.status_text = error.to_string();
            self.status_text_changed();
            return;
        }
        self.status_text = "Stroke style added".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn add_outer_glow_style(&mut self) {
        if let Err(error) = self.invoke_command(command_id::STYLE_ADD_OUTER_GLOW, CommandArgs::None)
        {
            self.status_text = error.to_string();
            self.status_text_changed();
            return;
        }
        self.status_text = "Outer Glow style added".to_owned();
        self.status_text_changed();
    }

    #[qslot]
    fn add_color_overlay_style(&mut self) {
        if let Err(error) =
            self.invoke_command(command_id::STYLE_ADD_COLOR_OVERLAY, CommandArgs::None)
        {
            self.status_text = error.to_string();
            self.status_text_changed();
            return;
        }
        self.status_text = "Color Overlay style added".to_owned();
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
        match self.invoke_command(
            command_id::PATH_STROKE_TO_LAYER,
            CommandArgs::PathStroke { layer_name },
        ) {
            Ok(()) => {
                let Some(id) = self.engine.graph.as_ref().and_then(|g| g.active_id()) else {
                    return;
                };
                if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
                    self.report_gpu("stroke path upload", &error);
                    return;
                }
                self.recomposite();
                self.status_text = "Path stroked to new layer".to_owned();
                self.status_text_changed();
            }
            Err(error) => self.report_gpu("stroke path", &error.to_string()),
        }
    }

    #[qslot]
    fn add_adjustment_layer(&mut self, kind: String) {
        if let Err(error) = self.invoke_command(
            command_id::FILTER_ADD_ADJUSTMENT,
            CommandArgs::FilterAdjustment { kind },
        ) {
            self.report_gpu("add adjustment", &error.to_string());
        }
    }

    #[qslot]
    fn set_adjustment_params(&mut self, p0: f32, p1: f32, p2: f32) {
        let _ = self.invoke_command(
            command_id::FILTER_SET_PARAMETERS,
            CommandArgs::FilterParameters { p0, p1, p2 },
        );
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
        if let Err(error) = self.invoke_command(
            command_id::FILTER_ADD_EFFECT,
            CommandArgs::FilterEffect {
                kind: kind.to_owned(),
            },
        ) {
            self.status_text = error.to_string();
            self.status_text_changed();
        }
    }

    #[qslot]
    fn reorder_active_effect(&mut self, effect_id: i64, to_index: i32) {
        if effect_id < 0 {
            return;
        }
        let _ = self.invoke_command(
            command_id::EFFECT_REORDER,
            CommandArgs::EffectReorder {
                effect_id: effect_id as u64,
                to_index,
            },
        );
    }

    #[qslot]
    fn set_active_effect_enabled(&mut self, effect_id: i64, enabled: bool) {
        if effect_id < 0 {
            return;
        }
        let _ = self.invoke_command(
            command_id::EFFECT_SET_ENABLED,
            CommandArgs::EffectSetEnabled {
                effect_id: effect_id as u64,
                enabled,
            },
        );
    }

    #[qslot]
    fn set_gaussian_radius(&mut self, radius: f32) {
        let _ = self.invoke_command(
            command_id::FILTER_SET_GAUSSIAN_RADIUS,
            CommandArgs::FilterGaussianRadius { radius },
        );
    }

    fn apply_selection_to_mask_host(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let r8 = match phototux_canvas::selection_snapshot() {
            Ok(bytes) => bytes,
            Err(error) => {
                self.report_gpu("selection to mask", &error);
                return;
            }
        };
        let needs_create = self
            .engine
            .graph
            .as_ref()
            .and_then(|graph| graph.get(id))
            .is_some_and(|layer| layer.mask.is_none());
        if needs_create {
            if let Err(error) = phototux_canvas::ensure_mask(id) {
                self.report_gpu("selection to mask", &error);
                return;
            }
            if let Err(error) = self.invoke_command(command_id::MASK_CREATE, CommandArgs::None) {
                self.report_gpu("selection to mask", &error.to_string());
                return;
            }
        }
        if let Err(error) = phototux_canvas::write_mask_r8(id, &r8) {
            self.report_gpu("selection to mask", &error);
            return;
        }
        self.engine.announce("Selection copied to layer mask");
        self.mark_dirty();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    fn apply_mask_host(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let Some(mask_meta) = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .and_then(|l| l.mask.clone())
        else {
            return;
        };
        let (_w, _h, mut rgba) = match phototux_canvas::read_layer_rgba(id) {
            Ok(v) => v,
            Err(error) => {
                self.report_gpu("apply mask", &error);
                return;
            }
        };
        let r8 = match phototux_canvas::read_mask_r8(id) {
            Ok(v) => v,
            Err(error) => {
                self.report_gpu("apply mask", &error);
                return;
            }
        };
        if r8.len() * 4 != rgba.len() {
            self.report_gpu("apply mask", "mask/layer size mismatch");
            return;
        }
        let density = mask_meta.density.clamp(0.0, 1.0);
        for (px, &m) in rgba.chunks_exact_mut(4).zip(r8.iter()) {
            let mut mf = m as f32 / 255.0;
            if mask_meta.inverted {
                mf = 1.0 - mf;
            }
            mf = 1.0 - density * (1.0 - mf);
            let a = (px[3] as f32 / 255.0) * mf;
            px[3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &rgba) {
            self.report_gpu("apply mask", &error);
            return;
        }
        if let Err(error) = self.invoke_command(command_id::MASK_DELETE, CommandArgs::None) {
            self.report_gpu("apply mask clear", &error.to_string());
            return;
        }
        let generation = self.engine.document_generation();
        self.engine
            .history
            .push_stroke("Apply layer mask", generation);
        self.engine.announce("Applied layer mask");
        self.mark_dirty();
        self.recomposite();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    fn apply_mask_to_selection_host(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let r8 = match phototux_canvas::read_mask_r8(id) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.report_gpu("mask to selection", &error);
                return;
            }
        };
        if let Err(error) = phototux_canvas::selection_restore(&r8) {
            self.report_gpu("mask to selection", &error);
            return;
        }
        let bounds = phototux_engine::SelectionRect {
            x: 0,
            y: 0,
            width: self.engine.size.width,
            height: self.engine.size.height,
        };
        self.engine
            .selection
            .set_mask_polygon(bounds, phototux_engine::SelectionCombine::Replace);
        self.engine.announce("Layer mask copied to pixel selection");
        self.push_selection_snapshot();
        self.sync_selection_fields();
        self.emit_selection_fields();
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
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
        if let Err(error) = self.invoke_command(command_id::MASK_CREATE, CommandArgs::None) {
            self.report_gpu("add mask", &error.to_string());
        }
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
        if let Err(error) = self.invoke_command(command_id::MASK_DELETE, CommandArgs::None) {
            self.report_gpu("delete mask", &error.to_string());
        }
    }

    #[qslot]
    fn set_mask_attributes_on_active(
        &mut self,
        density: f32,
        feather: f32,
        inverted: bool,
        linked: bool,
        contrast: f32,
        shift: f32,
    ) {
        let enabled = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .and_then(|l| l.mask.as_ref())
            .map(|m| m.enabled)
            .unwrap_or(true);
        let _ = self.invoke_command(
            command_id::MASK_SET_ATTRIBUTES,
            CommandArgs::MaskAttributes {
                enabled,
                linked,
                density,
                feather,
                inverted,
                contrast,
                shift,
            },
        );
    }

    #[qslot]
    fn set_mask_enabled_on_active(&mut self, enabled: bool) {
        let _ = self.invoke_command(
            command_id::MASK_SET_ENABLED,
            CommandArgs::MaskSetEnabled { enabled },
        );
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
        self.mask_density_changed();
        self.mask_feather_changed();
        self.mask_contrast_changed();
        self.mask_shift_changed();
        self.mask_inverted_changed();
        self.mask_linked_changed();
        self.edit_target_changed();
        self.edit_target_label_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn set_clips_to_below_on_active(&mut self, clips: bool) {
        let _ = self.invoke_command(
            command_id::LAYER_SET_CLIP,
            CommandArgs::LayerSetClip { clips },
        );
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
        self.push_active_text(
            body,
            font_family,
            font_size,
            tracking,
            line_spacing,
            alignment,
            color_hex,
            self.text_frame_w,
            self.text_frame_h,
            self.text_wrap,
        );
    }

    #[qslot]
    fn update_active_text_frame(&mut self, frame_w: f32, frame_h: f32, wrap: bool) {
        self.push_active_text(
            self.text_body.clone(),
            self.text_font_family.clone(),
            self.text_font_size,
            self.text_tracking,
            self.text_line_spacing,
            self.text_alignment,
            self.text_color_hex.clone(),
            frame_w,
            frame_h,
            wrap,
        );
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "QML Character panel maps each field; packed struct would fight qtbridge slots"
    )]
    fn push_active_text(
        &mut self,
        body: String,
        font_family: String,
        font_size: f32,
        tracking: f32,
        line_spacing: f32,
        alignment: i32,
        color_hex: String,
        frame_w: f32,
        frame_h: f32,
        wrap: bool,
    ) {
        let rgba =
            phototux_engine::ColorState::from_hex(&color_hex).unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let content = TextContent {
            text: body,
            font_family,
            font_size_pt: font_size.clamp(4.0, 512.0),
            color_rgba: rgba,
            alignment: u8::try_from(alignment.clamp(0, 2)).unwrap_or(0),
            tracking,
            line_spacing: line_spacing.clamp(0.5, 4.0),
            frame_w: frame_w.max(0.0),
            frame_h: frame_h.max(0.0),
            wrap,
        };
        if self
            .invoke_command(
                command_id::TEXT_SET_CONTENT,
                CommandArgs::TextSetContent { content },
            )
            .is_ok()
        {
            self.sync_text_fields();
            self.emit_text_fields();
        }
    }

    #[qslot]
    fn path_set_closed(&mut self, closed: bool) {
        let _ = self.invoke_command(
            command_id::PATH_SET_CLOSED,
            CommandArgs::PathSetClosed { closed },
        );
        self.sync_path_edit_fields();
        self.emit_path_edit_fields();
    }

    #[qslot]
    fn path_move_anchor(&mut self, index: i32, x: f32, y: f32) {
        if index < 0 {
            return;
        }
        let _ = self.invoke_command(
            command_id::PATH_MOVE_ANCHOR,
            CommandArgs::PathMoveAnchor {
                index: index as usize,
                x,
                y,
            },
        );
        self.sync_path_edit_fields();
        self.emit_path_edit_fields();
    }

    #[qslot]
    fn path_add_anchor(&mut self, x: f32, y: f32) {
        let _ = self.invoke_command(
            command_id::PATH_ADD_ANCHOR,
            CommandArgs::PathAddAnchor { x, y, index: None },
        );
        self.sync_path_edit_fields();
        self.emit_path_edit_fields();
    }

    #[qslot]
    fn path_delete_selected_anchor(&mut self) {
        let index = self.path_edit_selected;
        if index < 0 {
            return;
        }
        let _ = self.invoke_command(
            command_id::PATH_DELETE_ANCHOR,
            CommandArgs::PathDeleteAnchor {
                index: index as usize,
            },
        );
        self.sync_path_edit_fields();
        self.emit_path_edit_fields();
    }

    /// Hit-test path anchors; returns index or -1.
    #[qslot]
    fn path_hit_test(&mut self, doc_x: f32, doc_y: f32) -> i32 {
        const HIT_RADIUS: f32 = 8.0;
        let Some(graph) = self.engine.graph.as_ref() else {
            return -1;
        };
        let path = graph
            .active_id()
            .and_then(|id| {
                let layer = graph.get(id)?;
                if layer.kind == LayerKind::Shape {
                    return layer.shape.as_ref().map(|s| &s.path);
                }
                None
            })
            .or_else(|| {
                let idx = graph.paths.active?;
                graph.paths.paths.get(idx)
            });
        let Some(path) = path else {
            return -1;
        };
        let mut best = None;
        for (i, anchor) in path.anchors.iter().enumerate() {
            let dx = anchor.x - doc_x;
            let dy = anchor.y - doc_y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= HIT_RADIUS {
                best = Some(match best {
                    Some((bi, bd)) if bd <= dist => (bi, bd),
                    _ => (i, dist),
                });
            }
        }
        match best {
            Some((i, _)) => {
                self.engine.path_edit_anchor = Some(i);
                self.path_edit_selected = i32::try_from(i).unwrap_or(-1);
                self.path_edit_selected_changed();
                self.path_edit_selected
            }
            None => -1,
        }
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
        let Some(session) = self.engine.transform_session.clone() else {
            return;
        };
        if session.draft.is_identity() {
            self.engine.transform_session = None;
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
                self.engine.set_composite_ms(ms);
                let _ = self.invoke_command(command_id::RASTER_TRANSFORM_COMMIT, CommandArgs::None);
            }
            Err(error) => {
                self.report_gpu("transform bake", &error);
            }
        }
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
        self.status_text_changed();
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
                self.engine.set_composite_ms(ms);
                self.crop_preview_active = false;
                self.clear_selection_stacks();
                let _ = self.invoke_command(
                    command_id::DOCUMENT_CROP,
                    CommandArgs::DocumentCrop {
                        width: new_size.width,
                        height: new_size.height,
                    },
                );
            }
            Err(error) => self.report_gpu("crop", &error),
        }
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
                let _ = self.invoke_command(
                    command_id::RASTER_FLIP,
                    CommandArgs::RasterFlip { horizontal },
                );
            }
            Err(error) => self.report_gpu("flip", &error),
        }
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
            Ok((_new_size, ms)) => {
                self.engine.set_composite_ms(ms);
                self.clear_selection_stacks();
                let _ = self.invoke_command(command_id::DOCUMENT_ROTATE_90, CommandArgs::None);
            }
            Err(error) => self.report_gpu("rotate canvas", &error),
        }
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
                let _ = self.invoke_command(command_id::RASTER_FILL, CommandArgs::None);
            }
            Err(error) => self.report_gpu("fill", &error),
        }
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
                let _ = self.invoke_command(command_id::RASTER_GRADIENT, CommandArgs::None);
            }
            Err(error) => self.report_gpu("gradient", &error),
        }
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
