//! QML-facing session via qtbridge (ADR-003). Package name `phototux_ui` → `import phototux_ui`.

mod chrome_contract;
mod clipboard;
mod data_url;
mod display_icc;
mod file_worker;
mod fonts;
mod history_model;
mod layer_model;
mod prefs;
mod selection_path;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

use file_worker::{FileCommand, FileEvent, FileWorker};
use phototux_canvas::PaintWorker;
use phototux_engine::{
    AlignOp, AlignTarget, CommandArgs, CommandEffects, CommandError, CropRect, DabMode,
    DocumentGraph, DocumentRegistry, DocumentSize, EngineCommand, EngineEvent, FilterParams,
    GradientKind, GradientRamp, Guide, GuideOrientation, HistoryKind, HostFollowUp,
    HostHistoryAction, Layer, LayerId, LayerKind, LayerTransform, NoticeLevel, OpenDocumentId,
    PathPoint, SelectionCombine, SelectionModifyOp, SelectionRect, SelectionShape, SelectionState,
    SessionState, ShapeBooleanPartner, ShapePreset, TextContent, TransformSession, VectorPath,
    WorkspaceState, bake_text_rgba8, command_id, parse_selection_modify_arg, stroke_path_rgba8,
    tool_id,
};
use prefs::Preferences;
use selection_path::PathVerdict;

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
    CompatibilityIssue, PtxDocument, Raster, RasterFormat, discard_recovery, format_report,
    list_recoverable, load_recovery,
};
use qtbridge::qobject;

static PROCESS_START: OnceLock<Instant> = OnceLock::new();

pub fn mark_process_started() {
    let _ = PROCESS_START.set(Instant::now());
}

fn resolve_icon_root() -> String {
    "qrc:/qt/qml/PhotoTux/App/icons".to_owned()
}

/// Recoverable snapshots as the chooser reads them.
///
/// A projection rather than the stored `RecoveryEntry` itself: the chooser
/// needs a label a person can tell apart, and the stored entry's fields are
/// the ones the *restore* path needs. It used to publish the raw entry, and the
/// chooser built each row's name from the first eight characters of
/// `document_id` — which are structurally always zero, so every row read
/// "Untitled (00000000)".
fn recovery_entries_view_json(entries: &[phototux_io::RecoveryEntry]) -> String {
    let rows: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.document_id,
                "shortId": entry.short_id(),
                "path": entry.original_path,
                "savedMs": entry.saved_unix_ms as f64,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

/// The active layer's blend ranges, as the Properties panel reads them.
///
/// Stops are published as `0..=255` because that is the scale on Photoshop's
/// Blend If sliders and the one a user reads a value back in; the engine keeps
/// them normalised, and this is the only place the two meet.
fn blend_if_state_json(blend_if: phototux_engine::BlendIf) -> String {
    let stops = |range: phototux_engine::BlendRange| {
        range
            .stops()
            .iter()
            .map(|v| (v * 255.0).round())
            .collect::<Vec<f32>>()
    };
    serde_json::json!({
        "channel": blend_if.channel.as_str(),
        "active": !blend_if.is_identity(),
        "labels": phototux_engine::BlendRange::STOP_LABELS,
        "thisLayer": stops(blend_if.this_layer),
        "underlying": stops(blend_if.underlying),
    })
    .to_string()
}

/// The active shape layer's appearance and geometry, or `{}` for anything else.
///
/// Hex for the two colours, because that is the form every colour field in the
/// shell edits. The bounds come from the path's anchors, which is arithmetic
/// on data the engine already holds — unlike a raster layer's extent, which is
/// a GPU readback and could not be published on every sync.
fn shape_state_json(shape: Option<&phototux_engine::ShapeContent>) -> String {
    let Some(shape) = shape else {
        return "{}".into();
    };
    let appearance = shape.appearance();
    let bounds = shape.path.bounds();
    serde_json::json!({
        "kind": shape.kind,
        "fill": phototux_engine::ColorState::to_hex(appearance.fill_rgba),
        "stroke": phototux_engine::ColorState::to_hex(appearance.stroke_rgba),
        "width": appearance.stroke_width,
        "maxWidth": phototux_engine::ShapeAppearance::MAX_STROKE_WIDTH,
        "filled": appearance.filled,
        "stroked": appearance.stroked,
        "invisible": appearance.is_invisible(),
        "anchors": shape.path.anchors.len(),
        "x": bounds.map_or(0.0, |b| b.x),
        "y": bounds.map_or(0.0, |b| b.y),
        "w": bounds.map_or(0.0, |b| b.width),
        "h": bounds.map_or(0.0, |b| b.height),
        "hasBounds": bounds.is_some(),
    })
    .to_string()
}

/// The active smart object as the panel needs it, or `{}` for anything else.
///
/// `hasSource` is the honest half: a document saved by a build that predates
/// smart objects, or one whose `SRCE` chunk something dropped, still opens —
/// showing the pixels it already had, which are the placed result — but cannot
/// be re-placed, and the panel has to say so rather than offering a control
/// that would fail.
fn smart_state_json(
    smart: Option<&phototux_engine::SmartObjectContent>,
    has_source: bool,
) -> String {
    let Some(smart) = smart else {
        return "{}".into();
    };
    let placement = smart.placement;
    serde_json::json!({
        "sourceName": smart.source_name,
        "sourceWidth": smart.source_width,
        "sourceHeight": smart.source_height,
        "hasSource": has_source,
        "placed": smart.is_placed(),
        "x": placement.translate_x,
        "y": placement.translate_y,
        "scale": placement.scale_x,
        "rotation": placement.rotation_deg,
    })
    .to_string()
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

/// Assign a projected value and notify QML only when it actually moved.
///
/// Computing a value and announcing it were two hand-maintained lists, so a
/// guard could be — and was — added to one half and not the other: the
/// accessibility tree was rebuilt only on change, then its notify fired on
/// every layer edit regardless, which is how T-009 flooded AT-SPI until the
/// session died. Fusing them means the guard cannot be half-applied.
///
/// Evaluates to `true` when the value changed, so dependent projections can be
/// recomputed only when their input moved.
macro_rules! publish {
    ($self:expr, $field:ident, $next:expr, $notify:ident) => {{
        let next = $next;
        if $self.$field == next {
            false
        } else {
            $self.$field = next;
            $self.$notify();
            true
        }
    }};
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
    notices_json: String,
    notices: phototux_engine::NoticeQueue,
    navigator_thumbnail: String,
    navigator_thumbnail_at: Option<Instant>,
    navigator_thumbnail_generation: u64,
    active_tool: String,
    has_document: bool,
    layer_count: i32,
    active_layer_index: i32,
    can_undo: bool,
    can_redo: bool,
    /// The layers panel's rows. Owned here and handed to QML as a property;
    /// see `layer_model` for why this replaced six index-aligned strings.
    layer_model: std::rc::Rc<std::cell::RefCell<crate::layer_model::LayerListModel>>,
    /// Mask state of the active layer only: 0 none, 1 enabled, 2 disabled.
    active_mask_flag: i32,
    /// Whether the active layer clips to the one below it.
    active_layer_clips: bool,
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
    active_layer_name: String,
    selected_layer_count: i32,
    inspector_subject: String,
    inspector_subjects_json: String,
    /// The history panel's rows. See `history_model` for why this replaced
    /// three index-aligned strings.
    history_model: std::rc::Rc<std::cell::RefCell<crate::history_model::HistoryListModel>>,
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
    /// Magic wand / colour range tolerance, 0..=1.
    selection_tolerance: f32,
    /// Wire name of the gradient shape the tool sweeps.
    gradient_kind: String,
    /// The gradient shapes, for the tool options.
    gradient_kinds_json: String,
    align_ops_json: String,
    tool_slots_json: String,
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
    /// Inspector disclosure group id → expanded, as a JSON object.
    disclosure_open_json: String,
    /// Static [`phototux_engine::DisclosureGroupDescriptor`] list for the inspector.
    disclosure_groups_json: String,
    /// Monotonic count of published composites. The canvas repaints when this
    /// moves, which is what lets its clock stop when nothing is changing.
    composite_generation: i32,
    /// Group id → `{text, severity}` for values a collapsed group would hide.
    inspector_badges_json: String,
    /// Static adjustment slider bounds, so QML and the badge rule agree.
    adjustment_ranges_json: String,
    /// `{kind: label}` for the adjustment editor heading.
    adjustment_labels_json: String,
    /// Filter kinds, labels and editor slots for the gallery chrome.
    filter_catalog_json: String,
    /// The active layer's styles and their editor descriptors.
    layer_styles_json: String,
    blend_if_json: String,
    shape_json: String,
    smart_json: String,
    blend_if_channels_json: String,
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
    /// Slot values of the active adjustment as a JSON array, index-aligned
    /// with the kind's editor slots. An array rather than a fixed set of
    /// scalars because the slot count is a property of the kind — Levels has
    /// five, Invert has none.
    adjustment_slots_json: String,
    /// Slot values behind [`Self::adjustment_slots_json`], for the badge rules.
    adjustment_slots: Vec<f32>,
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
    /// Layer id → a smart object's pristine source pixels (DR-032).
    ///
    /// Held here rather than in the engine, which describes documents and owns
    /// no pixel buffers, and rather than on the GPU, which would mean a second
    /// texture per layer and a new canvas API. A placement restores this and
    /// re-applies the whole transform, so nothing is ever composed twice —
    /// which is the entire behaviour that separates a smart object from a
    /// layer someone transformed.
    smart_sources: HashMap<LayerId, phototux_engine::SmartSource>,
    /// Inactive open documents (active session is [`Self::engine`]).
    doc_registry: DocumentRegistry,
    active_doc_id: Option<OpenDocumentId>,
    document_tabs_json: String,
    worker: PaintWorker,
    file_worker: FileWorker,
    /// RGBA image clipboard (app-local; may also push OS image).
    ///
    /// Typed rather than a bare tuple so a buffer that does not match its own
    /// dimensions cannot be stored — see `clipboard::ImagePayload`.
    clipboard_rgba: Option<crate::clipboard::ImagePayload>,
    /// Selection coverage clipboard (R8, document-sized).
    clipboard_selection_r8: Option<crate::clipboard::CoveragePayload>,
    /// Layer mask clipboard (R8, document-sized).
    clipboard_mask_r8: Option<crate::clipboard::CoveragePayload>,
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
    /// Right dock as `[{tabs: [...], active: "..."}]`, derived so QML never
    /// re-implements the grouping rule.
    dock_groups_json: String,
    /// Pending shell capability request; see `phototux_engine::HostRequest`.
    pending_host_request: String,
    panel_visibility_json: String,
    tool_descriptors_json: String,
    /// Blend-mode vocabulary for the Properties combo; see
    /// [`phototux_engine::blend_modes_json`].
    blend_modes_json: String,
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
    text_layer_active: bool,
    text_body: String,
    text_font_family: String,
    /// JSON array of font family names: fallbacks until `fc-list` has run.
    available_fonts_json: String,
    /// Same list unserialized, for the missing-font disclosure badge.
    available_font_families: Vec<String>,
    /// True once fontconfig discovery has replaced the fallback list.
    fonts_discovered: bool,
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
    /// The active layer's style at `index`, if there is one.
    fn active_layer_style(&self, index: usize) -> Option<phototux_engine::LayerStyle> {
        self.engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .and_then(|layer| layer.styles.get(index).copied())
    }

    /// Store the active adjustment's slot values and publish them.
    ///
    /// The vector feeds the inspector badge rules and the JSON feeds QML; they
    /// are set together so a badge can never describe a value the panel is not
    /// showing.
    fn set_adjustment_slot_values(&mut self, values: &[f32]) {
        if self.adjustment_slots == values {
            return;
        }
        self.adjustment_slots = values.to_vec();
        self.adjustment_slots_json =
            serde_json::to_string(values).unwrap_or_else(|_| "[]".to_owned());
        self.adjustment_slots_json_changed();
    }

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
            notices_json: "[]".into(),
            notices: phototux_engine::NoticeQueue::default(),
            navigator_thumbnail: String::new(),
            navigator_thumbnail_at: None,
            navigator_thumbnail_generation: u64::MAX,
            active_tool: engine.active_tool.clone(),
            has_document: engine.has_document,
            layer_count: 0,
            active_layer_index: -1,
            can_undo: false,
            can_redo: false,
            layer_model: <crate::layer_model::LayerListModel as qtbridge::QObjectHolder>::default_with_attached_qobject(),
            active_mask_flag: 0,
            active_layer_clips: false,
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
            active_layer_name: String::new(),
            selected_layer_count: 0,
            inspector_subject: String::new(),
            inspector_subjects_json: phototux_engine::inspector_subjects_json(),
            history_model: <crate::history_model::HistoryListModel as qtbridge::QObjectHolder>::default_with_attached_qobject(),
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
            selection_tolerance: 0.15,
            gradient_kind: GradientKind::default().as_str().to_owned(),
            gradient_kinds_json: serde_json::to_string(
                &GradientKind::ALL
                    .iter()
                    .map(|k| {
                        serde_json::json!({
                            "id": k.as_str(),
                            "label": k.label(),
                            "icon": k.icon_key(),
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".into()),
            align_ops_json: phototux_engine::align_ops_json(),
            tool_slots_json: phototux_engine::tool_slots_json(),
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
            disclosure_open_json: "{}".to_owned(),
            disclosure_groups_json: phototux_engine::disclosure_groups_json(),
            composite_generation: 0,
            inspector_badges_json: "{}".to_owned(),
            adjustment_ranges_json: phototux_engine::adjustment_editor_ranges_json(),
            adjustment_labels_json: phototux_engine::adjustment_labels_json(),
            filter_catalog_json: phototux_engine::filter_catalog_json(),
            layer_styles_json: "[]".into(),
            blend_if_json: "{}".into(),
            shape_json: "{}".into(),
            smart_json: "{}".into(),
            blend_if_channels_json: phototux_engine::blend_if_channels_json(),
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
            adjustment_slots_json: "[]".into(),
            adjustment_slots: Vec::new(),
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
            smart_sources: HashMap::new(),
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
            dock_groups_json: "[]".to_owned(),
            dock_topology_json: phototux_engine::DockTopology::essentials()
                .to_json()
                .unwrap_or_else(|_| "{}".into()),
            pending_host_request: String::new(),
            panel_visibility_json: WorkspaceState::essentials().visibility_json(),
            tool_descriptors_json: phototux_engine::tools_json(),
            blend_modes_json: phototux_engine::blend_modes_json(),
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
            text_layer_active: false,
            text_body: String::new(),
            text_font_family: "Noto Sans".into(),
            available_fonts_json: fonts::fallback_font_families_json(),
            available_font_families: Vec::new(),
            fonts_discovered: false,
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
        let parts = document.into_parts();
        let graph = parts.graph;
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        // Smart-object sources belong to the document being opened, so the
        // ones from whatever was open before must not survive into it.
        self.smart_sources.clear();
        for (id, source) in parts.sources {
            self.smart_sources.insert(
                LayerId(id),
                phototux_engine::SmartSource {
                    width: source.width(),
                    height: source.height(),
                    pixels: source.pixels().to_vec(),
                },
            );
        }
        match phototux_canvas::open_document(graph.size, graph.layers()) {
            Ok(ms) => {
                self.finish_opened_ptx(path, title, graph, parts.rasters, parts.masks, ms);
            }
            Err(error) => self.fail_io("Open", &error),
        }
    }

    fn finish_opened_ptx(
        &mut self,
        path: PathBuf,
        title: String,
        graph: DocumentGraph,
        rasters: HashMap<u64, Raster>,
        masks: HashMap<u64, Raster>,
        ms: f32,
    ) {
        for (id, raster) in rasters {
            if let Err(error) = phototux_canvas::write_layer_rgba(LayerId(id), raster.pixels()) {
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
        self.record_composite(ms);
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

    /// Surface a device or I/O failure. Takes a `String` because that is what
    /// the canvas returns; a *command* failure has a typed error and belongs in
    /// [`Self::report_action_error`], which does not run the device-lost check
    /// on text that can never name a device.
    fn report_gpu(&mut self, operation: &str, error: &str) {
        let lower = error.to_ascii_lowercase();
        if lower.contains("device lost")
            || lower.contains("surface lost")
            || phototux_canvas::gpu_is_lost()
        {
            self.enter_gpu_lost();
            return;
        }
        self.notify(NoticeLevel::Error, format!("{operation} failed: {error}"));
        eprintln!("[phototux] {operation}: {error}");
    }

    /// Post a message to the toast channel.
    ///
    /// Not the status bar. That carries the document summary — size, zoom,
    /// active layer, tool — which is state, always true and always there. A
    /// message is an event, true once, and putting the two in one string meant
    /// the next summary refresh silently erased whatever the user had not yet
    /// read.
    fn notify(&mut self, level: NoticeLevel, text: impl Into<String>) {
        self.notices.post(level, text);
        self.publish_notices();
    }

    fn publish_notices(&mut self) {
        let json = self.notices.to_json();
        publish!(self, notices_json, json, notices_json_changed);
    }

    fn enter_gpu_lost(&mut self) {
        if !self.gpu_lost {
            self.gpu_lost = true;
            self.gpu_lost_changed();
            self.refresh_inspector_badges();
        }
        self.engine
            .announce("Graphics device lost — document preserved");
        self.notify(
            NoticeLevel::Error,
            "Graphics device lost — the document is preserved. Use Recover to restore the canvas.",
        );
        self.publish_announcement();
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
        publish!(
            self,
            composite_ms,
            self.engine.composite_ms,
            composite_ms_changed
        );
        self.stroke_latency_ms = self.engine.stroke_latency_ms;
        publish!(
            self,
            dirty_rect_json,
            match self.engine.dirty_rect {
                Some([x, y, w, h]) => format!("[{x},{y},{w},{h}]"),
                None => String::new(),
            },
            dirty_rect_json_changed
        );
        publish!(
            self,
            overlay_view_generation,
            i32::try_from(self.engine.overlay_view_generation.min(i32::MAX as u64)).unwrap_or(0),
            overlay_view_generation_changed
        );
        self.active_tool = self.engine.active_tool.clone();
        self.has_document = self.engine.has_document;
        publish!(
            self,
            layer_count,
            self.engine.layer_count(),
            layer_count_changed
        );
        publish!(
            self,
            active_layer_index,
            self.engine.active_layer_index(),
            active_layer_index_changed
        );
        publish!(self, can_undo, self.engine.can_undo(), can_undo_changed);
        publish!(self, can_redo, self.engine.can_redo(), can_redo_changed);
        // The model notifies Qt itself, per row, so there is no property to
        // publish here — only rows to bring up to date.
        crate::layer_model::apply_rows(
            &self.layer_model,
            self.engine
                .layer_rows()
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        publish!(
            self,
            active_mask_flag,
            self.engine.active_mask_flag(),
            active_mask_flag_changed
        );
        publish!(
            self,
            active_layer_clips,
            self.engine.active_layer_clips(),
            active_layer_clips_changed
        );
        if self.engine.mask_edit_layer.is_some_and(|id| {
            self.engine
                .graph
                .as_ref()
                .and_then(|graph| graph.get(id))
                .is_none_or(|layer| layer.mask.is_none())
        }) {
            self.engine.mask_edit_layer = None;
        }
        publish!(
            self,
            mask_edit_active,
            matches!(
                self.engine.paint_target(),
                phototux_engine::PaintTarget::LayerMask
            ),
            mask_edit_active_changed
        );
        // Read the six mask fields as one tuple rather than assigning each in
        // both arms of an if/else. The no-mask defaults are the interesting
        // half — they are what the panel shows for an unmasked layer — and
        // stating them once beside the real values keeps the two arms honest.
        let (density, feather, inverted, linked, contrast, shift) = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .and_then(|l| l.mask.as_ref())
            .map_or((1.0, 0.0, false, true, 0.0, 0.0), |m| {
                (
                    m.density, m.feather, m.inverted, m.linked, m.contrast, m.shift,
                )
            });
        publish!(self, mask_density, density, mask_density_changed);
        publish!(self, mask_feather, feather, mask_feather_changed);
        publish!(self, mask_inverted, inverted, mask_inverted_changed);
        publish!(self, mask_linked, linked, mask_linked_changed);
        publish!(self, mask_contrast, contrast, mask_contrast_changed);
        publish!(self, mask_shift, shift, mask_shift_changed);
        publish!(
            self,
            edit_target,
            self.engine.edit_target_id().to_owned(),
            edit_target_changed
        );
        publish!(
            self,
            edit_target_label,
            self.engine.edit_target_label().to_owned(),
            edit_target_label_changed
        );
        publish!(
            self,
            active_layer_kind,
            self.engine.active_layer_kind(),
            active_layer_kind_changed
        );
        publish!(
            self,
            active_layer_name,
            self.engine.active_layer_name(),
            active_layer_name_changed
        );
        publish!(
            self,
            selected_layer_count,
            i32::try_from(self.engine.selected_layer_ids.len()).unwrap_or(i32::MAX),
            selected_layer_count_changed
        );
        // Only the id. The panel resolves the title and glyph through the
        // subject table, because the subject it is *showing* is not always the
        // one the selection reports — the document scope stays on the document
        // while a raster layer is active.
        publish!(
            self,
            inspector_subject,
            self.engine.inspector_subject().as_str().to_owned(),
            inspector_subject_changed
        );
        self.publish_history_projection();
        publish!(
            self,
            brush_preset_names,
            self.engine.brush_presets.names_joined(),
            brush_preset_names_changed
        );
        let (proof_profile, proof_active, embedded_icc) = self.engine.graph.as_ref().map_or_else(
            || (String::new(), false, false),
            |graph| {
                (
                    graph.color.soft_proof_profile.clone(),
                    graph.color.soft_proof_active(),
                    graph.color.has_embedded_icc(),
                )
            },
        );
        publish!(
            self,
            soft_proof_profile,
            proof_profile,
            soft_proof_profile_changed
        );
        publish!(
            self,
            soft_proof_active,
            proof_active,
            soft_proof_active_changed
        );
        publish!(
            self,
            has_embedded_icc,
            embedded_icc,
            has_embedded_icc_changed
        );
        if publish!(
            self,
            accessibility_tree_json,
            self.build_accessibility_tree_json(),
            accessibility_tree_json_changed
        ) {
            publish!(
                self,
                atspi_projection_json,
                phototux_engine::project_semantic_tree_json(&self.accessibility_tree_json),
                atspi_projection_json_changed
            );
        }
        self.sync_selection_fields();
        self.sync_transform_fields();
        publish!(
            self,
            document_path,
            self.engine.document_path.clone().unwrap_or_default(),
            document_path_changed
        );
        publish!(
            self,
            graph_revision,
            self.engine.graph_revision() as i32,
            graph_revision_changed
        );
        let active_layer = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)));
        let opacity = active_layer.map_or(1.0, |l| l.opacity);
        let blend =
            active_layer.map_or_else(|| "normal".to_owned(), |l| l.blend.as_str().to_owned());
        let fill_hex = active_layer.and_then(|l| l.fill.as_ref()).map_or_else(
            || "#738CBF".to_owned(),
            |f| phototux_engine::ColorState::to_hex(f.color_rgba),
        );
        publish!(self, active_opacity, opacity, active_opacity_changed);
        publish!(self, active_blend, blend, active_blend_changed);
        publish!(self, fill_color_hex, fill_hex, fill_color_hex_changed);
        self.sync_inspector_mixed_fields();
        self.sync_color_fields();
        publish!(
            self,
            viewport_width,
            self.engine.viewport_w,
            viewport_width_changed
        );
        publish!(
            self,
            viewport_height,
            self.engine.viewport_h,
            viewport_height_changed
        );
        self.sync_adjustment_fields();
        self.sync_text_fields();
        self.sync_path_edit_fields();
        self.sync_filter_preview_fields();
        self.sync_guides_fields();
        self.refresh_pref_effective_json();
        self.pref_effective_json_changed();
        self.refresh_inspector_badges();
        publish!(
            self,
            status_text,
            self.engine.status_summary(),
            status_text_changed
        );
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
            self.set_adjustment_slot_values(&[]);
            self.has_gaussian_blur = false;
            self.gaussian_radius = 0.0;
            let empty = blend_if_state_json(phototux_engine::BlendIf::default());
            publish!(self, blend_if_json, empty, blend_if_json_changed);
            let no_shape = shape_state_json(None);
            publish!(self, shape_json, no_shape, shape_json_changed);
            let no_smart = smart_state_json(None, false);
            publish!(self, smart_json, no_smart, smart_json_changed);
            return;
        };
        // Copied out before publishing: the layer is borrowed from `self`, and
        // the publisher needs `self` mutably.
        let adjustment = layer.adjustment.as_ref().map(|params| {
            (
                params.kind_key(),
                params.slots(),
                params.editor_slots().len(),
            )
        });
        let styles_json = phototux_engine::layer_styles_json(&layer.styles);
        let blend_if = blend_if_state_json(layer.blend_if);
        let shape = shape_state_json(layer.shape.as_ref());
        let smart = smart_state_json(
            layer.smart.as_ref(),
            self.smart_sources.contains_key(&layer.id),
        );
        publish!(self, blend_if_json, blend_if, blend_if_json_changed);
        publish!(self, shape_json, shape, shape_json_changed);
        publish!(self, smart_json, smart, smart_json_changed);
        publish!(
            self,
            layer_styles_json,
            styles_json,
            layer_styles_json_changed
        );
        match adjustment {
            Some((kind, slots, used)) => {
                self.adjustment_kind = kind.to_owned();
                self.set_adjustment_slot_values(&slots[..used]);
            }
            None => {
                self.adjustment_kind.clear();
                self.set_adjustment_slot_values(&[]);
            }
        }
        let layer = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)));
        let Some(layer) = layer else {
            return;
        };
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
            publish!(
                self,
                inspector_opacity_mixed,
                false,
                inspector_opacity_mixed_changed
            );
            publish!(
                self,
                inspector_blend_mixed,
                false,
                inspector_blend_mixed_changed
            );
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
        let opacity_mixed = phototux_engine::values_are_mixed(&opacities);
        let blend_mixed = phototux_engine::values_are_mixed(&blends);
        publish!(
            self,
            inspector_opacity_mixed,
            opacity_mixed,
            inspector_opacity_mixed_changed
        );
        publish!(
            self,
            inspector_blend_mixed,
            blend_mixed,
            inspector_blend_mixed_changed
        );
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

    /// Bring the history model up to date.
    ///
    /// The model decides for itself whether anything moved and tells Qt only
    /// what did, so callers no longer need to guess. Three index-aligned
    /// strings used to describe this one list, and both callers rebuilt all
    /// three regardless of which had changed.
    fn publish_history_projection(&mut self) {
        crate::history_model::apply_rows(
            &self.history_model,
            self.engine
                .history
                .rows_newest_first()
                .into_iter()
                .map(Into::into)
                .collect(),
        );
    }

    /// Announce the fields that `sync_from_engine` does not publish itself.
    ///
    /// This was a hand-maintained list of every layer-adjacent property,
    /// announced unconditionally on every edit — the twin of the assignment
    /// list in `sync_from_engine`, and free to drift from it. Thirty-one of
    /// them now publish where they are computed, so a value that did not move
    /// no longer wakes its bindings; dragging the opacity slider used to fire
    /// the whole list once per pointer sample.
    ///
    /// What remains is the properties whose value is not stored on the session
    /// — computed getters and sub-emitters — which have nothing to compare
    /// against and so must still announce blind.
    fn emit_layer_fields(&mut self) {
        self.sync_inspector_mixed_fields();
        self.emit_selection_fields();
        self.emit_transform_fields();
        self.foreground_hex_changed();
        self.background_hex_changed();
        self.recent_colors_changed();
        self.brush_color_changed();
        self.adjustment_kind_changed();
        self.adjustment_slots_json_changed();
        self.has_gaussian_blur_changed();
        self.gaussian_radius_changed();
        self.effects_joined_changed();
        self.emit_text_fields();
        self.emit_path_edit_fields();
        self.emit_filter_preview_fields();
        self.emit_guides_fields();
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
                self.record_composite(ms);
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

    /// Run a GPU selection edit with its undo snapshot taken first.
    ///
    /// The snapshot reads the GPU mask and `engine.selection` as they are
    /// *now*, so it has to be taken before the edit overwrites either. Every
    /// call site used to restate that ordering, and one of them —
    /// `apply_mask_to_selection_host` — had it inverted, so its snapshot
    /// captured the post-edit state and Ctrl+Z was a no-op. Taking it here
    /// makes the order a property of the operation instead.
    ///
    /// Returns whether the edit succeeded, so the caller can decide whether to
    /// record a command.
    fn commit_selection_edit<F>(&mut self, label: &str, run: F) -> bool
    where
        F: FnOnce() -> Result<(), String>,
    {
        self.push_selection_snapshot();
        match run() {
            Ok(()) => true,
            Err(error) => {
                self.report_gpu(label, &error);
                false
            }
        }
    }

    /// Run a GPU layer edit: snapshot, hand the op current layer metadata, then
    /// publish the composite time and the repaint together.
    ///
    /// The tail is the part that kept getting dropped — T-025 was a composite
    /// that published its time without asking the canvas to repaint. `run`
    /// receives the layer metadata the GPU needs and returns its own result
    /// plus the measured composite milliseconds.
    fn commit_layer_edit<T, F>(&mut self, label: &str, run: F) -> Option<T>
    where
        F: FnOnce(&[Layer]) -> Result<(T, f32), String>,
    {
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        match run(&layers) {
            Ok((value, ms)) => {
                self.record_composite(ms);
                Some(value)
            }
            Err(error) => {
                self.report_gpu(label, &error);
                None
            }
        }
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
        self.refresh_workspace_presets_json();
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

    fn refresh_workspace_presets_json(&mut self) {
        self.workspace_presets_json =
            phototux_engine::merged_workspace_presets_json(&self.prefs.user_workspace_presets_json);
    }

    fn emit_workspace_presets_json(&mut self) {
        self.workspace_presets_json_changed();
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

    /// Publish descriptor defaults merged with user overrides, so QML reads a
    /// resolved boolean per group instead of re-deriving the default.
    ///
    /// Overrides stay sparse in the store: a group the user never touched keeps
    /// following its descriptor default when that default changes.
    fn refresh_disclosure_open_json(&mut self) {
        let resolved: std::collections::BTreeMap<String, bool> =
            phototux_engine::default_disclosure_groups()
                .into_iter()
                .map(|group| {
                    let open = self
                        .prefs
                        .disclosure_is_open(&group.id, group.open_by_default);
                    (group.id, open)
                })
                .collect();
        self.disclosure_open_json =
            serde_json::to_string(&resolved).unwrap_or_else(|_| "{}".to_owned());
    }

    /// Project the dock's tab groups for QML.
    fn build_dock_groups_json(&self) -> String {
        let groups: Vec<serde_json::Value> = self
            .workspace
            .dock
            .right_groups()
            .into_iter()
            .map(|tabs| {
                let active = tabs
                    .first()
                    .and_then(|id| self.workspace.effective_active_tab(id))
                    .unwrap_or_default();
                serde_json::json!({ "tabs": tabs, "active": active })
            })
            .collect();
        serde_json::to_string(&groups).unwrap_or_else(|_| "[]".into())
    }

    /// Record a completed composite: publish its time and repaint the canvas.
    ///
    /// Both halves belong together. Repaint is driven by the generation
    /// counter, so a path that composites and reports the time without bumping
    /// it produces work the user never sees — which is exactly what happened to
    /// stroke undo once the canvas stopped repainting every frame.
    fn record_composite(&mut self, ms: f32) {
        self.engine.set_composite_ms(ms);
        self.bump_composite_generation();
    }

    /// A new composite has been published; ask the canvas to repaint.
    fn bump_composite_generation(&mut self) {
        self.composite_generation = self.composite_generation.wrapping_add(1);
        self.composite_generation_changed();
    }

    /// How often the Navigator's picture may be rebuilt.
    ///
    /// Rebuilding costs a full composite readback — thirty-odd megabytes at 4K
    /// — so it is deliberately slower than the eye. The Navigator answers
    /// "where am I in the image", a question whose answer does not change
    /// meaningfully between two frames of a brush stroke, and DR-017's budget
    /// protects the path this would otherwise sit on.
    const NAVIGATOR_THUMBNAIL_INTERVAL_MS: u128 = 600;
    /// Longest edge of the Navigator's picture, in pixels.
    ///
    /// The panel is under three hundred wide; more detail than this is paid for
    /// in readback and thrown away by the scaler.
    const NAVIGATOR_THUMBNAIL_EDGE: u32 = 200;

    /// Rebuild the Navigator's picture if enough has changed and enough time
    /// has passed.
    ///
    /// Both conditions, not either: the generation alone would rebuild on every
    /// dab of a stroke, and the clock alone would rebuild a document nobody is
    /// editing.
    fn refresh_navigator_thumbnail(&mut self) {
        // The canvas is asked, not `self.has_document`: that field is synced
        // *after* the composite that a new document triggers, so on the first
        // pass it still says there is nothing to draw.
        if !phototux_canvas::has_document() {
            if !self.navigator_thumbnail.is_empty() {
                self.navigator_thumbnail.clear();
                self.navigator_thumbnail_generation = u64::MAX;
                self.navigator_thumbnail_changed();
            }
            return;
        }
        let generation = u64::from(self.composite_generation.max(0).unsigned_abs());
        if generation == self.navigator_thumbnail_generation {
            return;
        }
        if let Some(last) = self.navigator_thumbnail_at
            && last.elapsed().as_millis() < Self::NAVIGATOR_THUMBNAIL_INTERVAL_MS
        {
            return;
        }
        self.navigator_thumbnail_at = Some(Instant::now());
        self.navigator_thumbnail_generation = generation;
        let Some(url) = Self::render_navigator_thumbnail() else {
            return;
        };
        publish!(self, navigator_thumbnail, url, navigator_thumbnail_changed);
    }

    /// Read the composite back, shrink it, and encode it as a `data:` URL.
    ///
    /// `None` on any failure: a Navigator that keeps its previous picture for a
    /// moment is better than one that blanks because a readback lost a race.
    fn render_navigator_thumbnail() -> Option<String> {
        let (width, height, pixels) = phototux_canvas::read_composite_rgba().ok()?;
        let thumb = phototux_engine::downsample_rgba8(
            &pixels,
            width,
            height,
            Self::NAVIGATOR_THUMBNAIL_EDGE,
        )?;
        let raster =
            phototux_io::Raster::new(thumb.width, thumb.height, thumb.pixels.into_boxed_slice())
                .ok()?;
        let mut png = Vec::new();
        phototux_io::encode(&mut png, &raster, phototux_io::RasterFormat::Png).ok()?;
        Some(crate::data_url::png_data_url(&png))
    }

    /// Recompute the header badges a collapsed inspector group would hide.
    ///
    /// Handbook 28 requires the badge to be available while the group's body
    /// does not exist, so it is derived here from host state rather than from
    /// the widgets that would otherwise own the invalid value.
    fn refresh_inspector_badges(&mut self) {
        let state = phototux_engine::InspectorState {
            adjustment_kind: &self.adjustment_kind,
            adjustment_slots: &self.adjustment_slots,
            selection_active: self.selection_active,
            selection_bounds: self
                .engine
                .selection
                .bounds
                .map(|b| (b.x, b.y, b.width, b.height)),
            document_size: (self.engine.size.width, self.engine.size.height),
            text_layer_active: self.text_layer_active,
            text_font_family: &self.text_font_family,
            known_font_families: self
                .fonts_discovered
                .then_some(self.available_font_families.as_slice()),
            gpu_lost: self.gpu_lost,
        };
        let next = phototux_engine::inspector_badges_json(&state);
        if next == self.inspector_badges_json {
            return;
        }
        self.inspector_badges_json = next;
        self.inspector_badges_json_changed();
    }

    /// Set every registered inspector disclosure group to `open` and persist once.
    fn set_all_disclosure_groups(&mut self, open: bool) {
        for group in phototux_engine::default_disclosure_groups() {
            self.prefs.set_disclosure_open(&group.id, open);
        }
        self.refresh_disclosure_open_json();
        self.persist_prefs();
        self.disclosure_open_json_changed();
    }

    fn sync_pref_fields_from_store(&mut self) {
        self.refresh_disclosure_open_json();
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
        self.panel_visibility_json = self.workspace.visibility_json();
        self.dock_topology_json = self
            .workspace
            .dock
            .to_json()
            .unwrap_or_else(|_| "{}".into());
        self.dock_groups_json = self.build_dock_groups_json();
        self.workspace_focus_json =
            serde_json::to_string(&self.workspace.focus).unwrap_or_else(|_| "{}".into());
        self.active_workspace_preset_id = self.workspace.active_preset_id.clone();
    }

    fn persist_workspace_visibility(&mut self) {
        self.prefs.apply_workspace(&self.workspace);
        self.sync_panel_visibility_from_workspace();
        self.persist_prefs();
        self.panel_visibility_json_changed();
        self.dock_topology_json_changed();
        self.dock_groups_json_changed();
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
        self.guides_json_changed();
        self.grid_spacing_changed();
    }

    fn persist_prefs(&mut self) {
        if let Err(error) = self.prefs.save() {
            self.notify(
                NoticeLevel::Error,
                format!("Preferences save failed: {error}"),
            );
        }
    }

    fn invoke_command(&mut self, id: &str, args: CommandArgs) -> Result<(), CommandError> {
        let effects = self.engine.invoke(id, args)?;
        self.apply_command_effects(effects);
        Ok(())
    }

    fn active_layer_has_mask(&self) -> bool {
        self.active_mask_flag != 0
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
            "smart_object" => {
                self.has_document && self.active_layer_kind == "smart-object" && !busy
            }
            // Distributing needs something in the middle to space out.
            "has_three_layers" => self.has_document && self.layer_count > 2 && !busy,
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
            cid::STYLE_ADD => Ok(CommandArgs::LayerStyleKind {
                kind: arg.unwrap_or("drop-shadow").to_owned(),
            }),
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
                self.request_host(phototux_engine::HostRequest::NewDocument);
            }
            "document.open" => {
                self.request_host(phototux_engine::HostRequest::OpenDocument);
            }
            "document.save" => {
                if !self.document_path.is_empty() {
                    self.save_document(String::new());
                } else {
                    self.request_host(phototux_engine::HostRequest::SaveDocumentAs);
                }
            }
            "document.save_as" => {
                self.request_host(phototux_engine::HostRequest::SaveDocumentAs);
            }
            "document.export" => {
                self.request_host(phototux_engine::HostRequest::ExportDocument);
            }
            "document.close" => {
                self.request_host(phototux_engine::HostRequest::CloseDocument);
            }
            "app.quit" => {
                self.request_host(phototux_engine::HostRequest::Quit);
            }
            "help.about" => {
                self.request_host(phototux_engine::HostRequest::ShowAbout);
            }
            "prefs.open" => self.open_preferences(),
            // Shortcut and palette route through the same activation the tool
            // shelf uses, so leaving an in-progress transform or crop is
            // cancelled identically however the tool was switched.
            "tool.activate" => {
                if let Some(id) = arg {
                    self.set_active_tool(id.to_owned());
                }
            }
            "inspector.expand_all" => self.set_all_disclosure_groups(true),
            "inspector.collapse_all" => self.set_all_disclosure_groups(false),
            "document.embed_icc" => {
                self.request_host(phototux_engine::HostRequest::EmbedIccProfile);
            }
            "app.recover_gpu" => self.recover_gpu(),
            "app.simulate_device_lost" => {
                if let Err(error) = phototux_canvas::simulate_device_lost() {
                    self.report_gpu("simulate device lost", &error);
                } else {
                    self.enter_gpu_lost();
                }
            }
            "palette.open" => {
                self.request_host(phototux_engine::HostRequest::OpenCommandPalette);
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
            "selection.modify" => match arg.and_then(parse_selection_modify_arg) {
                Some((op, radius)) => self.apply_selection_modify(op, radius),
                None => {
                    // A registry entry the host cannot read is a wiring bug,
                    // not user error, so it names the argument that failed
                    // rather than doing nothing. The engine-side test
                    // `selection_modify_actions_carry_a_parsable_argument`
                    // keeps the shipped registry out of this branch.
                    self.status_text =
                        format!("Unreadable selection op: {}", arg.unwrap_or_default());
                    self.status_text_changed();
                }
            },
            "raster.flip" => self.flip_active_layer(arg != Some("v")),
            "document.rotate_90" => self.rotate_canvas_90_cw(),
            "text.bake" => self.bake_text_layer(),
            "layer.align" => self.align_layers(arg.unwrap_or_default().to_owned()),
            "shape.create" => self.add_shape_layer(arg.unwrap_or("rect").to_owned()),
            "shape.rasterize" => self.rasterize_shape_layer(),
            "smart.create" => self.convert_to_smart_object(),
            "smart.reset" => self.reset_smart_placement(),
            "smart.rasterize" => self.rasterize_smart_object(),
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
                self.toggle_panel_by_id(&format!("panel.{panel}"));
            }
            _ => {
                self.notify(NoticeLevel::Warning, format!("Unknown host op: {op}"));
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
            HostFollowUp::PlaceSmartObject { id } => {
                self.place_smart_object(id);
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
        // One projection rebuild per command, regardless of how many sync flags
        // the effect carries. Emission order below is unchanged.
        if effects.sync_layers || effects.sync_doc || effects.sync_selection {
            self.sync_from_engine();
        }
        if effects.sync_layers {
            self.emit_layer_fields();
            self.active_blend_changed();
        }
        if effects.sync_doc {
            self.emit_doc_fields();
        }
        if effects.sync_selection {
            self.emit_selection_fields();
            self.can_undo_changed();
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
                Ok(ms) => self.record_composite(ms),
                Err(error) => self.report_gpu("stroke undo", &error),
            },
            HostHistoryAction::Redo(HistoryKind::Stroke) => match phototux_canvas::redo_stroke() {
                Ok(ms) => self.record_composite(ms),
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
        // Only the live filter-gallery preview needs a patched layer list; the
        // steady-state path composites the graph's own slice without cloning
        // every layer (names, effect vectors, masks) on each edit.
        let preview = self
            .engine
            .filter_preview
            .as_ref()
            .and_then(|preview| Some((preview.layer_id, preview.to_effect()?)));
        let result = match preview {
            Some((layer_id, effect)) => {
                let mut layers = graph.layers().to_vec();
                if let Some(layer) = layers.iter_mut().find(|l| l.id == layer_id) {
                    layer.effects.push(effect);
                }
                phototux_canvas::sync_and_composite(&layers)
            }
            None => phototux_canvas::sync_and_composite(graph.layers()),
        };
        match result {
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
                self.engine.clear_dirty_rect();
                self.bump_composite_generation();
            }
            Err(e) => {
                // The composite runs from a timer as well as from edits, so it
                // can fire in the window before the GPU side of the document
                // exists. That is a transient state and the next pass will
                // succeed — reporting it would put a failure in front of the
                // user for something that is not wrong. It was always being
                // posted; the status bar simply overwrote it before anyone saw.
                if phototux_canvas::has_document() {
                    self.report_gpu("composite", &e);
                }
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
                self.record_composite(ms);
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
        // Layer ids restart at 1 in every graph, so leaving these behind would
        // hand the next document this one's sources under the same ids.
        let smart_sources: Vec<_> = std::mem::take(&mut self.smart_sources)
            .into_iter()
            .collect();
        phototux_canvas::close_document();
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        self.doc_registry
            .park_active(id, title, session, layer_pixels, smart_sources, dirty);
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
        self.smart_sources = parked.smart_sources.into_iter().collect();
        self.document_name = parked.title;
        self.dirty = parked.dirty;
        self.active_doc_id = Some(id);
        self.doc_registry.set_active_id(Some(id));
        if let Some(graph) = self.engine.graph.as_ref() {
            let size = graph.size;
            let layers = graph.layers().to_vec();
            match phototux_canvas::recover_gpu_document(size, &layers, &parked.layer_pixels) {
                Ok(ms) => {
                    self.record_composite(ms);
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

    /// A field published conditionally must not also be announced blind.
    ///
    /// `publish!` exists to keep a property quiet when its value did not move.
    /// An unconditional `x_changed()` for the same field in `emit_layer_fields`
    /// undoes that entirely — the notify fires anyway, so the guard buys
    /// nothing while looking like it works. That is the exact shape of the
    /// drift this pairing was introduced to end, and it is invisible at the
    /// call site because the two halves sit hundreds of lines apart.
    ///
    /// Reading the source is what makes them comparable: both halves are
    /// macro-expanded or generated, so there is no runtime handle to assert
    /// against.
    #[test]
    fn published_fields_are_not_also_announced_unconditionally() {
        let source = include_str!("lib.rs");

        let body = source
            .split_once("fn emit_layer_fields(&mut self) {")
            .expect("emit_layer_fields exists")
            .1;
        let body = body.split_once("\n    }").expect("function closes").0;

        let announced: Vec<&str> = body
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("self.")?
                    .strip_suffix("_changed();")
            })
            .collect();
        assert!(
            !announced.is_empty(),
            "emit_layer_fields announces nothing — the parse broke, not the invariant"
        );

        let published: Vec<&str> = source
            .split("publish!(")
            .skip(1)
            .filter_map(|rest| {
                let after_self = rest.split_once("self,")?.1;
                Some(after_self.split(',').next()?.trim())
            })
            .collect();
        assert!(
            !published.is_empty(),
            "no publish! sites found — parse broke"
        );

        let both: Vec<&str> = announced
            .iter()
            .copied()
            .filter(|field| published.contains(field))
            .collect();
        assert!(
            both.is_empty(),
            "these fields publish conditionally and then announce anyway, \
             which defeats the guard: {both:?}"
        );
    }

    /// Handbook 28: group registration order is stable and MUST NOT be
    /// reordered, so the inspector must lay groups out in registry order.
    /// Reading the QML is what makes the two orders comparable at all — the
    /// layout is declarative and has no runtime handle to assert against.
    ///
    /// Every enablement tag an action declares must be one the host answers.
    ///
    /// `action_enablement` ends in a catch-all that falls back to
    /// `has_document`, so a tag the host does not know is not an error — it is
    /// an action that quietly becomes enabled whenever a document is open.
    /// That is exactly wrong for the ones that guard a kind or a mask, and
    /// nothing said so. The arms are read out of this file as text, because
    /// the alternative is a third list of tag names for someone to forget.
    #[test]
    fn every_enablement_tag_an_action_declares_is_one_the_host_answers() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
            .expect("the ui crate can read its own source");
        let body = source
            .split("fn action_enablement(&self, tag: &str) -> bool {")
            .nth(1)
            .and_then(|rest| rest.split("\n    }").next())
            .expect("action_enablement is where this test thinks it is");
        let answered: Vec<&str> = body
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|tag| !tag.is_empty())
            .collect();
        assert!(
            answered.len() > 8,
            "found {} arms — the scan broke rather than the host",
            answered.len()
        );
        for action in phototux_engine::default_actions() {
            assert!(
                answered.contains(&action.enablement.as_str()),
                "{} declares enablement {:?}, which no arm answers — it would \
                 silently fall through to has_document",
                action.id,
                action.enablement
            );
        }
    }

    /// Which file that is, is part of what this test pins. The groups lived in
    /// `Main.qml` until the Properties body was extracted, and reading a file
    /// that no longer declares any leaves an empty list — which compares
    /// unequal and so still fails, but reports a reorder rather than a move.
    /// The emptiness check below names the real cause.
    #[test]
    fn inspector_lays_groups_out_in_registry_order() {
        let panel_qml = concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml/PropertiesPanel.qml");
        let source = std::fs::read_to_string(panel_qml).expect("read PropertiesPanel.qml");
        let laid_out: Vec<&str> = source
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("groupId: \"")?;
                rest.strip_suffix('"')
            })
            .collect();
        assert!(
            !laid_out.is_empty(),
            "PropertiesPanel.qml declares no groups — the panel body moved, \
             and this test needs to follow it to keep checking anything"
        );
        let registered: Vec<String> = phototux_engine::default_disclosure_groups()
            .into_iter()
            .map(|group| group.id)
            .collect();
        assert_eq!(
            laid_out, registered,
            "Properties group order diverged from the disclosure registry"
        );
    }

    /// QML names tools by string, and `set_active_tool` answers an unknown one
    /// by quietly activating the brush. That fallback is right for a wiring
    /// bug and terrible as a way to find one: a mistyped id in a rail button
    /// or a tool predicate looks like a tool that just does not work, with
    /// nothing logged. Reading the QML is the only way to compare the two
    /// vocabularies — one side is a declarative binding.
    #[test]
    fn every_tool_named_in_the_qml_shell_is_a_tool_the_host_knows() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml");
        let mut checked = 0_usize;
        for entry in std::fs::read_dir(dir).expect("read qml/") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("qml") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read qml file");
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            for id in source.split("\"tool.").skip(1).filter_map(|rest| {
                let id = rest.split('"').next()?;
                // `tool.activate` is a host op, not a tool; the ids this test
                // is about never contain a further quote or a space.
                (!id.contains(' ') && id != "activate").then_some(id)
            }) {
                let full = format!("tool.{id}");
                assert!(
                    phototux_engine::tool_id::is_known(&full),
                    "{name} names {full:?}, which the host does not know — \
                     selecting it silently activates the brush"
                );
                checked += 1;
            }
        }
        assert!(
            checked > 0,
            "no tool ids found in qml/ — the parse broke, not the invariant"
        );
    }

    /// The canvas creates shapes directly, not only through the Layer menu, so
    /// the registry test in the engine does not cover every caller. An unknown
    /// kind now creates nothing, which from the shell looks like a click that
    /// did not register.
    #[test]
    fn every_shape_created_from_the_qml_shell_names_a_known_preset() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml");
        let mut checked = 0_usize;
        for entry in std::fs::read_dir(dir).expect("read qml/") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("qml") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read qml file");
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            for kind in source
                .split("addShapeLayer(\"")
                .skip(1)
                .filter_map(|rest| rest.split('"').next())
            {
                assert!(
                    phototux_engine::ShapePreset::parse(kind).is_some(),
                    "{name} creates {kind:?}, which names no shape preset — \
                     the call would create nothing"
                );
                checked += 1;
            }
        }
        assert!(
            checked > 0,
            "no addShapeLayer calls found in qml/ — the parse broke, not the invariant"
        );
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
        "layerModel",
        Member = layer_model,
        Notify = layer_model_changed
    );
    qproperty!(
        "activeMaskFlag",
        Member = active_mask_flag,
        Notify = active_mask_flag_changed
    );
    qproperty!(
        "activeLayerClips",
        Member = active_layer_clips,
        Notify = active_layer_clips_changed
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
        "activeLayerName",
        Member = active_layer_name,
        Notify = active_layer_name_changed
    );
    qproperty!(
        "selectedLayerCount",
        Member = selected_layer_count,
        Notify = selected_layer_count_changed
    );
    qproperty!(
        "inspectorSubject",
        Member = inspector_subject,
        Notify = inspector_subject_changed
    );
    qproperty!(
        "inspectorSubjectsJson",
        Member = inspector_subjects_json,
        Notify = inspector_subjects_json_changed
    );
    qproperty!(
        "historyModel",
        Member = history_model,
        Notify = history_model_changed
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
        "selectionTolerance",
        Member = selection_tolerance,
        Notify = selection_tolerance_changed
    );
    qproperty!(
        "gradientKind",
        Member = gradient_kind,
        Notify = gradient_kind_changed
    );
    qproperty!(
        "gradientKindsJson",
        Member = gradient_kinds_json,
        Notify = gradient_kinds_json_changed
    );
    qproperty!(
        "blendIfJson",
        Member = blend_if_json,
        Notify = blend_if_json_changed
    );
    qproperty!(
        "shapeJson",
        Member = shape_json,
        Notify = shape_json_changed
    );
    qproperty!(
        "smartJson",
        Member = smart_json,
        Notify = smart_json_changed
    );
    qproperty!(
        "blendIfChannelsJson",
        Member = blend_if_channels_json,
        Notify = blend_if_channels_json_changed
    );
    qproperty!(
        "navigatorThumbnail",
        Member = navigator_thumbnail,
        Notify = navigator_thumbnail_changed
    );
    qproperty!(
        "noticesJson",
        Member = notices_json,
        Notify = notices_json_changed
    );
    qproperty!(
        "toolSlotsJson",
        Member = tool_slots_json,
        Notify = tool_slots_json_changed
    );
    qproperty!(
        "alignOpsJson",
        Member = align_ops_json,
        Notify = align_ops_json_changed
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
        "disclosureOpenJson",
        Member = disclosure_open_json,
        Notify = disclosure_open_json_changed
    );
    qproperty!(
        "disclosureGroupsJson",
        Member = disclosure_groups_json,
        Notify = disclosure_groups_json_changed
    );
    qproperty!(
        "compositeGeneration",
        Member = composite_generation,
        Notify = composite_generation_changed
    );
    qproperty!(
        "inspectorBadgesJson",
        Member = inspector_badges_json,
        Notify = inspector_badges_json_changed
    );
    qproperty!(
        "adjustmentRangesJson",
        Member = adjustment_ranges_json,
        Notify = adjustment_ranges_json_changed
    );
    qproperty!(
        "adjustmentLabelsJson",
        Member = adjustment_labels_json,
        Notify = adjustment_labels_json_changed
    );
    qproperty!(
        "filterCatalogJson",
        Member = filter_catalog_json,
        Notify = filter_catalog_json_changed
    );
    qproperty!(
        "layerStylesJson",
        Member = layer_styles_json,
        Notify = layer_styles_json_changed
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
        "adjustmentSlotsJson",
        Member = adjustment_slots_json,
        Notify = adjustment_slots_json_changed
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
        "dockGroupsJson",
        Member = dock_groups_json,
        Notify = dock_groups_json_changed
    );
    qproperty!(
        "pendingHostRequest",
        Member = pending_host_request,
        Notify = pending_host_request_changed
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
        "blendModesJson",
        Member = blend_modes_json,
        Notify = blend_modes_json_changed
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
    /// The model object itself never changes identity — it is created once and
    /// lives as long as the session — so this fires only to satisfy the
    /// property declaration. Row changes reach QML through the model's own
    /// signals, which is the point of having one.
    #[qsignal]
    fn layer_model_changed(&mut self);
    #[qsignal]
    fn active_mask_flag_changed(&mut self);
    #[qsignal]
    fn active_layer_clips_changed(&mut self);
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
    fn active_layer_name_changed(&mut self);
    #[qsignal]
    fn selected_layer_count_changed(&mut self);
    #[qsignal]
    fn inspector_subject_changed(&mut self);
    #[qsignal]
    fn inspector_subjects_json_changed(&mut self);
    /// Fires only to satisfy the property declaration; the model's identity
    /// never changes. Row changes reach QML through the model's own signals.
    #[qsignal]
    fn history_model_changed(&mut self);
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
    fn selection_tolerance_changed(&mut self);
    #[qsignal]
    fn gradient_kind_changed(&mut self);
    #[qsignal]
    fn gradient_kinds_json_changed(&mut self);
    #[qsignal]
    fn align_ops_json_changed(&mut self);
    #[qsignal]
    fn tool_slots_json_changed(&mut self);
    #[qsignal]
    fn notices_json_changed(&mut self);
    #[qsignal]
    fn navigator_thumbnail_changed(&mut self);
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
    fn disclosure_open_json_changed(&mut self);
    #[qsignal]
    fn disclosure_groups_json_changed(&mut self);
    #[qsignal]
    fn composite_generation_changed(&mut self);
    #[qsignal]
    fn inspector_badges_json_changed(&mut self);
    #[qsignal]
    fn adjustment_ranges_json_changed(&mut self);
    #[qsignal]
    fn adjustment_labels_json_changed(&mut self);
    #[qsignal]
    fn filter_catalog_json_changed(&mut self);
    #[qsignal]
    fn layer_styles_json_changed(&mut self);
    #[qsignal]
    fn blend_if_json_changed(&mut self);
    #[qsignal]
    fn shape_json_changed(&mut self);
    #[qsignal]
    fn smart_json_changed(&mut self);
    #[qsignal]
    fn blend_if_channels_json_changed(&mut self);
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
    fn adjustment_slots_json_changed(&mut self);
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
    fn dock_groups_json_changed(&mut self);
    #[qsignal]
    fn pending_host_request_changed(&mut self);
    #[qsignal]
    fn panel_visibility_json_changed(&mut self);
    #[qsignal]
    fn tool_descriptors_json_changed(&mut self);
    #[qsignal]
    fn blend_modes_json_changed(&mut self);
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

    #[qslot]
    fn assign_document_profile(&mut self, profile: String) {
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_ASSIGN_PROFILE,
            CommandArgs::AssignProfile { profile },
        ) {
            self.report_action_error(&error);
        } else {
            self.notify(
                NoticeLevel::Info,
                format!(
                    "Assigned profile (pixels not converted): {}",
                    self.engine
                        .graph
                        .as_ref()
                        .map(|g| g.color.assigned_profile.as_str())
                        .unwrap_or("?")
                ),
            );
        }
    }

    /// Convert document pixels into `profile` (destructive; DR-012).
    #[qslot]
    fn convert_document_profile(&mut self, profile: String) {
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_CONVERT_PROFILE,
            CommandArgs::ConvertProfile { profile },
        ) {
            self.report_action_error(&error);
        }
    }

    /// Rebuild GPU document resources after device/surface loss (engine graph unchanged).
    #[qslot]
    fn recover_gpu(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            self.notify(
                NoticeLevel::Warning,
                "Open a document before recovering the canvas.",
            );
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
                self.refresh_inspector_badges();
                self.record_composite(ms);
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
                self.notify(NoticeLevel::Info, "Graphics recovered — canvas restored");
                self.publish_announcement();
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
                self.notify(NoticeLevel::Error, format!("Embed ICC failed: {error}"));
                return;
            }
        };
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(error) => {
                self.notify(NoticeLevel::Error, format!("Embed ICC failed: {error}"));
                return;
            }
        };
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_SET_ICC,
            CommandArgs::SetIcc { bytes: Some(bytes) },
        ) {
            self.notify(NoticeLevel::Error, format!("Embed ICC failed: {error}"));
        }
    }

    #[qslot]
    fn clear_embedded_icc(&mut self) {
        if let Err(error) = self.invoke_command(
            command_id::DOCUMENT_SET_ICC,
            CommandArgs::SetIcc { bytes: None },
        ) {
            self.notify(NoticeLevel::Error, format!("Clear ICC failed: {error}"));
        }
    }

    #[qslot]
    fn invoke_action(&mut self, id: String) {
        let Some(action) = phototux_engine::action_by_id(&id) else {
            self.notify(NoticeLevel::Warning, format!("Unknown action: {id}"));
            return;
        };
        if !self.action_enablement(&action.enablement) {
            let label = action.label.replace('&', "");
            let reason = if self.io_busy
                && matches!(
                    action.enablement.as_str(),
                    "io_idle"
                        | "has_document"
                        | "has_document_io_idle"
                        | "can_undo"
                        | "can_redo"
                        | "selection_active"
                        | "has_mask"
                        | "no_mask"
                        | "has_multiple_layers"
                ) {
                "busy"
            } else {
                match action.enablement.as_str() {
                    "has_document" | "has_document_io_idle" => "no document open",
                    "can_undo" => "nothing to undo",
                    "can_redo" => "nothing to redo",
                    "io_idle" => "busy",
                    other => other,
                }
            };
            self.notify(
                NoticeLevel::Warning,
                format!("Action unavailable: {label} ({reason})"),
            );
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
                        self.report_action_error(&error);
                    }
                }
                Err(error) => self.report_action_error(&error),
            }
        }
    }

    /// Surface action/command failures in the status footer and Properties announce.
    ///
    /// Takes the typed error rather than its rendered text. The classification
    /// used to be recovered by searching that text for the word "rejected",
    /// having just thrown away the value that knew the answer — which also
    /// mis-routed any other failure whose message happened to contain the word.
    fn report_action_error(&mut self, error: &CommandError) {
        if error.is_user_correctable() {
            let message = error.user_message();
            // Warning, not info: the command did not happen. The colour and the
            // spoken prefix are the only thing distinguishing "I did that" from
            // "I could not do that".
            self.notify(NoticeLevel::Warning, message.clone());
            self.engine.announce(message);
            self.publish_announcement();
            return;
        }
        self.report_gpu("action", &error.to_string());
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
            self.notify(NoticeLevel::Info, error.to_string());
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
            self.notify(NoticeLevel::Info, error.to_string());
        }
        self.sync_filter_preview_fields();
        self.emit_filter_preview_fields();
    }

    #[qslot]
    fn filter_gallery_apply(&mut self) {
        if let Err(error) = self.invoke_command(command_id::FILTER_COMMIT, CommandArgs::None) {
            self.notify(NoticeLevel::Info, error.to_string());
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
            self.notify(NoticeLevel::Info, self.engine.last_announce.clone());
            self.publish_announcement();
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
        self.publish_history_projection();
        self.can_undo_changed();
        self.can_redo_changed();
    }

    /// Replace the fallback font list with fontconfig's, once.
    ///
    /// Called by the Character chrome when it first becomes reachable, keeping
    /// the ~80 ms `fc-list` subprocess off the cold-boot path.
    #[qslot]
    fn ensure_fonts_discovered(&mut self) {
        if self.fonts_discovered {
            return;
        }
        self.fonts_discovered = true;
        self.available_font_families = fonts::discover_font_families();
        self.available_fonts_json = fonts::font_families_json(&self.available_font_families);
        self.available_fonts_json_changed();
        // The Character group can now say whether its family is installed.
        self.refresh_inspector_badges();
    }

    /// Persist an inspector disclosure group's expanded state (handbook 28:
    /// presentation state, never document state).
    #[qslot]
    fn set_disclosure_open(&mut self, group_id: String, open: bool) {
        if group_id.is_empty() {
            return;
        }
        self.prefs.set_disclosure_open(&group_id, open);
        self.refresh_disclosure_open_json();
        self.persist_prefs();
        self.disclosure_open_json_changed();
    }

    /// Expand every inspector disclosure group in one step.
    #[qslot]
    fn expand_all_disclosure_groups(&mut self) {
        self.set_all_disclosure_groups(true);
    }

    /// Collapse every inspector disclosure group in one step.
    #[qslot]
    fn collapse_all_disclosure_groups(&mut self) {
        self.set_all_disclosure_groups(false);
    }

    /// Raise a docked panel to be the visible tab of its group.
    #[qslot]
    fn raise_panel_tab(&mut self, panel_id: String) {
        if self.workspace.dock.set_active_tab(&panel_id).is_err() {
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Set visibility by panel descriptor id (prefs / descriptor-driven chrome).
    #[qslot]
    fn set_panel_visible(&mut self, panel_id: String, value: bool) {
        if !self.workspace.set_visible(&panel_id, value) {
            self.notify(NoticeLevel::Warning, format!("Unknown panel: {panel_id}"));
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
                    self.notify(
                        NoticeLevel::Warning,
                        format!("Dock topology rejected: {reason}"),
                    );
                    return;
                }
                self.persist_workspace_visibility();
            }
            Err(reason) => {
                self.notify(
                    NoticeLevel::Warning,
                    format!("Dock topology invalid: {reason}"),
                );
            }
        }
    }

    /// Apply a workspace mutation, reporting failure and persisting on success.
    ///
    /// Seven slots previously repeated this epilogue verbatim. Keeping it in one
    /// place is what makes "a rejected layout change must not be persisted" a
    /// property of the operation rather than a convention each slot restates.
    fn commit_workspace_op(&mut self, result: Result<(), String>, failure_label: &str) {
        if let Err(reason) = result {
            self.notify(NoticeLevel::Info, format!("{failure_label}: {reason}"));
            return;
        }
        self.persist_workspace_visibility();
    }

    /// Move a panel within the right stack (`delta` −1 up / +1 down).
    #[qslot]
    fn move_panel_in_stack(&mut self, panel_id: String, delta: i32) {
        let outcome = self
            .workspace
            .move_panel_in_stack(&panel_id, delta)
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Panel move failed");
    }

    /// Commit a dragged panel height.
    ///
    /// The shell drags in pixels and commits on release rather than on every
    /// motion event: each commit bumps the workspace revision and writes prefs,
    /// which is not something a drag should do sixty times a second.
    #[qslot]
    fn set_panel_height(&mut self, panel_id: String, height: i32) {
        let Ok(height) = u32::try_from(height) else {
            return;
        };
        let outcome = self
            .workspace
            .set_panel_height(&panel_id, height)
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Panel resize failed");
    }

    /// Reorder right stack by indices (DnD commit).
    #[qslot]
    fn reorder_panel_in_stack(&mut self, from: i32, to: i32) {
        if from < 0 || to < 0 {
            return;
        }
        let outcome = self
            .workspace
            .reorder_panel_in_stack(from as usize, to as usize)
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Panel reorder failed");
    }

    /// Tear a docked panel into a floating window.
    #[qslot]
    fn tear_off_panel(&mut self, panel_id: String, x: i32, y: i32, width: i32, height: i32) {
        let w = width.max(200) as u32;
        let h = height.max(120) as u32;
        let outcome = self
            .workspace
            .tear_off_panel(&panel_id, x, y, w, h, "")
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Tear-off failed");
    }

    /// Return a floating panel to the right dock stack.
    #[qslot]
    fn redock_panel(&mut self, panel_id: String) {
        let outcome = self
            .workspace
            .redock_panel(&panel_id)
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Redock failed");
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
        let outcome = self
            .workspace
            .set_floating_geometry(&panel_id, x, y, w, h)
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Float geometry failed");
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
        let outcome = self
            .workspace
            .toggle_auto_hide(&panel_id)
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Auto-hide failed");
    }

    /// Pin (reveal) an auto-hidden panel.
    #[qslot]
    fn pin_panel(&mut self, panel_id: String) {
        let outcome = self
            .workspace
            .pin_panel(&panel_id)
            .map_err(|reason| reason.to_string());
        self.commit_workspace_op(outcome, "Pin failed");
    }

    fn toggle_panel_by_id(&mut self, panel_id: &str) {
        if !self.workspace.toggle(panel_id) {
            self.notify(NoticeLevel::Warning, format!("Unknown panel: {panel_id}"));
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
        self.notify(NoticeLevel::Info, "Workspace reset to Essentials");
    }

    #[qslot]
    fn apply_workspace_preset(&mut self, preset_id: String) {
        let Some(preset) = phototux_engine::resolve_workspace_preset(
            &preset_id,
            &self.prefs.user_workspace_presets_json,
        ) else {
            self.notify(
                NoticeLevel::Warning,
                format!("Unknown workspace preset: {preset_id}"),
            );
            return;
        };
        self.workspace.apply_preset(&preset);
        self.persist_workspace_visibility();
        self.notify(NoticeLevel::Info, format!("Workspace: {}", preset.title));
    }

    #[qslot]
    fn save_user_workspace_preset(&mut self, title: String) {
        let title = title.trim().to_owned();
        if title.is_empty() {
            self.notify(NoticeLevel::Warning, "Give the workspace preset a name.");
            return;
        }
        let slug = phototux_engine::slugify_workspace_preset_title(&title);
        let id = format!("{}{slug}", phototux_engine::USER_WORKSPACE_PRESET_PREFIX);
        let preset = phototux_engine::WorkspacePreset::from_workspace(id, title, &self.workspace);
        if let Err(err) = self.prefs.upsert_user_workspace_preset(preset.clone()) {
            self.notify(NoticeLevel::Info, err);
            return;
        }
        self.workspace.active_preset_id = preset.id.clone();
        self.refresh_workspace_presets_json();
        self.persist_workspace_visibility();
        self.emit_workspace_presets_json();
        self.notify(
            NoticeLevel::Info,
            format!("Saved workspace preset “{}”", preset.title),
        );
    }

    #[qslot]
    fn delete_user_workspace_preset(&mut self, preset_id: String) {
        if !self.prefs.delete_user_workspace_preset(&preset_id) {
            self.notify(
                NoticeLevel::Warning,
                format!("Not a user workspace preset: {preset_id}"),
            );
            return;
        }
        if self.workspace.active_preset_id == preset_id {
            self.workspace.active_preset_id.clear();
        }
        self.refresh_workspace_presets_json();
        self.persist_prefs();
        self.sync_panel_visibility_from_workspace();
        self.active_workspace_preset_id_changed();
        self.emit_workspace_presets_json();
        self.notify(NoticeLevel::Info, "Deleted user workspace preset");
    }

    #[qslot]
    fn restore_last_saved_workspace(&mut self) {
        let Some(ws) = self.prefs.load_last_saved_workspace() else {
            self.notify(
                NoticeLevel::Warning,
                "There is no saved workspace layout to restore.",
            );
            return;
        };
        self.workspace = ws;
        self.persist_workspace_visibility();
        self.notify(NoticeLevel::Info, "Workspace restored from last saved");
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
        // An unrecognised id falls back rather than being refused: this is the
        // shell's own tool rail talking, so a miss is a wiring bug, and leaving
        // the user with no active tool would be worse than the brush. The
        // engine-side tests keep a shipped id from ever reaching the fallback.
        let id = if tool_id::is_known(&tool) {
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
        // Always remember the last tool; restore_last_tool only gates apply-on-launch.
        self.persist_prefs();
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
            dirty |= self.handle_engine_event(ev);
        }
        for event in self.file_worker.poll_events() {
            self.handle_file_event(event);
        }
        if dirty {
            self.emit_poll_dirty_changes();
        }
        // Here rather than in `emit_poll_dirty_changes`, which only runs when a
        // worker event marked something dirty, and rather than in the composite
        // itself, where a readback would sit on the path DR-017 budgets. This
        // runs every tick; the interval and generation guards inside do the
        // throttling.
        self.refresh_navigator_thumbnail();
    }

    fn handle_engine_event(&mut self, ev: EngineEvent) -> bool {
        match ev {
            EngineEvent::CompositeDone { ms } => {
                self.engine.set_composite_ms(ms);
                // Both readouts show two decimals, and this fires on every
                // paced composite throughout a stroke. Signalling a difference
                // finer than the display shows relaid out the status bar and
                // the collapsed Diagnostics summary for nothing.
                let visibly_changed = (ms * 100.0).round() != (self.composite_ms * 100.0).round();
                self.composite_ms = ms;
                if visibly_changed {
                    self.composite_ms_changed();
                }
                self.bump_composite_generation();
                false
            }
            EngineEvent::StrokeLatency { ms } => {
                self.engine.set_stroke_latency_ms(ms);
                self.stroke_latency_ms = ms;
                self.stroke_latency_ms_changed();
                false
            }
            EngineEvent::StrokeJournaled(entry) => {
                // Off the UI thread: this arrives in the frame tick at pen-up,
                // and serializing a few thousand dabs and writing them to disk
                // is not work to do in the frame the user is watching.
                let _ = self
                    .file_worker
                    .send(crate::file_worker::FileCommand::JournalStroke(Box::new(
                        entry,
                    )));
                false
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
                true
            }
            EngineEvent::Error(e) => {
                self.report_gpu("paint worker", &e);
                false
            }
        }
    }

    fn handle_file_event(&mut self, event: FileEvent) {
        match event {
            FileEvent::Opened { path, raster } => self.handle_raster_opened(path, raster),
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
            } => self.handle_psd_opened(path, graph, layer_rasters, flattened, report),
            FileEvent::Saved { path } => self.handle_file_saved(path),
            FileEvent::Autosaved => {
                self.notify(NoticeLevel::Info, "Autosave written");
            }
            FileEvent::Exported { path } => {
                self.io_busy = false;
                self.notify(NoticeLevel::Info, format!("Exported {}", path.display()));
                self.io_busy_changed();
                self.status_text_changed();
            }
            FileEvent::Cancelled { operation } => {
                self.io_busy = false;
                self.notify(NoticeLevel::Info, format!("{operation} cancelled"));
                self.io_busy_changed();
                self.status_text_changed();
            }
            FileEvent::Failed { operation, message } => {
                self.fail_io(operation, &message);
            }
        }
    }

    fn emit_poll_dirty_changes(&mut self) {
        let prev_status = self.status_text.clone();
        let prev_a11y = self.accessibility_tree_json.clone();
        let prev_can_undo = self.can_undo;
        let prev_can_redo = self.can_redo;
        let prev_dirty = self.dirty;
        self.sync_from_engine();
        // Guarded: ~100 menu items bind to enablement and re-evaluate together
        // on can_undo/can_redo, so emitting unconditionally rebuilt the whole
        // menu state at every pen-up even when nothing had changed.
        if self.can_undo != prev_can_undo {
            self.can_undo_changed();
        }
        if self.can_redo != prev_can_redo {
            self.can_redo_changed();
        }
        if self.dirty != prev_dirty {
            self.dirty_changed();
        }
        if self.status_text != prev_status {
            self.status_text_changed();
        }
        if self.accessibility_tree_json != prev_a11y {
            self.accessibility_tree_json_changed();
            self.atspi_projection_json_changed();
        }
    }

    fn handle_raster_opened(&mut self, path: PathBuf, raster: Raster) {
        let size = DocumentSize::new(raster.width(), raster.height());
        let layer_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Image".to_owned());
        if let Err(error) = self.prepare_new_document_tab(&layer_name) {
            self.fail_io("Open", &error);
            return;
        }
        let graph = DocumentGraph::new_flattened(size, layer_name.clone());
        let Some(target_layer) = graph.active_id() else {
            self.fail_io("Open", "decoded document has no layer");
            return;
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
                self.record_composite(ms);
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

    fn handle_psd_opened(
        &mut self,
        path: PathBuf,
        graph: DocumentGraph,
        layer_rasters: Vec<(LayerId, Raster)>,
        flattened: Option<Raster>,
        report: Vec<CompatibilityIssue>,
    ) {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported.psd".into());
        if let Err(error) = self.prepare_new_document_tab(&title) {
            self.fail_io("Open", &error);
            return;
        }
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        let Some(ms) = self.open_psd_pixels(&graph, &layer_rasters, flattened.as_ref()) else {
            return;
        };
        self.engine.replace_graph(graph);
        self.record_composite(ms);
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

    fn open_psd_pixels(
        &mut self,
        graph: &DocumentGraph,
        layer_rasters: &[(LayerId, Raster)],
        flattened: Option<&Raster>,
    ) -> Option<f32> {
        let result = if !layer_rasters.is_empty() {
            phototux_canvas::open_document(graph.size, graph.layers()).and_then(|ms| {
                for (id, raster) in layer_rasters {
                    phototux_canvas::write_layer_rgba(*id, raster.pixels())?;
                }
                Ok(ms)
            })
        } else if let Some(raster) = flattened {
            let Some(target_layer) = graph.active_id() else {
                self.fail_io("Open", "PSD import has no layer");
                return None;
            };
            phototux_canvas::open_raster_document(
                graph.size,
                graph.layers(),
                target_layer,
                raster.pixels(),
            )
        } else {
            self.fail_io("Open", "PSD import produced no pixel data");
            return None;
        };
        match result {
            Ok(ms) => Some(ms),
            Err(error) => {
                self.fail_io("Open", &error);
                None
            }
        }
    }

    fn handle_file_saved(&mut self, path: PathBuf) {
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
        self.notify(NoticeLevel::Info, format!("Saved {}", path.display()));
        self.sync_from_engine();
        self.emit_doc_fields();
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
        // Every dab tool, not two named ids: the retouch tools are the same
        // brush with a different dab mode, and naming brush and eraser here
        // silently refused all seven of them.
        if !DabMode::is_dab_tool(&self.engine.active_tool) {
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
            self.notify(NoticeLevel::Info, error);
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
            self.notify(NoticeLevel::Info, error);
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
            self.notify(NoticeLevel::Info, error);
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
        self.notify(NoticeLevel::Info, format!("Opening {}…", path.display()));
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
        self.notify(NoticeLevel::Info, format!("Saving {}…", path.display()));
        self.io_busy_changed();
        self.status_text_changed();
        let sources = self.smart_source_rasters();
        if let Err(error) = self.file_worker.send(FileCommand::SavePtx {
            path,
            graph,
            sources,
        }) {
            self.pending_save_generation = None;
            self.fail_io("Save", &error);
        }
    }

    /// Smart-object sources as `.ptx` assets.
    ///
    /// Sources for layers that are no longer smart objects are skipped rather
    /// than written: undoing a rasterize puts the payload back and the host
    /// still holds the pixels, but a source with no layer to belong to would
    /// be dead weight in every later save.
    fn smart_source_rasters(&self) -> HashMap<u64, Raster> {
        let Some(graph) = self.engine.graph.as_ref() else {
            return HashMap::new();
        };
        self.smart_sources
            .iter()
            .filter(|(id, _)| graph.get(**id).is_some_and(|l| l.smart.is_some()))
            .filter_map(|(id, source)| {
                Raster::new(
                    source.width,
                    source.height,
                    source.pixels.clone().into_boxed_slice(),
                )
                .ok()
                .map(|raster| (id.0, raster))
            })
            .collect()
    }

    #[qslot]
    fn cancel_io(&mut self) {
        self.file_worker.cancel_token().cancel();
        self.notify(NoticeLevel::Info, "Cancelling…");
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
        let sources = self.smart_source_rasters();
        let _ = self.file_worker.send(FileCommand::Autosave {
            graph,
            original,
            sources,
        });
    }

    #[qslot]
    fn sync_recovery_list_fields(&mut self) {
        let entries = list_recoverable().unwrap_or_default();
        self.recovery_entries_json = recovery_entries_view_json(&entries);
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
                self.notify(NoticeLevel::Info, "Recovered unsaved document");
            }
            Err(error) => self.fail_io("Recover", &error.to_string()),
        }
    }

    /// Discard every recoverable snapshot.
    ///
    /// Reachable in one step because the list is unbounded: a session that
    /// crashed repeatedly leaves dozens of entries, and clearing them one at a
    /// time is the kind of chore people abandon halfway. The chooser makes the
    /// caller confirm before this runs — it permanently deletes unsaved work.
    #[qslot]
    fn discard_all_recovery(&mut self) {
        let Ok(entries) = list_recoverable() else {
            return;
        };
        let count = entries.len();
        for entry in &entries {
            let _ = discard_recovery(entry);
        }
        self.sync_recovery_list_fields();
        self.emit_recovery_list();
        self.set_status(format!("Discarded {count} recovery snapshot(s)"));
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
        self.notify(NoticeLevel::Info, format!("Exporting {}…", path.display()));
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
        // Nothing parks these on the way out, and each one is a whole document
        // of pixels: closing tabs without this leaks one per smart object.
        self.smart_sources.clear();
        self.active_doc_id = None;
        self.doc_registry.set_active_id(None);
        self.selection_preview_active = false;
        self.crop_preview_active = false;
        let next = self.doc_registry.parked_ids().next();
        if let Some(next_id) = next {
            if let Err(error) = self.activate_document_id(next_id) {
                self.notify(NoticeLevel::Info, error);
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

    /// Publish a shell capability request for QML to act on.
    ///
    /// This used to be written into `status_text` behind a `"host:"` prefix,
    /// which made the status bar an RPC channel and let any QML caller forge a
    /// request through `setStatus`. A dedicated property keeps the two apart.
    fn request_host(&mut self, request: phototux_engine::HostRequest) {
        self.pending_host_request = request.as_str().to_owned();
        self.pending_host_request_changed();
    }

    /// Acknowledge the pending request once the host has acted on it.
    #[qslot]
    fn clear_host_request(&mut self) {
        if self.pending_host_request.is_empty() {
            return;
        }
        self.pending_host_request.clear();
        self.pending_host_request_changed();
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
        let next = self.engine.fps;
        // Emitted once per frame from the frame clock. The readout is rounded to
        // whole frames per second, so signalling on every change of a smoothed
        // float relaid out the status bar for a value that did not visibly move.
        if next.round() == self.fps.round() {
            self.fps = next;
            return;
        }
        self.fps = next;
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

    /// Mirror the engine's latest announcement to QML.
    ///
    /// Copying the string and emitting its notify were written out as a pair at
    /// every announcing slot. Pairing them here means an announcement cannot be
    /// stored without being published, which is the failure that leaves a screen
    /// reader describing the previous action.
    fn publish_announcement(&mut self) {
        self.last_announce = self.engine.last_announce.clone();
        self.last_announce_changed();
    }

    #[qslot]
    /// Post an informational message. Kept as a slot because QML calls it.
    fn set_status(&mut self, text: String) {
        self.notify(NoticeLevel::Info, text);
    }

    // —— Layers / undo (Phase 3) ——

    /// Dismiss one toast, by the id the projection gave it.
    #[qslot]
    fn dismiss_notice(&mut self, id: i64) {
        if u64::try_from(id).is_ok_and(|id| self.notices.dismiss(id)) {
            self.publish_notices();
        }
    }

    /// Dismiss every toast at once.
    #[qslot]
    fn dismiss_all_notices(&mut self) {
        if !self.notices.is_empty() {
            self.notices.clear();
            self.publish_notices();
        }
    }

    #[qslot]
    fn add_layer(&mut self) {
        if let Err(error) = self.invoke_command(command_id::LAYER_CREATE, CommandArgs::None) {
            self.report_action_error(&error);
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
        self.commit_selection_edit("selection apply", || match shape {
            SelectionShape::Rect => phototux_canvas::selection_apply_rect(rect, mode),
            SelectionShape::Ellipse => phototux_canvas::selection_apply_ellipse(rect, mode),
            SelectionShape::Mask => Ok(()),
        });
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
        let PathVerdict::Polygon(parsed) = selection_path::classify(&points) else {
            // Every way a path can be unusable ends here: too few points, or
            // points that enclose nothing. There is nothing to commit, so the
            // in-progress path is dropped without a report.
            self.clear_selection_path();
            return;
        };
        let mode = SelectionCombine::parse(&combine);
        self.commit_selection_edit("polygon selection", || {
            phototux_canvas::selection_apply_polygon(&parsed, mode)
        });
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
        self.commit_selection_edit("deselect", phototux_canvas::selection_clear);
        self.selection_preview_active = false;
        self.clear_selection_path();
        let _ = self.invoke_command(command_id::SELECTION_DESELECT, CommandArgs::None);
    }

    #[qslot]
    fn select_all(&mut self) {
        if !self.engine.has_document {
            return;
        }
        self.commit_selection_edit("select all", phototux_canvas::selection_select_all);
        self.selection_preview_active = false;
        let _ = self.invoke_command(command_id::SELECTION_SELECT_ALL, CommandArgs::None);
    }

    #[qslot]
    fn invert_selection(&mut self) {
        if !self.engine.has_document {
            return;
        }
        self.commit_selection_edit("invert selection", phototux_canvas::selection_invert);
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
        let payload = match crate::clipboard::ImagePayload::new(width, height, pixels) {
            Ok(payload) => payload,
            Err(refusal) => {
                self.notify(NoticeLevel::Info, refusal.message("Copy"));
                return;
            }
        };
        let os_ok = Self::push_os_clipboard_rgba(&payload).is_ok();
        self.clipboard_rgba = Some(payload);
        self.notify(
            NoticeLevel::Info,
            if os_ok {
                "Copied (app + system clipboard)".to_owned()
            } else {
                "Copied (app clipboard only)".to_owned()
            },
        );
    }

    /// Copy selection coverage plus active-layer (or composite) pixels masked by it.
    fn copy_active_selection_payload(&mut self) {
        let Ok(r8) = phototux_canvas::selection_snapshot() else {
            self.notify(NoticeLevel::Warning, "Make a selection first.");
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let width = graph.size.width;
        let height = graph.size.height;

        // Validate both buffers against the document before either is used.
        // The masking loop below indexes `pixels` by `r8`'s length, so their
        // agreement is a precondition of that loop rather than a policy check
        // that could be deferred to the payload constructors.
        let Ok(coverage) = crate::clipboard::CoveragePayload::new(width, height, r8) else {
            self.notify(NoticeLevel::Error, "Copy selection failed: size mismatch");
            return;
        };
        let rgba_result = self
            .active_id()
            .and_then(|id| phototux_canvas::read_layer_rgba(id).ok())
            .or_else(|| phototux_canvas::read_composite_rgba().ok());
        let Some((rw, rh, pixels)) = rgba_result else {
            // Coverage-only fallback so Paste as Selection still works.
            self.copy_selection_mask();
            return;
        };
        let (Ok(image), true) = (
            crate::clipboard::ImagePayload::new(rw, rh, pixels),
            coverage.fits(rw, rh),
        ) else {
            self.copy_selection_mask();
            return;
        };

        // Both now describe the same document, so one coverage byte lines up
        // with one pixel's alpha.
        let mut pixels = image.rgba().to_vec();
        for (i, &cov) in coverage.coverage().iter().enumerate() {
            let a = u16::from(pixels[i * 4 + 3]);
            pixels[i * 4 + 3] = ((a * u16::from(cov)) / 255) as u8;
        }
        let Ok(image) = crate::clipboard::ImagePayload::new(width, height, pixels) else {
            self.copy_selection_mask();
            return;
        };
        let os_ok = Self::push_os_clipboard_rgba(&image).is_ok();
        self.clipboard_selection_r8 = Some(coverage);
        self.clipboard_rgba = Some(image);
        self.engine.announce("Selection copied");
        self.notify(
            NoticeLevel::Info,
            if os_ok {
                "Copied selection pixels (app + system clipboard)".to_owned()
            } else {
                "Copied selection pixels (app clipboard)".to_owned()
            },
        );
        self.publish_announcement();
    }

    #[qslot]
    fn copy_selection_mask(&mut self) {
        let Ok(r8) = phototux_canvas::selection_snapshot() else {
            self.notify(NoticeLevel::Error, "Copy selection failed: no mask");
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let width = graph.size.width;
        let height = graph.size.height;
        // Size ceiling and coverage shape are one question: does this buffer
        // describe this document, and may we hold it.
        let coverage = match crate::clipboard::CoveragePayload::new(width, height, r8) {
            Ok(coverage) => coverage,
            Err(refusal) => {
                self.notify(NoticeLevel::Info, refusal.message("Copy"));
                return;
            }
        };
        let os_ok = Self::push_os_clipboard_gray(&coverage).is_ok();
        self.clipboard_selection_r8 = Some(coverage);
        self.engine.announce("Selection copied");
        self.notify(
            NoticeLevel::Info,
            if os_ok {
                "Copied selection (app + system grayscale)".to_owned()
            } else {
                "Copied selection (app clipboard)".to_owned()
            },
        );
        self.publish_announcement();
    }

    #[qslot]
    fn copy_layer_mask(&mut self) {
        let Some(id) = self.active_id() else {
            self.notify(NoticeLevel::Warning, "Select a layer first.");
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
            self.notify(NoticeLevel::Warning, "This layer has no mask to copy.");
            return;
        }
        let width = graph.size.width;
        let height = graph.size.height;
        let Ok(r8) = phototux_canvas::read_mask_r8(id) else {
            self.notify(NoticeLevel::Error, "Copy mask failed: layer has no mask");
            return;
        };
        // Coverage, not an image: this validates the buffer describes *this*
        // document, which the ceiling check that used to stand here did not.
        let coverage = match crate::clipboard::CoveragePayload::new(width, height, r8) {
            Ok(coverage) => coverage,
            Err(refusal) => {
                self.notify(NoticeLevel::Info, refusal.message("Copy"));
                return;
            }
        };
        let os_ok = Self::push_os_clipboard_gray(&coverage).is_ok();
        self.clipboard_mask_r8 = Some(coverage);
        self.engine.announce("Layer mask copied");
        self.notify(
            NoticeLevel::Info,
            if os_ok {
                "Copied layer mask (app + system grayscale)".to_owned()
            } else {
                "Copied layer mask (app clipboard)".to_owned()
            },
        );
        self.publish_announcement();
    }

    #[qslot]
    fn paste_selection_mask(&mut self) {
        let Some(coverage) = self
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
        // One question, not two: the payload already guarantees its buffer
        // matches its own dimensions, so this only asks whether those are the
        // open document's.
        if !coverage.fits(graph.size.width, graph.size.height) {
            self.fail_io(
                "Paste selection",
                "clipboard mask size does not match document",
            );
            return;
        }
        if let Err(error) = phototux_canvas::selection_restore(coverage.coverage()) {
            self.report_gpu("paste selection", &error);
            return;
        }
        self.engine.selection.active = true;
        self.sync_from_engine();
        self.emit_doc_fields();
        self.engine.announce("Selection pasted");
        self.notify(NoticeLevel::Info, "Pasted selection from clipboard");
        self.publish_announcement();
    }

    #[qslot]
    fn paste_layer_mask(&mut self) {
        let Some(coverage) = self
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
        if !coverage.fits(graph.size.width, graph.size.height) {
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
                self.report_action_error(&error);
                return;
            }
        }
        if let Err(error) = phototux_canvas::ensure_mask(id) {
            self.report_gpu("paste mask ensure", &error);
            return;
        }
        if let Err(error) = phototux_canvas::write_mask_r8(id, coverage.coverage()) {
            self.report_gpu("paste mask upload", &error);
            return;
        }
        self.recomposite();
        self.sync_from_engine();
        self.emit_layer_fields();
        self.engine.announce("Mask pasted onto active layer");
        self.notify(NoticeLevel::Info, "Pasted mask onto active layer");
        self.publish_announcement();
    }

    fn push_os_clipboard_rgba(image: &crate::clipboard::ImagePayload) -> Result<(), String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard
            .set_image(arboard::ImageData {
                width: image.width() as usize,
                height: image.height() as usize,
                bytes: std::borrow::Cow::Borrowed(image.rgba()),
            })
            .map_err(|e| e.to_string())
    }

    /// Push coverage to the OS as a greyscale image, so external apps see it.
    fn push_os_clipboard_gray(coverage: &crate::clipboard::CoveragePayload) -> Result<(), String> {
        let preview = crate::clipboard::ImagePayload::new(
            coverage.width(),
            coverage.height(),
            coverage.to_gray_rgba(),
        )
        .map_err(|refusal| refusal.message("Copy"))?;
        Self::push_os_clipboard_rgba(&preview)
    }

    /// Read an image from the OS clipboard, if it is one we may carry.
    ///
    /// Anything the OS offers is untrusted: the dimensions and the buffer come
    /// from another process and need not agree, so this validates rather than
    /// assuming.
    fn pull_os_clipboard_rgba() -> Option<crate::clipboard::ImagePayload> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let img = clipboard.get_image().ok()?;
        let width = u32::try_from(img.width).ok()?;
        let height = u32::try_from(img.height).ok()?;
        crate::clipboard::ImagePayload::new(width, height, img.bytes.into_owned()).ok()
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
        self.notify(
            NoticeLevel::Info,
            format!(
                "Soft-proof: display profile ({})",
                self.display_profile_name
            ),
        );
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
        self.notify(NoticeLevel::Info, format!("Brush preset: {}", preset.name));
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
        let Some(image) = payload else {
            self.fail_io("Paste", "clipboard empty");
            return;
        };
        // Checked before the command runs. The upload is validated at the GPU
        // too, but by then the paste has already committed a layer — the user
        // was left with an empty "Pasted" layer and a texture error instead of
        // a refusal.
        let Some(size) = self.engine.graph.as_ref().map(|g| g.size) else {
            self.fail_io("Paste", "no document");
            return;
        };
        if !image.fits(size.width, size.height) {
            self.fail_io("Paste", "clipboard image size does not match document");
            return;
        }
        match self.invoke_command(
            command_id::CLIPBOARD_PASTE_LAYER,
            CommandArgs::PasteLayer {
                name: "Pasted".into(),
            },
        ) {
            Ok(()) => {
                if let Some(id) = self.engine.graph.as_ref().and_then(|g| g.active_id()) {
                    if let Err(error) = phototux_canvas::write_layer_rgba(id, image.rgba()) {
                        self.report_gpu("paste upload", &error);
                        return;
                    }
                    self.recomposite();
                    self.notify(NoticeLevel::Info, "Pasted layer");
                }
            }
            Err(error) => self.report_gpu("paste", &error.to_string()),
        }
    }

    #[qslot]
    fn add_group_layer(&mut self) {
        if let Err(error) = self.invoke_command(command_id::LAYER_GROUP, CommandArgs::None) {
            self.report_action_error(&error);
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
            self.report_action_error(&error);
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

    /// Recolour the active shape layer.
    ///
    /// Colours arrive as hex because that is what the shell's other colour
    /// fields speak, and refusing an unparseable one is better than defaulting
    /// it: a typo would otherwise silently repaint the shape black.
    #[qslot]
    fn set_shape_appearance(
        &mut self,
        fill_hex: String,
        stroke_hex: String,
        stroke_width: f32,
        filled: bool,
        stroked: bool,
    ) {
        let Some(fill_rgba) = phototux_engine::ColorState::from_hex(&fill_hex) else {
            self.notify(
                NoticeLevel::Warning,
                format!("{fill_hex} is not a colour — try #RRGGBB."),
            );
            return;
        };
        let Some(stroke_rgba) = phototux_engine::ColorState::from_hex(&stroke_hex) else {
            self.notify(
                NoticeLevel::Warning,
                format!("{stroke_hex} is not a colour — try #RRGGBB."),
            );
            return;
        };
        if let Err(error) = self.invoke_command(
            command_id::SHAPE_SET_APPEARANCE,
            CommandArgs::ShapeSetAppearance {
                appearance: phototux_engine::ShapeAppearance {
                    fill_rgba,
                    stroke_rgba,
                    stroke_width,
                    filled,
                    stroked,
                },
            },
        ) {
            // A recolour to the value it already holds is refused by design —
            // every slider release would otherwise push a history entry that
            // changed nothing — and is not worth a toast.
            if !matches!(
                error,
                phototux_engine::CommandError::Rejected("that is already how it is drawn")
            ) {
                self.report_action_error(&error);
            }
        }
    }

    #[qslot]
    fn add_text_layer(&mut self, text: String) {
        if let Err(error) =
            self.invoke_command(command_id::TEXT_CREATE, CommandArgs::TextCreate { text })
        {
            self.report_action_error(&error);
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
            self.notify(NoticeLevel::Warning, "Select a text layer to bake it.");
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
            self.report_action_error(&error);
            return;
        }
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
            self.report_gpu("bake text upload", &error);
            return;
        }
        self.recomposite();
        self.engine
            .announce("Text baked to pixels — editable text discarded");
        self.notify(
            NoticeLevel::Info,
            "Text baked to pixels — editable text discarded",
        );
    }

    /// Align or distribute the object selection (`op`: an [`AlignOp`] key).
    ///
    /// The measuring happens here rather than in the engine because every
    /// layer is document-sized: the only edges worth aligning are the ones
    /// around each layer's visible pixels, and the pixels live on the GPU.
    /// Once measured, the engine decides where everything goes.
    #[qslot]
    fn align_layers(&mut self, op: String) {
        let Some(op) = AlignOp::parse(&op) else {
            self.notify(
                NoticeLevel::Warning,
                format!("Unknown align operation: {op}"),
            );
            return;
        };
        let targets = self.align_targets();
        if targets.len() < op.min_targets() {
            self.set_status(format!(
                "{} needs {} layers with visible content",
                op.label(),
                op.min_targets()
            ));
            return;
        }
        if let Err(error) = self.invoke_command(
            command_id::LAYER_ALIGN,
            CommandArgs::AlignLayers { op, targets },
        ) {
            self.report_action_error(&error);
        }
    }

    /// Measure what the object selection would move, one target per object.
    ///
    /// A group becomes a single target holding every member, because the
    /// compositor does not pass a group's transform to its children — the only
    /// way to move a group is to move each member by the same amount. Layers
    /// with nothing visible (an empty layer, an adjustment) contribute no box
    /// and are left out rather than aligned to a rectangle that is not there.
    fn align_targets(&mut self) -> Vec<AlignTarget> {
        let Some(graph) = self.engine.graph.as_ref() else {
            return Vec::new();
        };
        let ids = if self.engine.selected_layer_ids.is_empty() {
            graph.active_id().into_iter().collect::<Vec<_>>()
        } else {
            self.engine.selected_layer_ids.clone()
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let mut targets = Vec::new();
        for id in ids {
            let Some(layer) = graph.get(id) else {
                continue;
            };
            let members: Vec<LayerId> = if layer.kind == LayerKind::Group {
                graph
                    .layers()
                    .iter()
                    .filter(|l| l.parent == Some(id))
                    .map(|l| l.id)
                    .collect()
            } else {
                vec![id]
            };
            if let Some(bounds) = self.union_placed_bounds(&members, w, h) {
                targets.push(AlignTarget { bounds, members });
            }
        }
        targets
    }

    /// Combined document-space box of `ids`, or `None` when none has content.
    fn union_placed_bounds(
        &self,
        ids: &[LayerId],
        w: u32,
        h: u32,
    ) -> Option<phototux_engine::Rect> {
        let graph = self.engine.graph.as_ref()?;
        let mut boxes = Vec::new();
        for id in ids {
            let Some(layer) = graph.get(*id) else {
                continue;
            };
            let Ok((lw, lh, pixels)) = phototux_canvas::read_layer_rgba(*id) else {
                continue;
            };
            if let Some(source) = phototux_engine::content_bounds(&pixels, lw, lh) {
                boxes.push(phototux_engine::placed_bounds(
                    source,
                    layer.transform,
                    w,
                    h,
                ));
            }
        }
        (!boxes.is_empty()).then(|| phototux_engine::align_frame(&boxes, boxes[0]))
    }

    /// Wrap the active layer's pixels as a smart object.
    ///
    /// The capture happens here because the pixels are on the GPU: the engine
    /// records that a source of this size exists and where it is placed, and
    /// the buffer itself stays host-side under the layer's id.
    #[qslot]
    fn convert_to_smart_object(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        let name = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let Ok((width, height, pixels)) = phototux_canvas::read_layer_rgba(id) else {
            self.notify(
                NoticeLevel::Warning,
                "That layer has no pixels to wrap — only a pixel layer can become a smart object.",
            );
            return;
        };
        let content = phototux_engine::SmartObjectContent::embedded(
            name,
            format!("smart-{}", id.0),
            width,
            height,
        );
        // Stored before the command, not after: invoking it republishes the
        // inspector projection, which asks whether this layer's source is
        // held. Inserting afterwards meant the panel's first look at a brand
        // new smart object reported its original pixels missing.
        self.smart_sources.insert(
            id,
            phototux_engine::SmartSource {
                width,
                height,
                pixels,
            },
        );
        match self.invoke_command(
            command_id::SMART_CREATE,
            CommandArgs::SmartCreate {
                content: Box::new(content),
            },
        ) {
            Ok(()) => self.notify(
                NoticeLevel::Info,
                "Wrapped as a smart object — transforms now re-apply to the original.",
            ),
            Err(error) => {
                self.smart_sources.remove(&id);
                self.report_action_error(&error);
            }
        }
    }

    /// Move, scale or rotate the active smart object, non-destructively.
    #[qslot]
    fn set_smart_placement(
        &mut self,
        translate_x: f32,
        translate_y: f32,
        scale: f32,
        rotation_deg: f32,
    ) {
        if let Err(error) = self.invoke_command(
            command_id::SMART_SET_PLACEMENT,
            CommandArgs::SmartSetPlacement {
                placement: phototux_engine::LayerTransform {
                    translate_x,
                    translate_y,
                    scale_x: scale,
                    scale_y: scale,
                    rotation_deg,
                },
            },
        ) {
            // Placing it where it already is is refused by design, so that the
            // slider settling back on its own value does not push history.
            if !matches!(
                error,
                phototux_engine::CommandError::Rejected("that is already where it sits")
            ) {
                self.report_action_error(&error);
            }
        }
    }

    /// Return the active smart object to where its source sits.
    #[qslot]
    fn reset_smart_placement(&mut self) {
        self.set_smart_placement(0.0, 0.0, 1.0, 0.0);
    }

    /// Bake the active smart object to ordinary pixels.
    #[qslot]
    fn rasterize_smart_object(&mut self) {
        let Some(id) = self.active_id() else {
            return;
        };
        match self.invoke_command(command_id::SMART_RASTERIZE, CommandArgs::None) {
            Ok(()) => {
                // The source is deliberately *not* dropped. Rasterizing is
                // undoable, and an undo puts the kind and the payload back —
                // throwing the pixels away here left the restored smart object
                // describing a source that no longer existed. It goes when the
                // document does, like the layer payloads history holds.
                let _ = id;
                self.notify(
                    NoticeLevel::Info,
                    "Rasterized to pixels. Undo restores the smart object.",
                );
            }
            Err(error) => self.report_action_error(&error),
        }
    }

    /// Re-render a smart object from its source at its current placement.
    ///
    /// Restore, then transform. Baking the *displayed* pixels again would
    /// compose this placement with the last one, which is what an ordinary
    /// layer does and what a smart object exists not to do.
    fn place_smart_object(&mut self, id: LayerId) {
        let Some(source) = self.smart_sources.get(&id).cloned() else {
            self.notify(
                NoticeLevel::Warning,
                "This smart object's original pixels are missing, so it cannot be re-placed.",
            );
            return;
        };
        let Some(placement) = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .and_then(|l| l.smart.as_ref())
            .map(|smart| smart.placement)
        else {
            return;
        };
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &source.pixels) {
            self.report_gpu("smart object restore", &error);
            return;
        }
        if !placement.is_identity()
            && let Some(error) = self.bake_smart_placement(id, placement)
        {
            self.report_gpu("smart object placement", &error);
            return;
        }
        self.recomposite();
    }

    /// Apply `placement` to the layer's current pixels, reporting any failure.
    fn bake_smart_placement(
        &mut self,
        id: LayerId,
        placement: phototux_engine::LayerTransform,
    ) -> Option<String> {
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        phototux_canvas::bake_layer_transform(id, placement, &layers)
            .err()
            .map(|error| error.to_string())
    }

    /// Create a shape layer (`kind`: rect|ellipse|polygon|gradient|line|live).
    #[qslot]
    fn add_shape_layer(&mut self, kind: String) {
        let Some(preset) = ShapePreset::parse(&kind) else {
            // Refused rather than defaulted to a rectangle: a shape the user
            // did not ask for is a document mutation they then have to notice.
            // `shape_create_actions_name_a_known_preset` keeps the shipped
            // callers out of this branch.
            self.notify(NoticeLevel::Warning, format!("Unknown shape: {kind}"));
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let doc_w = graph.size.width;
        let doc_h = graph.size.height;
        let content = preset.content(doc_w, doc_h);
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
                match phototux_engine::rasterize_shape_content(&content, doc_w, doc_h) {
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
        let combined = match phototux_engine::rasterize_shape_content(&preserved, w, h) {
            Ok(p) => p,
            Err(_error) => {
                // Fallback: pure raster boolean bake.
                let pixels_a = match phototux_engine::rasterize_shape_content(&content_a, w, h) {
                    Ok(p) => p,
                    Err(e) => {
                        self.report_gpu("shape boolean", &e);
                        return;
                    }
                };
                let pixels_b = match phototux_engine::rasterize_shape_content(&content_b, w, h) {
                    Ok(p) => p,
                    Err(e) => {
                        self.report_gpu("shape boolean", &e);
                        return;
                    }
                };
                match phototux_engine::boolean_rgba8(&pixels_a, &pixels_b, op) {
                    Ok(p) => {
                        self.notify(
                            NoticeLevel::Warning,
                            format!(
                                "Boolean {} (raster bake; vector path unavailable)",
                                op.as_str()
                            ),
                        );
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
            self.notify(
                NoticeLevel::Info,
                format!("Boolean {} (vector-preserving)", op.as_str()),
            );
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
        let pixels = match phototux_engine::rasterize_shape_content(&content, w, h) {
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
            self.notify(
                NoticeLevel::Warning,
                "Select a shape layer to rasterize it.",
            );
            return;
        }
        let Some(content) = layer.shape.clone() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        let pixels = match phototux_engine::rasterize_shape_content(&content, w, h) {
            Ok(p) => p,
            Err(error) => {
                self.report_gpu("rasterize shape", &error);
                return;
            }
        };
        if let Err(error) = self.invoke_command(command_id::SHAPE_RASTERIZE, CommandArgs::None) {
            self.report_action_error(&error);
            return;
        }
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
            self.report_gpu("rasterize shape upload", &error);
            return;
        }
        self.recomposite();
        self.notify(NoticeLevel::Info, "Shape rasterized to pixels");
    }

    /// Morphological / feather modify of the live selection mask (`op`: feather|expand|contract).
    #[qslot]
    fn modify_selection(&mut self, op: String, radius: i32) {
        // QML has no enums and no unsigned integers, so both arguments arrive
        // as the widest thing it can express. Narrow them here and let the
        // rest of the path carry types that cannot say "unknown op".
        let Some(parsed) = SelectionModifyOp::parse(&op) else {
            self.notify(NoticeLevel::Warning, format!("Unknown selection op: {op}"));
            return;
        };
        let Ok(radius) = u32::try_from(radius) else {
            return;
        };
        self.apply_selection_modify(parsed, radius);
    }

    /// Run a selection-channel edit that has already been named and sized.
    ///
    /// Separate from the slot so the action registry path can reach it with
    /// the op it parsed, rather than rendering the op back to a string for the
    /// slot to parse a second time.
    /// Choose the gradient shape; an unknown name is ignored rather than
    /// falling back, since sweeping a shape nobody asked for is an edit the
    /// user has to notice and undo.
    #[qslot]
    fn set_gradient_kind(&mut self, name: String) {
        let Some(kind) = GradientKind::parse(&name) else {
            return;
        };
        if self.gradient_kind == kind.as_str() {
            return;
        }
        self.gradient_kind = kind.as_str().to_owned();
        self.gradient_kind_changed();
    }

    /// Anchor the clone stamp at a document point.
    ///
    /// Alt-click while the clone tool is active. The offset is fixed when the
    /// next stroke begins, so the copy stays aligned with the original rather
    /// than following the cursor.
    #[qslot]
    fn set_clone_anchor(&mut self, doc_x: f32, doc_y: f32) {
        self.send_paint(EngineCommand::SetCloneAnchor { x: doc_x, y: doc_y });
        self.notify(
            NoticeLevel::Info,
            format!("Clone source at {:.0}, {:.0}", doc_x, doc_y),
        );
    }

    #[qslot]
    fn set_selection_tolerance(&mut self, value: f32) {
        let value = value.clamp(0.0, 1.0);
        if (self.selection_tolerance - value).abs() < f32::EPSILON {
            return;
        }
        self.selection_tolerance = value;
        self.selection_tolerance_changed();
    }

    /// Select by colour at a document pixel — the magic wand and colour range.
    ///
    /// `contiguous` is the only difference between the two tools: the wand
    /// floods from the seed, colour range takes every matching pixel.
    #[qslot]
    fn color_select_at(&mut self, doc_x: f32, doc_y: f32, contiguous: bool) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        if doc_x < 0.0 || doc_y < 0.0 {
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "guarded non-negative above and bounds-checked below"
        )]
        let (x, y) = (doc_x as u32, doc_y as u32);
        if x >= w || y >= h {
            return;
        }
        let Some(layer) = graph.active_id() else {
            return;
        };
        let tolerance = self.selection_tolerance;
        let combine = self.engine.selection.combine;
        if !self.commit_selection_edit("color select", || {
            phototux_canvas::selection_color_select(layer, x, y, tolerance, contiguous, combine)
        }) {
            return;
        }
        let _ = self.invoke_command(
            command_id::SELECTION_COLOR_SELECT,
            CommandArgs::SelectionColorSelect {
                contiguous,
                tolerance,
                combine,
            },
        );
    }

    fn apply_selection_modify(&mut self, op: SelectionModifyOp, radius: u32) {
        let Ok(mask) = phototux_canvas::selection_snapshot() else {
            return;
        };
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        let (w, h) = (graph.size.width, graph.size.height);
        match op.apply(w, h, &mask, radius) {
            Ok(bytes) => {
                if !self.commit_selection_edit("modify selection", || {
                    phototux_canvas::selection_restore(&bytes)
                }) {
                    return;
                }
                let _ = self.invoke_command(
                    command_id::SELECTION_MODIFY,
                    CommandArgs::SelectionModify { op, radius },
                );
                self.notify(
                    NoticeLevel::Info,
                    format!("Selection {} ({radius}px)", op.as_str()),
                );
            }
            Err(error) => self.report_gpu("modify selection", &error),
        }
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
                self.notify(NoticeLevel::Info, "Path stroked to new layer");
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
            self.report_action_error(&error);
        }
    }

    /// Write one editor slot of the active adjustment, leaving the rest alone.
    ///
    /// The chrome sends one slot rather than the whole array because that is
    /// what a slider drag means, and because the slot count differs per kind:
    /// a panel that rebuilt the array would have to know which kind it is
    /// editing, which is exactly the knowledge the editor no longer carries.
    #[qslot]
    fn set_adjustment_slot(&mut self, index: i32, value: f32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let Some(params) = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .and_then(|l| l.adjustment.as_ref())
        else {
            return;
        };
        if index >= params.editor_slots().len() {
            return;
        }
        let mut slots = params.slots();
        slots[index] = value;
        let _ = self.invoke_command(
            command_id::FILTER_SET_PARAMETERS,
            CommandArgs::FilterParameters { slots },
        );
    }

    /// Move one Blend If handle, leaving the other seven alone.
    ///
    /// `range` is 0 for this layer and 1 for the underlying composite; `stop`
    /// indexes `BlendRange::STOP_LABELS`. Values arrive on the 0–255 scale the
    /// sliders show and are normalised here, which keeps the panel free of the
    /// engine's units.
    #[qslot]
    fn set_blend_if_stop(&mut self, range: i32, stop: i32, value: f32) {
        let Ok(stop) = usize::try_from(stop) else {
            return;
        };
        if stop >= phototux_engine::BlendRange::STOP_LABELS.len() {
            return;
        }
        let Some(mut blend_if) = self.active_blend_if() else {
            return;
        };
        let target = if range == 0 {
            &mut blend_if.this_layer
        } else {
            &mut blend_if.underlying
        };
        let mut stops = target.stops();
        stops[stop] = (value / 255.0).clamp(0.0, 1.0);
        *target = phototux_engine::BlendRange::from_stops(stops);
        self.apply_blend_if(blend_if);
    }

    /// Choose which channel the blend ranges read.
    #[qslot]
    fn set_blend_if_channel(&mut self, channel: String) {
        let Some(parsed) = phototux_engine::BlendIfChannel::parse(&channel) else {
            self.notify(
                NoticeLevel::Warning,
                format!("Unknown blend channel: {channel}"),
            );
            return;
        };
        let Some(mut blend_if) = self.active_blend_if() else {
            return;
        };
        blend_if.channel = parsed;
        self.apply_blend_if(blend_if);
    }

    /// Return the active layer to the ranges that hide nothing.
    ///
    /// Reachable in one click because Blend If is easy to leave half-set: two
    /// handles nudged off their stops hide part of a layer with no other
    /// symptom, and hunting eight sliders back to their ends is worse than a
    /// button.
    #[qslot]
    fn reset_blend_if(&mut self) {
        let Some(current) = self.active_blend_if() else {
            return;
        };
        self.apply_blend_if(phototux_engine::BlendIf {
            channel: current.channel,
            ..Default::default()
        });
    }

    fn active_blend_if(&self) -> Option<phototux_engine::BlendIf> {
        let graph = self.engine.graph.as_ref()?;
        graph.get(graph.active_id()?).map(|layer| layer.blend_if)
    }

    fn apply_blend_if(&mut self, blend_if: phototux_engine::BlendIf) {
        let _ = self.invoke_command(
            command_id::LAYER_SET_BLEND_IF,
            CommandArgs::SetBlendIf { blend_if },
        );
    }

    /// Write one scalar slot of one layer style, leaving the rest alone.
    #[qslot]
    fn set_layer_style_slot(&mut self, index: i32, slot: i32, value: f32) {
        let (Ok(index), Ok(slot)) = (usize::try_from(index), usize::try_from(slot)) else {
            return;
        };
        let Some(style) = self.active_layer_style(index) else {
            return;
        };
        if slot >= style.editor_slots().len() {
            return;
        }
        let mut slots = style.slots();
        slots[slot] = value;
        let _ = self.invoke_command(
            command_id::STYLE_SET_PARAMS,
            CommandArgs::LayerStyleParams { index, slots },
        );
    }

    /// Replace one colour of one layer style.
    #[qslot]
    fn set_layer_style_color(&mut self, index: i32, color: i32, r: f32, g: f32, b: f32) {
        let (Ok(index), Ok(color_index)) = (usize::try_from(index), usize::try_from(color)) else {
            return;
        };
        let _ = self.invoke_command(
            command_id::STYLE_SET_COLOR,
            CommandArgs::LayerStyleColor {
                index,
                color_index,
                rgba: [r, g, b, 1.0],
            },
        );
    }

    #[qslot]
    fn set_layer_style_enabled(&mut self, index: i32, enabled: bool) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let _ = self.invoke_command(
            command_id::STYLE_SET_ENABLED,
            CommandArgs::LayerStyleEnabled { index, enabled },
        );
    }

    #[qslot]
    fn remove_layer_style(&mut self, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let _ = self.invoke_command(
            command_id::STYLE_REMOVE,
            CommandArgs::LayerStyleIndex { index },
        );
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
                self.report_action_error(&error);
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
        self.publish_announcement();
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
        let (width, height, mut rgba) = match phototux_canvas::read_layer_rgba(id) {
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
        // One definition of mask semantics, shared with the composite shader:
        // soften by feather over the whole mask, then apply the four per-sample
        // parameters to each texel. The copy that used to live here dropped
        // contrast and shift, and neither side softened at all, so baking
        // produced pixels that did not match the canvas.
        let softened = mask_meta.feathered(width, height, &r8);
        if mask_meta.bake_into_rgba8(&mut rgba, &softened).is_err() {
            self.report_gpu("apply mask", "mask/layer size mismatch");
            return;
        }
        if let Err(error) = phototux_canvas::write_layer_rgba(id, &rgba) {
            self.report_gpu("apply mask", &error);
            return;
        }
        if let Err(error) = self.invoke_command(command_id::MASK_DELETE, CommandArgs::None) {
            self.report_action_error(&error);
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
        self.publish_announcement();
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
        if !self.commit_selection_edit("mask to selection", || {
            phototux_canvas::selection_restore(&r8)
        }) {
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
        self.sync_selection_fields();
        self.emit_selection_fields();
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
        self.publish_announcement();
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
            self.report_action_error(&error);
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
            self.report_action_error(&error);
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
            self.notify(
                NoticeLevel::Warning,
                format!("Unknown guide orientation: {orientation}"),
            );
            return;
        };
        let position = self.engine.guides.snap_value(position, orient);
        self.engine.guides.add_guide(Guide {
            orientation: orient,
            position,
        });
        self.sync_guides_fields();
        self.emit_guides_fields();
        self.notify(NoticeLevel::Info, format!("Guide added at {position:.0}px"));
    }

    #[qslot]
    fn clear_guides(&mut self) {
        self.engine.guides.clear_guides();
        self.sync_guides_fields();
        self.emit_guides_fields();
        self.notify(NoticeLevel::Info, "Guides cleared");
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
        session.constrain_aspect = constrain;
        session.draft = LayerTransform {
            translate_x,
            translate_y,
            scale_x,
            scale_y,
            rotation_deg,
        }
        .with_usable_scale(constrain);
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

    /// Fold a committed transform into a smart object's placement.
    ///
    /// Reports whether it did — the caller bakes only if it did not.
    ///
    /// A smart object exists so that a transform is re-applied to the original
    /// pixels rather than accumulated on the result. Baking the tool's draft
    /// into the layer instead left the placement saying something else, and
    /// the next nudge of the scale slider restored the source and silently
    /// discarded the transform the user had just committed.
    fn fold_transform_into_placement(
        &mut self,
        session: &phototux_engine::TransformSession,
    ) -> bool {
        let Some(placement) = self
            .engine
            .graph
            .as_ref()
            .and_then(|graph| graph.get(session.layer_id))
            .filter(|layer| layer.kind == LayerKind::SmartObject)
            .and_then(|layer| layer.smart.as_ref())
            .map(|smart| smart.placement)
        else {
            return false;
        };
        // The compositor has been previewing the draft through the layer's own
        // transform. Put that back before re-placing, or the folded placement
        // is applied on top of a preview of itself.
        if let Some(graph) = self.engine.graph.as_mut()
            && let Some(layer) = graph.get_mut(session.layer_id)
        {
            layer.transform = session.baseline;
        }
        self.engine.transform_session = None;
        if let Err(error) = self.invoke_command(
            command_id::SMART_SET_PLACEMENT,
            CommandArgs::SmartSetPlacement {
                placement: placement.folded_with(session.draft),
            },
        ) {
            self.report_action_error(&error);
        }
        self.sync_transform_fields();
        true
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
        if self.fold_transform_into_placement(&session) {
            self.emit_transform_fields();
            return;
        }
        if self
            .commit_layer_edit("transform bake", |layers| {
                phototux_canvas::bake_layer_transform(session.layer_id, session.draft, layers)
                    .map(|ms| ((), ms))
            })
            .is_some()
        {
            let _ = self.invoke_command(command_id::RASTER_TRANSFORM_COMMIT, CommandArgs::None);
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
        // Crop bakes into untransformed pixels, so the copies it hands the GPU
        // are flattened first.
        let cropped = self.commit_layer_edit("crop", |layers| {
            let mut flat = layers.to_vec();
            for layer in &mut flat {
                layer.transform = LayerTransform::identity();
            }
            phototux_canvas::crop_document(rect, &flat)
        });
        if let Some(new_size) = cropped {
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
        if self
            .commit_layer_edit("flip", |layers| {
                phototux_canvas::flip_layer(id, horizontal, layers).map(|ms| ((), ms))
            })
            .is_some()
        {
            let _ = self.invoke_command(
                command_id::RASTER_FLIP,
                CommandArgs::RasterFlip { horizontal },
            );
        }
    }

    #[qslot]
    fn rotate_canvas_90_cw(&mut self) {
        if !self.engine.has_document {
            return;
        }
        if self
            .commit_layer_edit("rotate canvas", phototux_canvas::rotate_canvas_90_cw)
            .is_some()
        {
            self.clear_selection_stacks();
            let _ = self.invoke_command(command_id::DOCUMENT_ROTATE_90, CommandArgs::None);
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
            self.notify(NoticeLevel::Warning, "Fill needs an unlocked raster layer.");
            return;
        };
        let fg = self.engine.colors.foreground;
        let use_selection = self.engine.selection.active;
        if self
            .commit_layer_edit("fill", |layers| {
                phototux_canvas::fill_layer(id, fg, layers, use_selection).map(|ms| ((), ms))
            })
            .is_some()
        {
            let _ = self.invoke_command(command_id::RASTER_FILL, CommandArgs::None);
        }
    }

    #[qslot]
    fn commit_linear_gradient(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let Some(id) = self.active_raster_paintable() else {
            self.notify(
                NoticeLevel::Warning,
                "Gradient needs an unlocked raster layer.",
            );
            return;
        };
        let c0 = self.engine.colors.foreground;
        let c1 = self.engine.colors.background;
        let use_selection = self.engine.selection.active;
        let kind = GradientKind::parse(&self.gradient_kind).unwrap_or_default();
        if self
            .commit_layer_edit("gradient", |layers| {
                phototux_canvas::apply_gradient(
                    GradientRamp {
                        kind,
                        start: [x0, y0],
                        end: [x1, y1],
                        start_rgba: c0,
                        end_rgba: c1,
                    },
                    id,
                    layers,
                    use_selection,
                )
                .map(|ms| ((), ms))
            })
            .is_some()
        {
            let _ = self.invoke_command(command_id::RASTER_GRADIENT, CommandArgs::None);
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
                self.notify(
                    NoticeLevel::Info,
                    format!(
                        "Sampled {}",
                        phototux_engine::ColorState::to_hex([r, g, b, 1.0])
                    ),
                );
            }
            Err(error) => self.report_gpu("eyedropper", &error),
        }
    }
}
