//! Pure document/session types — no Qt (ADR-006, ADR-011, ADR-017).

mod actions;
mod brush_preset;
mod camera;
mod cancel;
mod color;
mod color_mgmt;
mod command;
mod commands;
mod cpu_composite;
mod document;
mod error;
mod guides;
mod history;
mod layer;
mod layer_style;
mod paths;
mod selection;
mod shell;
mod stroke;
mod text_bake;
mod transform;
mod undo;

pub use actions::{
    ActionDescriptor, action_by_id, action_shortcuts_json, actions_for_context, actions_json,
    chord_map_from_action_shortcuts, context_actions_json, default_action_shortcuts,
    default_actions, default_shortcut_map, effective_action_shortcuts, effective_shortcuts_json,
    normalize_shortcut, resolve_shortcut, shortcut_conflict, shortcuts_json,
};
pub use brush_preset::{BrushPreset, BrushPresetLibrary};
pub use camera::{Camera2D, FpsTracker, Rect};
pub use cancel::CancelToken;
pub use color::{ColorState, SampleSource};
pub use color_mgmt::{ConvertPlan, DocumentColorState, convert_rgba8_profile};
pub use command::{EngineCommand, EngineEvent};
pub use commands::{
    CommandArgs, CommandEffects, CommandError, HostFollowUp, HostHistoryAction, command_id,
};
pub use cpu_composite::{CpuLayerRef, composite_rgba8};
pub use document::{DocumentGraph, GRAPH_SCHEMA_VERSION, MAX_LAYERS};
pub use error::DocumentError;
pub use guides::{Guide, GuideOrientation, ViewGuides};
pub use history::{HistoryEntry, HistoryKind, HistoryService};
pub use layer::{
    AdjustmentParams, BlendMode, FilterEffect, FilterParams, Layer, LayerId, LayerKind, LayerMask,
    LayerTransform, LockFlags, MAX_BLUR_RADIUS, PaintTarget, ShapeContent, TextContent,
};
pub use layer_style::{LayerStyle, apply_styles_rgba8};
pub use paths::{
    PathDocument, PathPoint, VectorPath, ellipse_path, rasterize_shape_rgba8, rect_path,
    stroke_path_rgba8,
};
pub use selection::{
    SelectionCombine, SelectionEllipse, SelectionRect, SelectionShape, SelectionState,
    contract_mask_r8, expand_mask_r8, feather_mask_r8,
};
pub use shell::{
    PanelDescriptor, ToolDescriptor, default_panels, default_tools, essentials_panel_visibility,
    panels_json, tools_json,
};
pub use stroke::{BrushParams, Dab, StrokeBuilder};
pub use text_bake::bake_text_rgba8;
pub use transform::{Affine2, CropRect, ResizeRequest, TransformPreview, TransformSession};
pub use undo::{GraphCommand, UndoStack, actions as undo_actions};

use serde::{Deserialize, Serialize};

/// Pixel dimensions of the open document canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Tool ids aligned with `assets/icons/ICON_MAP.md`.
pub mod tool_id {
    pub const BRUSH: &str = "tool.brush";
    pub const ERASER: &str = "tool.eraser";
    pub const PAN: &str = "tool.pan";
    pub const ZOOM: &str = "tool.zoom";
    pub const SELECT_RECT: &str = "tool.select.rect";
    pub const SELECT_ELLIPSE: &str = "tool.select.ellipse";
    pub const SELECT_LASSO: &str = "tool.select.lasso";
    pub const SELECT_POLYGON: &str = "tool.select.polygon";
    pub const MOVE: &str = "tool.move";
    pub const TRANSFORM: &str = "tool.transform";
    pub const CROP: &str = "tool.crop";
    pub const FILL: &str = "tool.fill";
    pub const GRADIENT: &str = "tool.gradient";
    pub const EYEDROPPER: &str = "tool.eyedropper";
    pub const TEXT: &str = "tool.text";
    pub const SHAPE: &str = "tool.shape";
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
    pub colors: ColorState,
    pub guides: ViewGuides,
    pub brush_presets: BrushPresetLibrary,
    pub document_path: Option<String>,
    /// Generation last successfully persisted (save receipt); `None` if never saved.
    pub last_persisted_generation: Option<u64>,
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
            colors: ColorState::default(),
            guides: ViewGuides::default(),
            brush_presets: BrushPresetLibrary::with_defaults(),
            document_path: None,
            last_persisted_generation: None,
        }
    }
}

impl SessionState {
    pub fn set_zoom(&mut self, zoom: f32) {
        self.camera.set_zoom(zoom);
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
        self.brush.eraser = self.active_tool == tool_id::ERASER;
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
    }

    /// Set the world-space point shown at the viewport center.
    pub fn set_pan(&mut self, world_x: f32, world_y: f32) {
        self.camera.pan_x = world_x;
        self.camera.pan_y = world_y;
    }

    pub fn zoom_at(&mut self, factor: f32, anchor_x: f32, anchor_y: f32) {
        self.camera
            .zoom_at(factor, anchor_x, anchor_y, self.viewport_w, self.viewport_h);
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
        let w = size.width.clamp(1, 32_768);
        let h = size.height.clamp(1, 32_768);
        self.size = DocumentSize::new(w, h);
        self.has_document = true;
        self.graph = Some(DocumentGraph::new(self.size));
        self.history.clear();
        self.selection.clear();
        self.transform_session = None;
        self.mask_edit_layer = None;
        self.document_path = None;
        self.last_persisted_generation = None;
        self.zoom_to_fit();
    }

    pub fn apply_flattened(&mut self, size: DocumentSize, layer_name: impl Into<String>) {
        let width = size.width.clamp(1, 32_768);
        let height = size.height.clamp(1, 32_768);
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
        self.last_persisted_generation = None;
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
            graph.generation
        } else {
            0
        }
    }

    /// Layer names bottom→top, joined for QML (pipe-separated).
    pub fn layer_names_joined(&self) -> String {
        self.graph
            .as_ref()
            .map(|g| {
                g.layers()
                    .iter()
                    .map(|l| l.name.as_str())
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .unwrap_or_default()
    }

    /// Visibility flags as "1|0|1".
    pub fn layer_visibility_joined(&self) -> String {
        self.graph
            .as_ref()
            .map(|g| {
                g.layers()
                    .iter()
                    .map(|l| if l.visible { "1" } else { "0" })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .unwrap_or_default()
    }

    pub fn layer_kinds_joined(&self) -> String {
        self.graph
            .as_ref()
            .map(|g| g.layer_kinds_joined())
            .unwrap_or_default()
    }

    pub fn layer_mask_flags_joined(&self) -> String {
        self.graph
            .as_ref()
            .map(|g| g.layer_mask_flags_joined())
            .unwrap_or_default()
    }

    pub fn layer_clips_joined(&self) -> String {
        self.graph
            .as_ref()
            .map(|g| g.layer_clips_joined())
            .unwrap_or_default()
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

    pub fn history_labels_joined(&self) -> String {
        self.history.labels_newest_first().join("|")
    }

    pub fn status_summary(&self) -> String {
        if !self.has_document {
            return "PhotoTux — create or open a document".to_owned();
        }
        let layers = self.layer_count();
        let idx = self.active_layer_index();
        let names = self.layer_names_joined();
        let kinds = self.layer_kinds_joined();
        let layer_name = names
            .split('|')
            .nth(idx as usize)
            .filter(|s| !s.is_empty())
            .unwrap_or("—");
        let layer_kind = kinds
            .split('|')
            .nth(idx as usize)
            .filter(|s| !s.is_empty())
            .unwrap_or("?");
        let edit = self.edit_target_label();
        let sel = if self.selection.active {
            "pixel selection"
        } else {
            "no pixel selection"
        };
        let comp = if self.composite_ms > 0.0 {
            format!(" · composite {:.2} ms", self.composite_ms)
        } else {
            String::new()
        };
        format!(
            "{}×{} · zoom {:.0}% · {} ({}) · {} · {} · {} layers · {}{}",
            self.size.width,
            self.size.height,
            self.camera.zoom * 100.0,
            layer_name,
            layer_kind,
            edit,
            sel,
            layers,
            self.active_tool,
            comp
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
        assert_eq!(session.layer_names_joined(), "photo.png");
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
