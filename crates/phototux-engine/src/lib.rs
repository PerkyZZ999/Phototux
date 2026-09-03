//! Pure document/session types — no Qt (DR-025, DR-002, DR-004).

mod actions;
mod align;
mod arrange;
mod atspi_map;
mod blend_if;
mod brush_preset;
mod budget_harness;
mod camera;
mod cancel;
mod color;
mod color_mgmt;
mod command;
mod command_conformance;
mod command_meta;
mod commands;
mod cpu_composite;
mod dock;
mod document;
mod document_registry;
mod effective_pref;
mod error;
mod filter_plan;
mod filter_preview;
mod gradient;
mod guides;
mod history;
mod host_request;
mod inspector;
mod layer;
mod layer_row;
mod layer_style;
mod notice;
mod paths;
mod render_plan;
mod selection;
mod shape_bake;
mod shape_boolean;
mod shape_preset;
mod shell;
mod snapshot_publish;
mod stroke;
mod stroke_journal;
mod text_bake;
mod thumbnail;
mod transform;
mod undo;
mod workspace;
mod workspace_preset;

pub use actions::{
    ActionDescriptor, action_by_id, action_shortcuts_json, actions_for_context, actions_json,
    chord_map_from_action_shortcuts, context_actions_json, default_action_shortcuts,
    default_actions, default_shortcut_map, effective_action_shortcuts, effective_shortcuts_json,
    normalize_shortcut, resolve_shortcut, shortcut_conflict, shortcuts_json,
};
pub use align::{
    AlignAxis, AlignOp, AlignTarget, align_frame, align_offsets, align_ops_json, content_bounds,
    placed_bounds,
};
pub use arrange::ArrangeOp;
pub use atspi_map::{
    AtspiProjectionNode, SemanticRole, project_semantic_tree, project_semantic_tree_json,
};
pub use blend_if::{BlendIf, BlendIfChannel, BlendRange, blend_if_channels_json};
pub use brush_preset::{BrushPreset, BrushPresetLibrary};
pub use budget_harness::{
    BudgetSample, measure_cpu_composite_8x256, measure_cpu_composite_10x512, run_soft_ci_suite,
};
pub use camera::{Camera2D, FpsTracker, Rect};
pub use cancel::CancelToken;
pub use color::{ColorState, SampleSource};
pub use color_mgmt::{
    ConvertPlan, DocumentColorState, MAX_ICC_BYTES, convert_rgba8_profile, minimal_icc_fixture,
    validate_icc_profile,
};
pub use command::{EngineCommand, EngineEvent};
pub use command_meta::ALL as COMMAND_META_ALL;
pub use command_meta::{
    CommandMeta, CommandScope, ConflictPolicy, MutationClass, UndoPolicy,
    meta_for as command_meta_for,
};
pub use commands::{
    CommandArgs, CommandEffects, CommandError, HostFollowUp, HostHistoryAction, command_id,
};
pub use cpu_composite::{CpuLayerRef, blend_rgb, composite_rgba8};
pub use dock::{DockTopology, FloatingPanelPlacement, ScreenRect};
pub use document::{CanvasAnchor, DocumentGraph, ExtensionBlob, GRAPH_SCHEMA_VERSION, MAX_LAYERS};
pub use document_registry::{
    DocumentRegistry, MAX_OPEN_DOCUMENTS, OpenDocumentId, ParkedDocument, SmartSource,
    max_open_documents,
};
pub use effective_pref::{PrefSource, resolve_layered, values_are_mixed};
pub use error::DocumentError;
pub use filter_plan::{FilterPlan, FilterPlanNode};
pub use filter_preview::{
    FilterPreviewSession, filter_catalog_json, gallery_effect_kinds, kind_is_supported,
};
pub use gradient::{GradientKind, GradientRamp};
pub use guides::{Guide, GuideOrientation, ViewGuides};
pub use history::{HistoryEntry, HistoryKind, HistoryRow, HistoryService};
pub use host_request::HostRequest;
pub use inspector::{InspectorSubject, subjects_json as inspector_subjects_json};
pub use layer::{
    AdjustmentParams, BlendMode, FillContent, FilterEffect, FilterParams, Layer, LayerId,
    LayerKind, LayerMask, LayerTransform, LockFlags, MAX_ADJUSTMENT_SLOTS, MAX_BLUR_RADIUS,
    PaintTarget, ShapeAppearance, ShapeBooleanPartner, ShapeContent, ShapeGradient,
    SmartObjectContent, TextContent, VectorMask, blend_modes_json,
};
pub use layer_row::{LayerRow, layer_rows};
pub use layer_style::{LayerStyle, StrokePosition, apply_styles_rgba8, layer_styles_json};
pub use notice::{Notice, NoticeLevel, NoticeQueue};
pub use paths::{
    PathDocument, PathPoint, VectorPath, ellipse_path, fill_gradient_even_odd, polygon_path,
    rasterize_shape_rgba8, rect_path, stroke_path_rgba8,
};
pub use render_plan::{
    BevelPlan, ColorOverlayPlan, GradientOverlayPlan, LayerRenderPlan, ShadowPlan, StrokePlan,
};
pub use selection::{
    SelectionCombine, SelectionEllipse, SelectionModifyOp, SelectionRect, SelectionShape,
    SelectionState, border_mask_r8, color_select_mask, contract_mask_r8, expand_mask_r8,
    feather_mask_r8, parse_selection_modify_arg, smooth_mask_r8,
};
pub use shape_bake::{rasterize_shape_content, rgba_f32_to_u8};
pub use shape_boolean::{BooleanOp, boolean_rgba8};
pub use shape_preset::ShapePreset;
pub use shell::{
    AdjustmentParamRange, DisclosureBadge, DisclosureGroupDescriptor, InspectorState,
    PanelDescriptor, PanelHeight, ToolDescriptor, adjustment_editor_ranges,
    adjustment_editor_ranges_json, adjustment_labels_json, default_disclosure_groups,
    default_panels, default_tools, disclosure_groups_json, essentials_panel_visibility,
    inspector_badges, inspector_badges_json, panels_json, tool_slots, tool_slots_json, tools_json,
};
pub use snapshot_publish::{
    MAX_SNAPSHOT_BYTES, PixelSnapshot, SnapshotError, SnapshotPublisher, solid_layer_rgba,
};
pub use stroke::{
    BrushParams, BrushTextureKind, Dab, DabMode, DabSource, StrokeBuilder, dab_coverage,
    paint_dabs_rgba, paint_dabs_rgba_from, stamp_dab_rgba, stamp_dab_rgba_from,
};
pub use stroke_journal::{
    BrushParamsSnapshot, DabSnapshot, JournalStroke, StrokeJournal, StrokeSample,
};
pub use text_bake::bake_text_rgba8;
pub use thumbnail::{Thumbnail, downsample_rgba8};
pub use transform::{Affine2, CropRect, ResizeRequest, TransformPreview, TransformSession};
pub use undo::{GraphCommand, UndoStack, actions as undo_actions};
pub use workspace::{WorkspaceFocus, WorkspaceState};
pub use workspace_preset::{
    MAX_USER_WORKSPACE_PRESETS, USER_WORKSPACE_PRESET_PREFIX, WorkspacePreset,
    builtin_workspace_presets, is_user_workspace_preset_id, merged_workspace_presets,
    merged_workspace_presets_json, parse_user_workspace_presets, resolve_workspace_preset,
    slugify_workspace_preset_title, user_workspace_presets_json, workspace_preset_by_id,
    workspace_presets_json,
};

use serde::{Deserialize, Serialize};

/// Pixel dimensions of the open document canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSize {
    pub width: u32,
    pub height: u32,
}

/// The largest edge a document may have, in pixels.
///
/// This is `wgpu::Limits::default().max_texture_dimension_2d`, which is what
/// `phototux_gpu` asks the adapter for. Every layer is a texture of the
/// document's size, so a document wider or taller than this cannot be
/// composited at all — and the failure is silent: wgpu refuses the allocation,
/// the compositor's result texture stays invalid, and every frame after that
/// logs a validation error while the canvas shows nothing. A 20000-pixel
/// document opened, populated its layers panel, and drew a blank rectangle
/// with no message of any kind.
///
/// The number lives here rather than in `phototux_gpu` because the dialogs and
/// the commands are what have to refuse, and they must be able to say so
/// without a device (DR-022). `the_gpu_crate_asks_for_the_limit_the_engine_promises`
/// keeps the two from drifting.
pub const MAX_DOCUMENT_DIMENSION: u32 = 8192;

impl DocumentSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Whether the compositor can hold a document this size.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        self.width >= 1
            && self.height >= 1
            && self.width <= MAX_DOCUMENT_DIMENSION
            && self.height <= MAX_DOCUMENT_DIMENSION
    }

    /// The offending edge, for a message that names a number the user typed.
    #[must_use]
    pub const fn oversized_edge(self) -> Option<u32> {
        if self.width > MAX_DOCUMENT_DIMENSION {
            Some(self.width)
        } else if self.height > MAX_DOCUMENT_DIMENSION {
            Some(self.height)
        } else {
            None
        }
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

/// Named size presets (DR-024). 1080p is the recommended default highlight in UI.
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

/// Tool ids aligned with `assets/icons/ICON_MAP.md`.
pub mod tool_id {
    /// Declares each tool constant and the [`ALL`] table from one list.
    ///
    /// The host has to reject ids it does not recognise, and it used to do
    /// that by restating all seventeen constants in a `matches!`. Two lists of
    /// the same thing drift: a tool added to the module but not the match
    /// would be silently replaced by the brush at the moment the user picked
    /// it. Generating both from one place removes the second list rather than
    /// testing that it agrees.
    macro_rules! tools {
        ($($name:ident => $id:literal),+ $(,)?) => {
            $(pub const $name: &str = $id;)+

            /// Every tool id this build knows, in declaration order.
            pub const ALL: &[&str] = &[$($id),+];
        };
    }

    tools! {
        BRUSH => "tool.brush",
        ERASER => "tool.eraser",
        PAN => "tool.pan",
        ZOOM => "tool.zoom",
        SELECT_RECT => "tool.select.rect",
        SELECT_ELLIPSE => "tool.select.ellipse",
        SELECT_LASSO => "tool.select.lasso",
        SELECT_POLYGON => "tool.select.polygon",
        SELECT_WAND => "tool.select.wand",
        SELECT_COLOR_RANGE => "tool.select.color-range",
        CLONE => "tool.clone",
        DODGE => "tool.dodge",
        BURN => "tool.burn",
        SPONGE => "tool.sponge",
        BLUR => "tool.blur",
        SHARPEN => "tool.sharpen",
        SMUDGE => "tool.smudge",
        MOVE => "tool.move",
        TRANSFORM => "tool.transform",
        CROP => "tool.crop",
        FILL => "tool.fill",
        GRADIENT => "tool.gradient",
        EYEDROPPER => "tool.eyedropper",
        TEXT => "tool.text",
        SHAPE => "tool.shape",
        PATH_EDIT => "tool.path-edit",
    }

    /// Whether `id` names a tool this build ships.
    #[must_use]
    pub fn is_known(id: &str) -> bool {
        ALL.contains(&id)
    }
}

/// Session state: camera + document graph + unified history.
#[derive(Debug)]
pub struct SessionState {
    pub size: DocumentSize,
    pub camera: Camera2D,
    pub brush_size: f32,
    pub brush_hardness: f32,
    pub brush_color: [f32; 4],
    pub active_tool: String,
    pub has_document: bool,
    pub fps: f32,
    pub composite_ms: f32,
    pub stroke_latency_ms: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub graph: Option<DocumentGraph>,
    pub history: HistoryService,
    pub brush: BrushParams,
    pub selection: SelectionState,
    pub transform_session: Option<TransformSession>,
    /// When set, brush/eraser edits this layer's mask instead of pixels.
    pub mask_edit_layer: Option<LayerId>,
    /// Object selection (layer ids) — distinct from pixel selection and edit target (DR-011).
    pub selected_layer_ids: Vec<LayerId>,
    /// Last polite announcement for chrome / a11y projection.
    pub last_announce: String,
    pub colors: ColorState,
    pub guides: ViewGuides,
    pub brush_presets: BrushPresetLibrary,
    pub document_path: Option<String>,
    /// Generation last successfully persisted (save receipt); `None` if never saved.
    pub last_persisted_generation: Option<u64>,
    /// Ephemeral filter gallery preview (not document authority until commit).
    pub filter_preview: Option<FilterPreviewSession>,
    /// Selected anchor index for path-edit tool.
    pub path_edit_anchor: Option<usize>,
    /// Latest bounded CPU pixel snapshot for workers (DR-005 / DR-028 E).
    pub pixel_publisher: SnapshotPublisher,
    /// Document-space dirty rect `[x, y, w, h]` for overlay/composite polish (DR-028 A7).
    pub dirty_rect: Option<[i32; 4]>,
    /// Bumped on pan/zoom so overlays know the full view is invalidated.
    pub overlay_view_generation: u64,
}

/// Immutable metadata lease for render/save coordination (handbook Phase 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentSnapshotLease {
    pub document_id: u128,
    pub generation: u64,
    pub revision: u64,
    pub width: u32,
    pub height: u32,
    pub active_layer: Option<LayerId>,
    pub layer_count: u32,
}

impl Default for SessionState {
    fn default() -> Self {
        let brush = BrushParams::default();
        Self {
            size: SizePreset::P1080.size(),
            camera: Camera2D::default(),
            brush_size: brush.size,
            brush_hardness: brush.hardness,
            brush_color: brush.color,
            active_tool: tool_id::BRUSH.to_owned(),
            has_document: false,
            fps: 0.0,
            composite_ms: 0.0,
            stroke_latency_ms: 0.0,
            viewport_w: 800.0,
            viewport_h: 600.0,
            graph: None,
            history: HistoryService::new(128),
            brush,
            selection: SelectionState::default(),
            transform_session: None,
            mask_edit_layer: None,
            selected_layer_ids: Vec::new(),
            last_announce: String::new(),
            colors: ColorState::default(),
            guides: ViewGuides::default(),
            brush_presets: BrushPresetLibrary::with_defaults(),
            document_path: None,
            last_persisted_generation: None,
            filter_preview: None,
            dirty_rect: None,
            overlay_view_generation: 0,
            path_edit_anchor: None,
            pixel_publisher: SnapshotPublisher::new(),
        }
    }
}

impl SessionState {
    /// Mark a document-space dirty rectangle (union with existing).
    pub fn mark_dirty_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.dirty_rect = Some(match self.dirty_rect {
            None => [x, y, w, h],
            Some([ox, oy, ow, oh]) => {
                let x0 = x.min(ox);
                let y0 = y.min(oy);
                let x1 = (x + w).max(ox + ow);
                let y1 = (y + h).max(oy + oh);
                [x0, y0, x1 - x0, y1 - y0]
            }
        });
    }

    /// Clear dirty rect after a full present/sync.
    pub fn clear_dirty_rect(&mut self) {
        self.dirty_rect = None;
    }

    /// Invalidate overlays for camera changes.
    pub fn bump_overlay_view(&mut self) {
        self.overlay_view_generation = self.overlay_view_generation.saturating_add(1);
        self.dirty_rect = None;
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.camera.set_zoom(zoom);
        self.bump_overlay_view();
    }

    pub fn set_brush_size(&mut self, size: f32) {
        self.brush_size = size.clamp(1.0, 500.0);
        self.brush.size = self.brush_size;
    }

    pub fn set_brush_hardness(&mut self, hardness: f32) {
        self.brush_hardness = hardness.clamp(0.0, 1.0);
        self.brush.hardness = self.brush_hardness;
    }

    pub fn set_brush_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.brush_color = [
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
            a.clamp(0.0, 1.0),
        ];
        self.brush.color = self.brush_color;
        self.colors.set_foreground(self.brush_color);
    }

    pub fn sync_brush_from_tool(&mut self) {
        self.brush.mode = DabMode::for_tool(&self.active_tool);
        self.brush.size = self.brush_size;
        self.brush.hardness = self.brush_hardness;
        self.brush.color = self.brush_color;
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_w = width.max(1.0);
        self.viewport_h = height.max(1.0);
    }

    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        self.camera.pan_by_screen(dx, dy);
        self.bump_overlay_view();
    }

    /// Set the world-space point shown at the viewport center.
    pub fn set_pan(&mut self, world_x: f32, world_y: f32) {
        self.camera.pan_x = world_x;
        self.camera.pan_y = world_y;
        self.bump_overlay_view();
    }

    pub fn zoom_at(&mut self, factor: f32, anchor_x: f32, anchor_y: f32) {
        self.camera
            .zoom_at(factor, anchor_x, anchor_y, self.viewport_w, self.viewport_h);
        self.bump_overlay_view();
    }

    /// Step the zoom one stop in or out, about the viewport centre.
    ///
    /// The pan needs no correction: `pan` *is* the world point drawn at the
    /// viewport centre, so changing only the scale leaves whatever is in the
    /// middle of the view in the middle of the view. That is what Photoshop's
    /// Ctrl+= does, and it is why the step commands do not take an anchor the
    /// way a wheel zoom does.
    pub fn zoom_step(&mut self, zoom_in: bool) {
        let next = self.camera.stepped_zoom(zoom_in);
        self.set_zoom(next);
    }

    pub fn zoom_to_fit(&mut self) {
        self.camera.zoom_to_fit(
            self.size.width as f32,
            self.size.height as f32,
            self.viewport_w,
            self.viewport_h,
            0.08,
        );
    }

    pub fn apply_size(&mut self, size: DocumentSize) {
        // Clamped to what the compositor can hold, not to a round number.
        // The callers refuse an oversized request with a message; this is the
        // last line, so that a caller that forgets still gets a document that
        // draws rather than one that logs a validation error every frame.
        let w = size.width.clamp(1, MAX_DOCUMENT_DIMENSION);
        let h = size.height.clamp(1, MAX_DOCUMENT_DIMENSION);
        self.size = DocumentSize::new(w, h);
        self.has_document = true;
        self.graph = Some(DocumentGraph::new(self.size));
        self.history.clear();
        self.selection.clear();
        self.transform_session = None;
        self.mask_edit_layer = None;
        self.selected_layer_ids.clear();
        self.last_announce.clear();
        self.document_path = None;
        self.last_persisted_generation = None;
        self.pixel_publisher.clear();
        self.sync_object_selection_to_active();
        self.zoom_to_fit();
    }

    pub fn apply_flattened(&mut self, size: DocumentSize, layer_name: impl Into<String>) {
        let width = size.width.clamp(1, MAX_DOCUMENT_DIMENSION);
        let height = size.height.clamp(1, MAX_DOCUMENT_DIMENSION);
        let graph = DocumentGraph::new_flattened(DocumentSize::new(width, height), layer_name);
        self.replace_graph(graph);
    }

    pub fn replace_graph(&mut self, graph: DocumentGraph) {
        self.size = graph.size;
        self.has_document = true;
        self.graph = Some(graph);
        self.history.clear();
        self.selection.clear();
        self.transform_session = None;
        self.mask_edit_layer = None;
        self.selected_layer_ids.clear();
        self.last_announce.clear();
        self.last_persisted_generation = None;
        self.pixel_publisher.clear();
        self.sync_object_selection_to_active();
        self.zoom_to_fit();
    }

    pub fn apply_preset(&mut self, preset: SizePreset) {
        self.apply_size(preset.size());
    }

    pub fn set_active_tool(&mut self, tool: &str) {
        self.active_tool = tool.to_owned();
        self.sync_brush_from_tool();
    }

    pub fn set_stroke_latency_ms(&mut self, ms: f32) {
        self.stroke_latency_ms = ms.max(0.0);
    }

    /// Screen pixel → document coordinates using session camera.
    pub fn screen_to_document(&self, sx: f32, sy: f32) -> (f32, f32) {
        self.camera
            .screen_to_world(sx, sy, self.viewport_w, self.viewport_h)
    }

    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps.max(0.0);
    }

    pub fn set_composite_ms(&mut self, ms: f32) {
        self.composite_ms = ms.max(0.0);
    }

    pub fn layer_count(&self) -> i32 {
        self.graph
            .as_ref()
            .map(|g| g.layer_count() as i32)
            .unwrap_or(0)
    }

    pub fn active_layer_index(&self) -> i32 {
        self.graph
            .as_ref()
            .and_then(|g| g.active_index())
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Compatibility alias used by older call sites.
    pub fn undo(&self) -> &HistoryService {
        &self.history
    }

    pub fn graph_revision(&self) -> u64 {
        self.graph.as_ref().map(|g| g.revision).unwrap_or(0)
    }

    /// Metadata snapshot lease for the open document (no pixel buffers).
    pub fn snapshot_lease(&self) -> Option<DocumentSnapshotLease> {
        let graph = self.graph.as_ref()?;
        Some(DocumentSnapshotLease {
            document_id: graph.document_id,
            generation: graph.generation,
            revision: graph.revision,
            width: graph.size.width,
            height: graph.size.height,
            active_layer: graph.active_id(),
            layer_count: graph.layer_count() as u32,
        })
    }

    /// Record a successful save of `generation`. Clears dirty only when it matches current.
    pub fn mark_persisted(&mut self, generation: u64) -> bool {
        self.last_persisted_generation = Some(generation);
        self.document_generation() == generation
    }

    /// Whether the document has edits newer than the last save receipt.
    pub fn is_dirty_vs_persisted(&self) -> bool {
        match (self.document_generation(), self.last_persisted_generation) {
            (0, _) => false,
            (current, Some(persisted)) => current != persisted,
            (_, None) => self.has_document,
        }
    }

    /// Bump generation after a host-owned pixel commit (stroke/transform/fill).
    pub fn bump_document_generation(&mut self) -> u64 {
        if let Some(graph) = self.graph.as_mut() {
            graph.bump_generation();
            let generation = graph.generation;
            self.pixel_publisher.invalidate_if_stale(generation);
            generation
        } else {
            0
        }
    }

    /// Publish a CPU composite snapshot under the current metadata lease.
    ///
    /// # Errors
    /// Returns [`SnapshotError`] when there is no document, budget is exceeded, or composite fails.
    pub fn publish_pixel_snapshot(
        &mut self,
        layers: &[crate::CpuLayerRef<'_>],
    ) -> Result<std::sync::Arc<PixelSnapshot>, SnapshotError> {
        let lease = self.snapshot_lease().ok_or(SnapshotError::NoDocument)?;
        self.pixel_publisher.publish_composite(lease, layers)
    }

    /// Publish a host-provided composite buffer (e.g. GPU readback).
    ///
    /// # Errors
    /// Returns [`SnapshotError`] on lease/budget/length mismatch.
    pub fn publish_pixel_snapshot_rgba(
        &mut self,
        rgba: Vec<u8>,
    ) -> Result<std::sync::Arc<PixelSnapshot>, SnapshotError> {
        let lease = self.snapshot_lease().ok_or(SnapshotError::NoDocument)?;
        self.pixel_publisher.publish_rgba_buffer(lease, rgba)
    }

    pub fn latest_pixel_snapshot(&self) -> Option<std::sync::Arc<PixelSnapshot>> {
        self.pixel_publisher.latest()
    }

    /// The active layer, if there is a document and it has one.
    /// The active layer's lock flags, or none when there is no layer.
    fn active_lock_flags(&self) -> crate::LockFlags {
        self.active_layer()
            .map(|layer| layer.locks)
            .unwrap_or_default()
    }

    fn active_layer(&self) -> Option<&crate::layer::Layer> {
        self.graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
    }

    /// Mask state of the active layer: `0` none, `1` enabled, `2` disabled.
    ///
    /// A scalar because the shell only ever wanted this one layer's flag. It
    /// used to reach it by splitting the whole stack's flags in QML and
    /// indexing by the active index — three chances to be wrong (a stale
    /// string, an index past the end, a silent `0` fallback) for a value the
    /// session can simply answer.
    #[must_use]
    pub fn active_mask_flag(&self) -> i32 {
        self.active_layer().map_or(0, |l| i32::from(l.mask_flag()))
    }

    /// Whether the active layer clips to the one below it.
    #[must_use]
    pub fn active_layer_clips(&self) -> bool {
        self.active_layer().is_some_and(|l| l.clips_to_below)
    }

    /// The active layer's kind (`raster`, `text`, …), empty when there is none.
    #[must_use]
    pub fn active_layer_kind(&self) -> String {
        self.active_layer()
            .map(|l| l.kind.as_str().to_owned())
            .unwrap_or_default()
    }

    /// Name of the active layer, empty when there is none.
    #[must_use]
    pub fn active_layer_name(&self) -> String {
        self.active_layer()
            .map(|l| l.name.clone())
            .unwrap_or_default()
    }

    /// What the Properties panel is describing (handbook 01).
    ///
    /// The document whenever there is no layer to describe — no document, or
    /// a graph with no active layer — and otherwise the active layer's kind.
    /// The panel decides its own *scope*: a user may ask for the document
    /// while a layer is active. This is only what the selection says.
    #[must_use]
    pub fn inspector_subject(&self) -> InspectorSubject {
        self.active_layer().map_or(InspectorSubject::Document, |l| {
            InspectorSubject::from_kind(l.kind)
        })
    }

    /// The layers panel's rows, top of the stack first.
    ///
    /// Empty when there is no document — the panel shows nothing, rather than
    /// a stack of one placeholder. A graph that exists always has at least one
    /// layer, so this is the only way the list is empty.
    #[must_use]
    pub fn layer_rows(&self) -> Vec<LayerRow> {
        self.graph
            .as_ref()
            .map(|g| layer_rows(g, &self.selected_layer_ids))
            .unwrap_or_default()
    }

    /// Active layer effects as `id:name:enabled|…` for Properties UI.
    pub fn active_effects_joined(&self) -> String {
        let Some(graph) = self.graph.as_ref() else {
            return String::new();
        };
        let Some(id) = graph.active_id() else {
            return String::new();
        };
        let Some(layer) = graph.get(id) else {
            return String::new();
        };
        layer
            .effects
            .iter()
            .map(|e| format!("{}:{}:{}", e.id, e.name, if e.enabled { "1" } else { "0" }))
            .collect::<Vec<_>>()
            .join("|")
    }

    /// Update object selection from a layers-panel click.
    ///
    /// - `ctrl`: toggle `index` in the selection
    /// - `shift`: select inclusive range from active to `index`
    /// - else: single-select `index` (and make it active)
    pub fn select_layer_click(&mut self, index: usize, ctrl: bool, shift: bool) {
        {
            let Some(graph) = self.graph.as_mut() else {
                return;
            };
            let Some(clicked_id) = graph.layers().get(index).map(|l| l.id) else {
                return;
            };
            if shift {
                let anchor = graph
                    .active_id()
                    .and_then(|id| graph.index_of(id))
                    .unwrap_or(index);
                let lo = anchor.min(index);
                let hi = anchor.max(index);
                let ids: Vec<LayerId> = graph.layers()[lo..=hi].iter().map(|l| l.id).collect();
                let _ = graph.set_active(clicked_id);
                self.selected_layer_ids = ids;
            } else if ctrl {
                if let Some(pos) = self
                    .selected_layer_ids
                    .iter()
                    .position(|id| *id == clicked_id)
                {
                    self.selected_layer_ids.remove(pos);
                    if self.selected_layer_ids.is_empty() {
                        let _ = graph.set_active(clicked_id);
                        self.selected_layer_ids = vec![clicked_id];
                    } else if graph.active_id() == Some(clicked_id) {
                        let keep = self.selected_layer_ids[self.selected_layer_ids.len() - 1];
                        let _ = graph.set_active(keep);
                    }
                } else {
                    self.selected_layer_ids.push(clicked_id);
                    let _ = graph.set_active(clicked_id);
                }
            } else {
                let _ = graph.set_active(clicked_id);
                self.selected_layer_ids = vec![clicked_id];
            }
        }
        let name = self.object_selection_names_joined();
        self.announce(format!("Object selection: {name}"));
    }

    pub fn paint_target(&self) -> PaintTarget {
        let active = self.graph.as_ref().and_then(|g| g.active_id());
        match self.mask_edit_layer {
            Some(id) if active == Some(id) => PaintTarget::LayerMask,
            _ => PaintTarget::LayerPixels,
        }
    }

    /// Toolkit-neutral edit-target id: `layer` or `mask`.
    pub fn edit_target_id(&self) -> &'static str {
        match self.paint_target() {
            PaintTarget::LayerMask => "mask",
            PaintTarget::LayerPixels => "layer",
        }
    }

    /// Short user-facing edit-target label.
    pub fn edit_target_label(&self) -> &'static str {
        match self.paint_target() {
            PaintTarget::LayerMask => "Layer mask",
            PaintTarget::LayerPixels => "Layer pixels",
        }
    }

    /// Align object selection with the active layer (single-select default).
    pub fn sync_object_selection_to_active(&mut self) {
        if let Some(id) = self.graph.as_ref().and_then(|g| g.active_id()) {
            self.selected_layer_ids = vec![id];
        } else {
            self.selected_layer_ids.clear();
        }
    }

    /// Replace object selection with explicit layer ids (must exist in the graph).
    pub fn set_object_selection(&mut self, ids: Vec<LayerId>) {
        let Some(graph) = self.graph.as_ref() else {
            self.selected_layer_ids.clear();
            return;
        };
        self.selected_layer_ids = ids
            .into_iter()
            .filter(|id| graph.get(*id).is_some())
            .collect();
    }

    pub fn object_selection_names_joined(&self) -> String {
        let Some(graph) = self.graph.as_ref() else {
            return String::new();
        };
        self.selected_layer_ids
            .iter()
            .filter_map(|id| graph.get(*id).map(|layer| layer.name.clone()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn announce(&mut self, message: impl Into<String>) {
        self.last_announce = message.into();
    }

    pub fn status_summary(&self) -> String {
        if !self.has_document {
            return "PhotoTux — create or open a document".to_owned();
        }
        let layers = self.layer_count();
        // Read the active layer rather than joining every layer's name and
        // kind into two strings in order to index one entry out of each.
        let active = self.active_layer();
        let layer_name = active
            .map(|l| l.name.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("—");
        let layer_kind = active
            .map(|l| l.kind.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("?");
        let edit = self.edit_target_label();
        let pixel = if self.selection.active {
            "pixel selection"
        } else {
            "no pixel selection"
        };
        let object = {
            let joined = self.object_selection_names_joined();
            if joined.is_empty() {
                "no object selection".to_owned()
            } else {
                format!("object: {joined}")
            }
        };
        // Keep composite ms out of this string — it updates every frame and must not
        // drive statusText / AT-SPI name churn (see Interactive Stability Checklist §14–15).
        format!(
            "{}×{} · zoom {:.0}% · {} ({}) · {} · {} · {} · {} layers · {}",
            self.size.width,
            self.size.height,
            self.camera.zoom * 100.0,
            layer_name,
            layer_kind,
            edit,
            object,
            pixel,
            layers,
            self.active_tool,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_the_document_session_sizes() {
        assert_eq!(SizePreset::P720.size(), DocumentSize::new(1280, 720));
        assert_eq!(SizePreset::P1080.size(), DocumentSize::new(1920, 1080));
        assert_eq!(SizePreset::P2k.size(), DocumentSize::new(2560, 1440));
        assert_eq!(SizePreset::P4k.size(), DocumentSize::new(3840, 2160));
    }

    #[test]
    fn zoom_clamped() {
        let mut s = SessionState::default();
        s.set_zoom(100.0);
        assert!((s.camera.zoom - Camera2D::MAX_ZOOM).abs() < f32::EPSILON);
        s.set_zoom(0.01);
        assert!((s.camera.zoom - Camera2D::MIN_ZOOM).abs() < f32::EPSILON);
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
    fn apply_preset_creates_graph() {
        let mut s = SessionState::default();
        s.set_viewport(1200.0, 800.0);
        assert!(!s.has_document);
        s.apply_preset(SizePreset::P4k);
        assert!(s.has_document);
        assert_eq!(s.size, DocumentSize::new(3840, 2160));
        assert_eq!(s.layer_count(), 2);
        assert!(s.graph.is_some());
    }

    #[test]
    fn apply_flattened_creates_single_layer() {
        let mut session = SessionState::default();
        session.apply_flattened(DocumentSize::new(640, 480), "photo.png");
        assert_eq!(session.layer_count(), 1);
        let rows = session.layer_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "photo.png");
    }

    #[test]
    fn preset_from_label() {
        assert_eq!(SizePreset::from_label("1080p"), Some(SizePreset::P1080));
        assert_eq!(SizePreset::from_label("2k"), Some(SizePreset::P2k));
        assert_eq!(SizePreset::from_label("nope"), None);
    }

    #[test]
    fn paint_target_follows_mask_edit_layer() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let active = s
            .graph
            .as_ref()
            .and_then(|g| g.active_id())
            .expect("active");
        assert_eq!(s.paint_target(), PaintTarget::LayerPixels);
        s.mask_edit_layer = Some(active);
        assert_eq!(s.paint_target(), PaintTarget::LayerMask);
        assert!(s.status_summary().contains("mask"));
        assert!(
            !s.status_summary().contains("composite"),
            "composite ms must stay out of status_summary to avoid AT-SPI thrash"
        );
    }

    /// Every source file cites the Decision Register, never an archived ADR.
    ///
    /// The ADR files were deleted in July 2026 and
    /// `Appendix/Archived-ADR-to-DR-Map.md` says in as many words that former
    /// ids are not a second authority. Twenty-six module headers went on
    /// citing them anyway, so `document.rs` opened by pointing a reader at two
    /// documents that do not exist. Nothing said so, because a citation is
    /// prose — it compiles either way, and the only cost is paid by whoever
    /// follows it.
    ///
    /// Reading the tree as text is what makes the rule checkable at all. The
    /// map itself is the one place an ADR id is still allowed to appear, and
    /// it lives under `internal_docs/`, which this walk does not enter.
    #[test]
    fn no_source_file_cites_an_archived_adr() {
        fn walk(dir: &std::path::Path, found: &mut Vec<String>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                // Build output is generated from these same sources, so it
                // carries copies of every citation and would report each twice.
                if name == "target" || name == "node_modules" || name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    walk(&path, found);
                } else if let Ok(text) = std::fs::read_to_string(&path) {
                    for (line, body) in text.lines().enumerate() {
                        if let Some(at) = body.find("ADR-")
                            && body[at + 4..].starts_with(|c: char| c.is_ascii_digit())
                        {
                            found.push(format!("{}:{}", path.display(), line + 1));
                        }
                    }
                }
            }
        }

        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let mut found = Vec::new();
        for area in ["crates", "qml", "scripts"] {
            walk(&root.join(area), &mut found);
        }
        assert!(
            found.is_empty(),
            "archived ADR ids cited in shipped source — cite the live DR instead \
             (see internal_docs/Appendix/Archived-ADR-to-DR-Map.md): {found:?}"
        );
    }

    #[test]
    fn snapshot_lease_and_save_receipt() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let lease = s.snapshot_lease().expect("lease");
        assert_eq!(lease.width, 1280);
        assert_eq!(lease.generation, 1);
        assert!(s.is_dirty_vs_persisted());
        assert!(s.mark_persisted(1));
        assert!(!s.is_dirty_vs_persisted());
        s.bump_document_generation();
        assert!(s.is_dirty_vs_persisted());
    }
}
