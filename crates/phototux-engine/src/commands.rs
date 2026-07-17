//! Named document/session command spine (handbook 08, DR-003).
//!
//! Paint-worker dab traffic stays in [`crate::command::EngineCommand`].
//! User-visible semantic mutations enter here via [`SessionState::invoke`].
//!
//! GPU pixel ops run in the host; many commands are **GPU-then-commit**
//! (host applies canvas work, then invoke updates graph/selection/history).

use thiserror::Error;

use crate::document::MAX_LAYERS;
use crate::error::DocumentError;
use crate::history::HistoryKind;
use crate::layer::{
    AdjustmentParams, BlendMode, FillContent, LayerId, LayerKind, LayerMask, LayerTransform,
    PaintTarget, ShapeContent, TextContent,
};
use crate::layer_style::LayerStyle;
use crate::selection::{SelectionCombine, SelectionRect, SelectionShape};
use crate::undo::actions as undo_actions;
use crate::{SessionState, tool_id};

/// Stable command identifiers (vendor-neutral; see Command Taxonomy).
pub mod command_id {
    pub const HISTORY_UNDO: &str = "history.undo";
    pub const HISTORY_REDO: &str = "history.redo";

    pub const LAYER_CREATE: &str = "layer.create";
    pub const LAYER_CREATE_FILL: &str = "layer.create-fill";
    pub const LAYER_SET_FILL_COLOR: &str = "layer.set-fill-color";
    pub const LAYER_DELETE: &str = "layer.delete";
    pub const LAYER_SET_ACTIVE: &str = "layer.set-active";
    pub const LAYER_SET_VISIBILITY: &str = "layer.set-visibility";
    pub const LAYER_SET_OPACITY: &str = "layer.set-opacity";
    pub const LAYER_SET_BLEND: &str = "layer.set-blend";
    pub const LAYER_REORDER: &str = "layer.reorder";
    pub const LAYER_GROUP: &str = "layer.group";
    pub const LAYER_UNGROUP: &str = "layer.ungroup";
    pub const LAYER_SET_CLIP: &str = "layer.set-clip";
    pub const LAYER_SET_LOCKS: &str = "layer.set-locks";

    pub const VIEW_ZOOM_TO: &str = "view.zoom-to";
    pub const VIEW_ZOOM_TO_FIT: &str = "view.zoom-to-fit";
    pub const VIEW_PAN_TO: &str = "view.pan-to";
    pub const VIEW_PAN_BY: &str = "view.pan-by";
    pub const VIEW_ZOOM_AT: &str = "view.zoom-at";
    pub const VIEW_SET_TOOL: &str = "view.set-tool";

    pub const DOCUMENT_NEW_PRESET: &str = "document.new-preset";
    pub const DOCUMENT_NEW_SIZE: &str = "document.new-size";
    pub const DOCUMENT_ASSIGN_PROFILE: &str = "document.assign-profile";
    pub const DOCUMENT_CONVERT_PROFILE: &str = "document.convert-profile";
    pub const DOCUMENT_SET_SOFT_PROOF: &str = "document.set-soft-proof";
    /// Embed or clear validated ICC profile bytes on the document.
    pub const DOCUMENT_SET_ICC: &str = "document.set-icc";
    pub const DOCUMENT_CROP: &str = "document.crop";
    pub const DOCUMENT_ROTATE_90: &str = "document.rotate-90";
    pub const HISTORY_JUMP: &str = "history.jump";

    pub const SELECTION_REPLACE: &str = "selection.replace";
    pub const SELECTION_DESELECT: &str = "selection.deselect";
    pub const SELECTION_INVERT: &str = "selection.invert";
    pub const SELECTION_SELECT_ALL: &str = "selection.select-all";
    pub const SELECTION_MODIFY: &str = "selection.modify";
    /// Copy pixel selection R8 into the active layer mask (host GPU).
    pub const SELECTION_TO_MASK: &str = "selection.to-mask";
    /// Load active layer mask R8 into the pixel selection channel (host GPU).
    pub const MASK_TO_SELECTION: &str = "mask.to-selection";

    pub const MASK_CREATE: &str = "mask.create";
    pub const MASK_DELETE: &str = "mask.delete";
    pub const MASK_SET_ENABLED: &str = "mask.set-enabled";
    pub const MASK_SET_ATTRIBUTES: &str = "mask.set-attributes";
    pub const MASK_CREATE_VECTOR: &str = "mask.create-vector";
    pub const MASK_APPLY: &str = "mask.apply";

    pub const TEXT_CREATE: &str = "text.create";
    pub const TEXT_SET_CONTENT: &str = "text.set-content";
    pub const TEXT_BAKE: &str = "text.bake";

    pub const SHAPE_CREATE: &str = "shape.create";
    pub const SHAPE_RASTERIZE: &str = "shape.rasterize";
    pub const SHAPE_BOOLEAN: &str = "shape.boolean";

    pub const FILTER_ADD_ADJUSTMENT: &str = "filter.add-adjustment";
    pub const FILTER_SET_PARAMETERS: &str = "filter.set-parameters";
    pub const FILTER_ADD_EFFECT: &str = "filter.add-effect";
    pub const FILTER_SET_GAUSSIAN_RADIUS: &str = "filter.set-gaussian-radius";
    /// Start / refresh ephemeral filter gallery preview (no document dirty).
    pub const FILTER_PREVIEW: &str = "filter.preview";
    /// Update preview parameters while gallery is open.
    pub const FILTER_SET_PREVIEW_PARAMS: &str = "filter.set-preview-params";
    /// Commit preview into layer effects + filter plan (undoable).
    pub const FILTER_COMMIT: &str = "filter.commit";
    /// Abort preview session without mutating authority.
    pub const FILTER_CANCEL_PREVIEW: &str = "filter.cancel-preview";
    pub const EFFECT_REORDER: &str = "effect.reorder";
    pub const EFFECT_SET_ENABLED: &str = "effect.set-enabled";

    pub const PATH_SET_CLOSED: &str = "path.set-closed";
    pub const PATH_MOVE_ANCHOR: &str = "path.move-anchor";
    pub const PATH_ADD_ANCHOR: &str = "path.add-anchor";
    pub const PATH_DELETE_ANCHOR: &str = "path.delete-anchor";

    pub const STYLE_ADD_DROP_SHADOW: &str = "style.add-drop-shadow";
    pub const STYLE_ADD_STROKE: &str = "style.add-stroke";
    pub const STYLE_ADD_OUTER_GLOW: &str = "style.add-outer-glow";
    pub const STYLE_ADD_COLOR_OVERLAY: &str = "style.add-color-overlay";

    pub const CLIPBOARD_PASTE_LAYER: &str = "clipboard.paste-layer";
    pub const PATH_STROKE_TO_LAYER: &str = "path.stroke-to-layer";

    pub const RASTER_TRANSFORM_COMMIT: &str = "raster.transform-commit";
    pub const RASTER_FLIP: &str = "raster.flip";
    pub const RASTER_FILL: &str = "raster.fill";
    pub const RASTER_GRADIENT: &str = "raster.gradient";
    pub const RASTER_PAINT_STROKE: &str = "raster.paint-stroke";

    /// Application chrome — host opens preferences dialog.
    pub const APP_SHOW_PREFERENCES: &str = "app.show-preferences";
    /// Application chrome — host opens filter gallery dialog.
    pub const APP_SHOW_FILTER_GALLERY: &str = "app.show-filter-gallery";
    /// Workspace chrome — reset panel visibility to Essentials.
    pub const WORKSPACE_RESET: &str = "workspace.reset";
    /// Workspace chrome — toggle a panel by id (`panel.layers`, …).
    pub const WORKSPACE_TOGGLE_PANEL: &str = "workspace.toggle-panel";
    /// Workspace chrome — apply a built-in layout preset by id.
    pub const WORKSPACE_APPLY_PRESET: &str = "workspace.apply-preset";

    /// Built-in commands registered for discovery / headless tests.
    pub const ALL: &[&str] = &[
        HISTORY_UNDO,
        HISTORY_REDO,
        HISTORY_JUMP,
        LAYER_CREATE,
        LAYER_CREATE_FILL,
        LAYER_SET_FILL_COLOR,
        LAYER_DELETE,
        LAYER_SET_ACTIVE,
        LAYER_SET_VISIBILITY,
        LAYER_SET_OPACITY,
        LAYER_SET_BLEND,
        LAYER_REORDER,
        LAYER_GROUP,
        LAYER_UNGROUP,
        LAYER_SET_CLIP,
        LAYER_SET_LOCKS,
        VIEW_ZOOM_TO,
        VIEW_ZOOM_TO_FIT,
        VIEW_PAN_TO,
        VIEW_PAN_BY,
        VIEW_ZOOM_AT,
        VIEW_SET_TOOL,
        DOCUMENT_NEW_PRESET,
        DOCUMENT_NEW_SIZE,
        DOCUMENT_ASSIGN_PROFILE,
        DOCUMENT_CONVERT_PROFILE,
        DOCUMENT_SET_SOFT_PROOF,
        DOCUMENT_SET_ICC,
        DOCUMENT_CROP,
        DOCUMENT_ROTATE_90,
        SELECTION_REPLACE,
        SELECTION_DESELECT,
        SELECTION_INVERT,
        SELECTION_SELECT_ALL,
        SELECTION_MODIFY,
        SELECTION_TO_MASK,
        MASK_TO_SELECTION,
        MASK_CREATE,
        MASK_DELETE,
        MASK_SET_ENABLED,
        MASK_SET_ATTRIBUTES,
        MASK_CREATE_VECTOR,
        MASK_APPLY,
        TEXT_CREATE,
        TEXT_SET_CONTENT,
        TEXT_BAKE,
        SHAPE_CREATE,
        SHAPE_RASTERIZE,
        SHAPE_BOOLEAN,
        FILTER_ADD_ADJUSTMENT,
        FILTER_SET_PARAMETERS,
        FILTER_ADD_EFFECT,
        FILTER_SET_GAUSSIAN_RADIUS,
        FILTER_PREVIEW,
        FILTER_SET_PREVIEW_PARAMS,
        FILTER_COMMIT,
        FILTER_CANCEL_PREVIEW,
        EFFECT_REORDER,
        EFFECT_SET_ENABLED,
        PATH_SET_CLOSED,
        PATH_MOVE_ANCHOR,
        PATH_ADD_ANCHOR,
        PATH_DELETE_ANCHOR,
        STYLE_ADD_DROP_SHADOW,
        STYLE_ADD_STROKE,
        STYLE_ADD_OUTER_GLOW,
        STYLE_ADD_COLOR_OVERLAY,
        CLIPBOARD_PASTE_LAYER,
        PATH_STROKE_TO_LAYER,
        RASTER_TRANSFORM_COMMIT,
        RASTER_FLIP,
        RASTER_FILL,
        RASTER_GRADIENT,
        RASTER_PAINT_STROKE,
        APP_SHOW_PREFERENCES,
        APP_SHOW_FILTER_GALLERY,
        WORKSPACE_RESET,
        WORKSPACE_TOGGLE_PANEL,
        WORKSPACE_APPLY_PRESET,
    ];
}

/// Host follow-up after a successful command (canvas / pixel / chrome work).
#[derive(Debug, Clone, PartialEq)]
pub enum HostFollowUp {
    None,
    /// Rewrite all layer pixels from `from` profile to `to`, then mark converted.
    ConvertPixels {
        from: String,
        to: String,
    },
    /// Open the preferences dialog (application chrome).
    ShowPreferences,
    /// Reset workspace panel visibility to Essentials.
    ResetWorkspace,
    /// Toggle visibility of a dock panel by descriptor id.
    TogglePanel {
        panel_id: String,
    },
    /// Apply a built-in workspace layout preset.
    ApplyWorkspacePreset {
        preset_id: String,
    },
    /// Copy pixel selection into the active layer mask (GPU host).
    SelectionToMask,
    /// Copy active layer mask into the pixel selection channel (GPU host).
    MaskToSelection,
    /// Bake active layer mask into layer pixels, then clear mask (GPU host).
    ApplyMask,
    /// Undo `steps` times to jump the history timeline (host applies stroke/selection stacks).
    HistoryJump {
        steps: u32,
    },
    /// Rasterize two shape layers, boolean-combine, write into `result` raster layer.
    ShapeBoolean {
        op: crate::BooleanOp,
        a: crate::LayerId,
        b: crate::LayerId,
        result: crate::LayerId,
    },
    /// Re-rasterize a shape layer after path edit (host GPU upload).
    RasterizeShape {
        id: crate::LayerId,
    },
    /// Open the filter gallery dialog (application chrome).
    ShowFilterGallery,
}

/// Parameters for [`SessionState::invoke`].
#[derive(Debug, Clone)]
pub enum CommandArgs {
    None,
    LayerIndex(i32),
    SetVisibility {
        index: i32,
        visible: bool,
    },
    SetOpacity {
        opacity: f32,
    },
    SetBlend {
        blend: String,
    },
    Reorder {
        to_index: i32,
    },
    Zoom {
        zoom: f32,
    },
    Pan {
        world_x: f32,
        world_y: f32,
    },
    PanBy {
        dx: f32,
        dy: f32,
    },
    ZoomAt {
        factor: f32,
        anchor_x: f32,
        anchor_y: f32,
    },
    Tool {
        tool: String,
    },
    NewPreset {
        label: String,
    },
    NewSize {
        width: u32,
        height: u32,
    },
    AssignProfile {
        profile: String,
    },
    ConvertProfile {
        profile: String,
    },
    SelectionReplace {
        shape: SelectionShape,
        combine: SelectionCombine,
        rect: SelectionRect,
        polygon: Vec<(f32, f32)>,
        label: String,
    },
    SelectionModify {
        op: String,
        radius: u32,
    },
    MaskSetEnabled {
        enabled: bool,
    },
    LayerSetClip {
        clips: bool,
    },
    TextCreate {
        text: String,
    },
    TextSetContent {
        content: TextContent,
    },
    ShapeCreate {
        content: ShapeContent,
    },
    ShapeBoolean {
        op: String,
    },
    FillCreate {
        color_rgba: [f32; 4],
    },
    FillColor {
        color_rgba: [f32; 4],
    },
    FilterAdjustment {
        kind: String,
    },
    FilterParameters {
        p0: f32,
        p1: f32,
        p2: f32,
    },
    FilterEffect {
        kind: String,
    },
    FilterGaussianRadius {
        radius: f32,
    },
    FilterPreview {
        kind: String,
    },
    FilterPreviewParams {
        p0: f32,
        p1: f32,
        p2: f32,
    },
    EffectReorder {
        effect_id: u64,
        to_index: i32,
    },
    EffectSetEnabled {
        effect_id: u64,
        enabled: bool,
    },
    PathSetClosed {
        closed: bool,
    },
    PathMoveAnchor {
        index: usize,
        x: f32,
        y: f32,
    },
    PathAddAnchor {
        x: f32,
        y: f32,
        index: Option<usize>,
    },
    PathDeleteAnchor {
        index: usize,
    },
    PasteLayer {
        name: String,
    },
    PathStroke {
        layer_name: String,
    },
    RasterFlip {
        horizontal: bool,
    },
    RasterPaintStroke {
        label: String,
    },
    DocumentCrop {
        width: u32,
        height: u32,
    },
    TogglePanel {
        panel_id: String,
    },
    ApplyWorkspacePreset {
        preset_id: String,
    },
    SetLocks {
        pixels: bool,
        position: bool,
        all: bool,
        alpha: bool,
    },
    MaskAttributes {
        enabled: bool,
        linked: bool,
        density: f32,
        feather: f32,
        inverted: bool,
    },
    SoftProof {
        profile: String,
        intent: String,
    },
    /// `None` clears embedded ICC; `Some` validates and embeds.
    SetIcc {
        bytes: Option<Vec<u8>>,
    },
    HistoryJump {
        entry_id: u64,
    },
}

/// Host-side follow-up for undo/redo entries that own GPU or selection stacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostHistoryAction {
    Undo(HistoryKind),
    Redo(HistoryKind),
}

/// Effects the UI host should apply after a successful command.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandEffects {
    pub recomposite: bool,
    pub dirty: bool,
    pub sync_layers: bool,
    pub sync_camera: bool,
    pub sync_doc: bool,
    pub sync_selection: bool,
    pub host_history: Option<HostHistoryAction>,
    pub host_follow_up: HostFollowUp,
    pub created_layer: Option<LayerId>,
    /// Document generation after the command (0 if no document).
    pub generation: u64,
}

impl CommandEffects {
    fn view_only() -> Self {
        Self {
            recomposite: false,
            dirty: false,
            sync_layers: false,
            sync_camera: true,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: 0,
        }
    }

    fn host_chrome(follow_up: HostFollowUp) -> Self {
        Self {
            recomposite: false,
            dirty: false,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: follow_up,
            created_layer: None,
            generation: 0,
        }
    }

    fn document_edit(generation: u64) -> Self {
        Self {
            recomposite: true,
            dirty: true,
            sync_layers: true,
            sync_camera: false,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        }
    }

    fn selection_edit(generation: u64) -> Self {
        Self {
            recomposite: false,
            dirty: true,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: true,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        }
    }
}

/// Typed command failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CommandError {
    #[error("unknown command `{0}`")]
    Unknown(String),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("command rejected: {0}")]
    Rejected(&'static str),
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
}

impl SessionState {
    /// Whether `id` is a registered built-in command.
    pub fn command_known(id: &str) -> bool {
        command_id::ALL.contains(&id)
    }

    /// Invoke a named command. Graph/history mutations for document scope run here;
    /// GPU stroke/selection/transform undo follow-ups are returned in [`CommandEffects::host_history`].
    ///
    /// # Errors
    /// Returns [`CommandError`] when the command is unknown, preconditions fail, or graph mutation fails.
    pub fn invoke(&mut self, id: &str, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        match id {
            command_id::HISTORY_UNDO => self.cmd_history_undo(),
            command_id::HISTORY_REDO => self.cmd_history_redo(),
            command_id::HISTORY_JUMP => self.cmd_history_jump(args),
            command_id::LAYER_CREATE => self.cmd_layer_create(),
            command_id::LAYER_CREATE_FILL => self.cmd_layer_create_fill(args),
            command_id::LAYER_SET_FILL_COLOR => self.cmd_layer_set_fill_color(args),
            command_id::LAYER_DELETE => self.cmd_layer_delete(),
            command_id::LAYER_SET_ACTIVE => self.cmd_layer_set_active(args),
            command_id::LAYER_SET_VISIBILITY => self.cmd_layer_set_visibility(args),
            command_id::LAYER_SET_OPACITY => self.cmd_layer_set_opacity(args),
            command_id::LAYER_SET_BLEND => self.cmd_layer_set_blend(args),
            command_id::LAYER_REORDER => self.cmd_layer_reorder(args),
            command_id::LAYER_GROUP => self.cmd_layer_group(),
            command_id::LAYER_UNGROUP => self.cmd_layer_ungroup(),
            command_id::LAYER_SET_CLIP => self.cmd_layer_set_clip(args),
            command_id::LAYER_SET_LOCKS => self.cmd_layer_set_locks(args),
            command_id::VIEW_ZOOM_TO => self.cmd_view_zoom(args),
            command_id::VIEW_ZOOM_TO_FIT => {
                self.zoom_to_fit();
                let mut e = CommandEffects::view_only();
                e.generation = self.document_generation();
                Ok(e)
            }
            command_id::VIEW_PAN_TO => self.cmd_view_pan(args),
            command_id::VIEW_PAN_BY => self.cmd_view_pan_by(args),
            command_id::VIEW_ZOOM_AT => self.cmd_view_zoom_at(args),
            command_id::VIEW_SET_TOOL => self.cmd_view_set_tool(args),
            command_id::DOCUMENT_NEW_PRESET => self.cmd_document_new_preset(args),
            command_id::DOCUMENT_NEW_SIZE => self.cmd_document_new_size(args),
            command_id::DOCUMENT_ASSIGN_PROFILE => self.cmd_document_assign_profile(args),
            command_id::DOCUMENT_CONVERT_PROFILE => self.cmd_document_convert_profile(args),
            command_id::DOCUMENT_SET_SOFT_PROOF => self.cmd_document_set_soft_proof(args),
            command_id::DOCUMENT_SET_ICC => self.cmd_document_set_icc(args),
            command_id::DOCUMENT_CROP => self.cmd_document_crop(args),
            command_id::DOCUMENT_ROTATE_90 => self.cmd_document_rotate_90(),
            command_id::SELECTION_REPLACE => self.cmd_selection_replace(args),
            command_id::SELECTION_DESELECT => self.cmd_selection_deselect(),
            command_id::SELECTION_INVERT => self.cmd_selection_invert(),
            command_id::SELECTION_SELECT_ALL => self.cmd_selection_select_all(),
            command_id::SELECTION_MODIFY => self.cmd_selection_modify(args),
            command_id::SELECTION_TO_MASK => self.cmd_selection_to_mask(),
            command_id::MASK_TO_SELECTION => self.cmd_mask_to_selection(),
            command_id::MASK_CREATE => self.cmd_mask_create(),
            command_id::MASK_DELETE => self.cmd_mask_delete(),
            command_id::MASK_SET_ENABLED => self.cmd_mask_set_enabled(args),
            command_id::MASK_SET_ATTRIBUTES => self.cmd_mask_set_attributes(args),
            command_id::MASK_CREATE_VECTOR => self.cmd_mask_create_vector(),
            command_id::MASK_APPLY => self.cmd_mask_apply(),
            command_id::TEXT_CREATE => self.cmd_text_create(args),
            command_id::TEXT_SET_CONTENT => self.cmd_text_set_content(args),
            command_id::TEXT_BAKE => self.cmd_text_bake(),
            command_id::SHAPE_CREATE => self.cmd_shape_create(args),
            command_id::SHAPE_RASTERIZE => self.cmd_shape_rasterize(),
            command_id::SHAPE_BOOLEAN => self.cmd_shape_boolean(args),
            command_id::FILTER_ADD_ADJUSTMENT => self.cmd_filter_add_adjustment(args),
            command_id::FILTER_SET_PARAMETERS => self.cmd_filter_set_parameters(args),
            command_id::FILTER_ADD_EFFECT => self.cmd_filter_add_effect(args),
            command_id::FILTER_SET_GAUSSIAN_RADIUS => self.cmd_filter_set_gaussian_radius(args),
            command_id::FILTER_PREVIEW => self.cmd_filter_preview(args),
            command_id::FILTER_SET_PREVIEW_PARAMS => self.cmd_filter_set_preview_params(args),
            command_id::FILTER_COMMIT => self.cmd_filter_commit(),
            command_id::FILTER_CANCEL_PREVIEW => self.cmd_filter_cancel_preview(),
            command_id::EFFECT_REORDER => self.cmd_effect_reorder(args),
            command_id::EFFECT_SET_ENABLED => self.cmd_effect_set_enabled(args),
            command_id::PATH_SET_CLOSED => self.cmd_path_set_closed(args),
            command_id::PATH_MOVE_ANCHOR => self.cmd_path_move_anchor(args),
            command_id::PATH_ADD_ANCHOR => self.cmd_path_add_anchor(args),
            command_id::PATH_DELETE_ANCHOR => self.cmd_path_delete_anchor(args),
            command_id::STYLE_ADD_DROP_SHADOW => self.cmd_style_add_drop_shadow(),
            command_id::STYLE_ADD_STROKE => self.cmd_style_add_stroke(),
            command_id::STYLE_ADD_OUTER_GLOW => self.cmd_style_add_outer_glow(),
            command_id::STYLE_ADD_COLOR_OVERLAY => self.cmd_style_add_color_overlay(),
            command_id::CLIPBOARD_PASTE_LAYER => self.cmd_clipboard_paste_layer(args),
            command_id::PATH_STROKE_TO_LAYER => self.cmd_path_stroke_to_layer(args),
            command_id::RASTER_TRANSFORM_COMMIT => self.cmd_raster_transform_commit(),
            command_id::RASTER_FLIP => self.cmd_raster_flip(args),
            command_id::RASTER_FILL => self.cmd_raster_history("Fill"),
            command_id::RASTER_GRADIENT => self.cmd_raster_history("Gradient"),
            command_id::RASTER_PAINT_STROKE => self.cmd_raster_paint_stroke(args),
            command_id::APP_SHOW_PREFERENCES => {
                Ok(CommandEffects::host_chrome(HostFollowUp::ShowPreferences))
            }
            command_id::APP_SHOW_FILTER_GALLERY => {
                Ok(CommandEffects::host_chrome(HostFollowUp::ShowFilterGallery))
            }
            command_id::WORKSPACE_RESET => {
                Ok(CommandEffects::host_chrome(HostFollowUp::ResetWorkspace))
            }
            command_id::WORKSPACE_TOGGLE_PANEL => self.cmd_workspace_toggle_panel(args),
            command_id::WORKSPACE_APPLY_PRESET => self.cmd_workspace_apply_preset(args),
            other => Err(CommandError::Unknown(other.to_owned())),
        }
    }

    fn cmd_workspace_toggle_panel(
        &self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::TogglePanel { panel_id } = args else {
            return Err(CommandError::InvalidArgument(
                "workspace.toggle-panel requires TogglePanel args",
            ));
        };
        if !panel_id.starts_with("panel.") {
            return Err(CommandError::InvalidArgument(
                "panel_id must be a panel.* descriptor id",
            ));
        }
        Ok(CommandEffects::host_chrome(HostFollowUp::TogglePanel {
            panel_id,
        }))
    }

    fn cmd_workspace_apply_preset(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::ApplyWorkspacePreset { preset_id } = args else {
            return Err(CommandError::InvalidArgument(
                "workspace.apply-preset requires ApplyWorkspacePreset args",
            ));
        };
        if crate::workspace_preset_by_id(&preset_id).is_none() {
            return Err(CommandError::InvalidArgument("unknown workspace preset"));
        }
        Ok(CommandEffects::host_chrome(
            HostFollowUp::ApplyWorkspacePreset { preset_id },
        ))
    }

    pub fn document_generation(&self) -> u64 {
        self.graph.as_ref().map(|g| g.generation).unwrap_or(0)
    }

    fn bump_generation(&mut self) {
        if let Some(graph) = self.graph.as_mut() {
            graph.bump_generation();
        }
    }

    fn active_layer_id(&self) -> Result<LayerId, CommandError> {
        self.graph
            .as_ref()
            .and_then(|g| g.active_id())
            .ok_or(CommandError::Rejected("no active layer"))
    }

    fn assert_active_paintable(&self) -> Result<LayerId, CommandError> {
        let id = self.active_layer_id()?;
        let layer = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .ok_or(CommandError::Rejected("no active layer"))?;
        if layer.paint_blocked() {
            return Err(CommandError::Rejected("layer pixels locked"));
        }
        Ok(id)
    }

    fn assert_active_movable(&self) -> Result<LayerId, CommandError> {
        let id = self.active_layer_id()?;
        let layer = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .ok_or(CommandError::Rejected("no active layer"))?;
        if layer.position_blocked() {
            return Err(CommandError::Rejected("layer position locked"));
        }
        Ok(id)
    }

    fn cmd_history_undo(&mut self) -> Result<CommandEffects, CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(kind) = self.history.undo_next(graph) else {
            return Err(CommandError::Rejected("nothing to undo"));
        };
        let generation = graph.generation;
        let mut effects = CommandEffects::document_edit(generation);
        match kind {
            HistoryKind::Graph => {
                self.bump_generation();
                effects.generation = self.document_generation();
            }
            other => {
                effects.host_history = Some(HostHistoryAction::Undo(other));
                effects.recomposite = false;
            }
        }
        Ok(effects)
    }

    fn cmd_history_jump(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::HistoryJump { entry_id } = args else {
            return Err(CommandError::InvalidArgument("expected HistoryJump"));
        };
        let steps = self
            .history
            .undo_steps_to_entry(entry_id)
            .ok_or(CommandError::Rejected("history entry not found"))?;
        if steps == 0 {
            return Err(CommandError::Rejected("already at history entry"));
        }
        self.announce(format!("Jump history (−{steps})"));
        Ok(CommandEffects::host_chrome(HostFollowUp::HistoryJump {
            steps: u32::try_from(steps).unwrap_or(u32::MAX),
        }))
    }

    fn cmd_history_redo(&mut self) -> Result<CommandEffects, CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(kind) = self.history.redo_next(graph) else {
            return Err(CommandError::Rejected("nothing to redo"));
        };
        let generation = graph.generation;
        let mut effects = CommandEffects::document_edit(generation);
        match kind {
            HistoryKind::Graph => {
                self.bump_generation();
                effects.generation = self.document_generation();
            }
            other => {
                effects.host_history = Some(HostHistoryAction::Redo(other));
                effects.recomposite = false;
            }
        }
        Ok(effects)
    }

    fn cmd_layer_create(&mut self) -> Result<CommandEffects, CommandError> {
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if graph.layer_count() >= MAX_LAYERS {
            return Err(CommandError::Document(DocumentError::layer_limit(
                MAX_LAYERS,
            )));
        }
        undo_actions::add_layer(graph, history, None)?;
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_layer_create_fill(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let color = match args {
            CommandArgs::FillCreate { color_rgba } => color_rgba,
            CommandArgs::None => FillContent::default().color_rgba,
            _ => return Err(CommandError::InvalidArgument("expected fill color")),
        };
        let content = FillContent {
            color_rgba: [
                color[0].clamp(0.0, 1.0),
                color[1].clamp(0.0, 1.0),
                color[2].clamp(0.0, 1.0),
                color[3].clamp(0.0, 1.0),
            ],
        };
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = graph.add_fill_top(None, content)?;
        let index = graph.index_of(id).unwrap_or(0);
        let layer = graph
            .get(id)
            .cloned()
            .ok_or(CommandError::Document(DocumentError::LayerMissingAfterAdd))?;
        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_graph_applied(
            crate::GraphCommand::AddLayer { id, index, layer },
            "Add fill layer",
            generation,
        );
        self.selected_layer_ids = vec![id];
        self.announce("Added fill layer");
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_layer_set_fill_color(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::FillColor { color_rgba } = args else {
            return Err(CommandError::InvalidArgument("expected fill color"));
        };
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Fill {
            return Err(CommandError::Rejected("active layer is not a fill"));
        }
        let next = FillContent {
            color_rgba: [
                color_rgba[0].clamp(0.0, 1.0),
                color_rgba[1].clamp(0.0, 1.0),
                color_rgba[2].clamp(0.0, 1.0),
                color_rgba[3].clamp(0.0, 1.0),
            ],
        };
        let Some(prev) = graph.set_fill(id, Some(next.clone())) else {
            return Err(CommandError::Rejected("set fill failed"));
        };
        if prev.as_ref() == Some(&next) {
            return Err(CommandError::Rejected("fill unchanged"));
        }
        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_graph_applied(
            crate::GraphCommand::SetFill {
                id,
                prev,
                next: Some(next),
            },
            "Set fill color",
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    /// Targets for multi-select structural ops (selection when non-empty, else active).
    fn structural_target_ids(&self) -> Result<Vec<LayerId>, CommandError> {
        let graph = self
            .graph
            .as_ref()
            .ok_or(CommandError::Document(DocumentError::NoDocument))?;
        let mut ids = if self.selected_layer_ids.is_empty() {
            match graph.active_id() {
                Some(id) => vec![id],
                None => return Err(CommandError::Rejected("no active layer")),
            }
        } else {
            self.selected_layer_ids.clone()
        };
        ids.retain(|id| graph.get(*id).is_some());
        if ids.is_empty() {
            return Err(CommandError::Rejected("no target layers"));
        }
        // Stable stack order (bottom → top).
        ids.sort_by_key(|id| graph.index_of(*id).unwrap_or(usize::MAX));
        ids.dedup();
        Ok(ids)
    }

    fn cmd_layer_delete(&mut self) -> Result<CommandEffects, CommandError> {
        let targets = self.structural_target_ids()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if targets.len() >= graph.layer_count() {
            return Err(CommandError::Rejected("cannot delete all layers"));
        }
        for id in &targets {
            let Some(layer) = graph.get(*id) else {
                return Err(CommandError::Rejected("layer missing"));
            };
            if layer.locks.all {
                return Err(CommandError::Rejected("layer is locked"));
            }
        }

        // Break clips whose base is being deleted (nearest non-clipping layer below).
        let mut batch = Vec::new();
        let order = graph.stack_order();
        for (idx, id) in order.iter().enumerate() {
            let Some(layer) = graph.get(*id) else {
                continue;
            };
            if !layer.clips_to_below || targets.contains(id) {
                continue;
            }
            let old_base = order[..idx]
                .iter()
                .rev()
                .find(|below| graph.get(**below).is_some_and(|l| !l.clips_to_below));
            if old_base.is_some_and(|b| targets.contains(b)) {
                if let Some(true) = graph.set_clips_to_below(*id, false) {
                    batch.push(crate::GraphCommand::SetClipsToBelow {
                        id: *id,
                        prev: true,
                        next: false,
                    });
                }
            }
        }

        let prev_active = graph.active_id();
        let broke = batch.len();
        // Delete top→bottom so recorded indices match removal order.
        let mut deletes = Vec::with_capacity(targets.len());
        for id in targets.iter().rev() {
            let Some((index, layer)) = graph.remove_layer(*id) else {
                for cmd in deletes.iter().rev() {
                    if let crate::GraphCommand::DeleteLayer { index, layer, .. } = cmd {
                        graph.insert_layer_at(*index, layer.clone());
                    }
                }
                for cmd in batch.iter().rev() {
                    if let crate::GraphCommand::SetClipsToBelow { id, prev, .. } = cmd {
                        let _ = graph.set_clips_to_below(*id, *prev);
                    }
                }
                return Err(CommandError::Rejected("delete layer failed"));
            };
            deletes.push(crate::GraphCommand::DeleteLayer {
                id: *id,
                index,
                layer,
                prev_active,
            });
        }
        deletes.reverse();
        batch.extend(deletes);
        graph.bump_generation();
        let generation = graph.generation;
        let label = if targets.len() == 1 {
            "Delete layer".to_owned()
        } else {
            format!("Delete {} layers", targets.len())
        };
        self.history
            .push_graph_applied(crate::GraphCommand::Batch(batch), label, generation);
        self.sync_object_selection_to_active();
        let name = self.object_selection_names_joined();
        if broke > 0 {
            self.announce(format!(
                "Object selection: {name}; released {broke} clipping mask(s)"
            ));
        } else {
            self.announce(format!("Object selection: {name}"));
        }
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_layer_set_active(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::LayerIndex(index) = args else {
            return Err(CommandError::InvalidArgument("expected layer index"));
        };
        let generation = {
            let Some(graph) = self.graph.as_mut() else {
                return Err(CommandError::Document(DocumentError::NoDocument));
            };
            if !graph.set_active_index(index.max(0) as usize) {
                return Err(CommandError::Rejected("invalid layer index"));
            }
            graph.generation
        };
        self.sync_object_selection_to_active();
        let had_preview = self.filter_preview.is_some();
        self.invalidate_filter_preview();
        let name = self.object_selection_names_joined();
        self.announce(format!("Object selection: {name}"));
        Ok(CommandEffects {
            recomposite: had_preview,
            dirty: false,
            sync_layers: true,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        })
    }

    fn cmd_layer_set_visibility(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SetVisibility { index, visible } = args else {
            return Err(CommandError::InvalidArgument("expected visibility args"));
        };
        let id = self
            .graph
            .as_ref()
            .and_then(|g| g.layers().get(index.max(0) as usize).map(|l| l.id))
            .ok_or(CommandError::Rejected("invalid layer index"))?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !undo_actions::set_visibility(graph, history, id, visible) {
            return Err(CommandError::Rejected("set visibility failed"));
        }
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_layer_set_opacity(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SetOpacity { opacity } = args else {
            return Err(CommandError::InvalidArgument("expected opacity"));
        };
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !undo_actions::set_opacity(graph, history, id, opacity) {
            return Err(CommandError::Rejected("set opacity failed"));
        }
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_layer_set_blend(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SetBlend { blend } = args else {
            return Err(CommandError::InvalidArgument("expected blend"));
        };
        let mode = BlendMode::from_str_label(&blend)
            .ok_or(CommandError::InvalidArgument("unknown blend mode"))?;
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !undo_actions::set_blend(graph, history, id, mode) {
            return Err(CommandError::Rejected("set blend failed"));
        }
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_layer_reorder(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::Reorder { to_index } = args else {
            return Err(CommandError::InvalidArgument("expected reorder index"));
        };
        let targets = self.structural_target_ids()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        for id in &targets {
            let Some(layer) = graph.get(*id) else {
                return Err(CommandError::Rejected("layer missing"));
            };
            if layer.locks.all || layer.locks.position {
                return Err(CommandError::Rejected("layer position locked"));
            }
        }
        let prev = graph.stack_order();
        let mut moving: Vec<LayerId> = Vec::new();
        let mut rest: Vec<LayerId> = Vec::new();
        for id in &prev {
            if targets.contains(id) {
                moving.push(*id);
            } else {
                rest.push(*id);
            }
        }
        if moving.is_empty() {
            return Err(CommandError::Rejected("no layers to reorder"));
        }
        let insert_at = (to_index.max(0) as usize).min(rest.len());
        let mut next = rest;
        next.splice(insert_at..insert_at, moving);
        if next == prev {
            return Err(CommandError::Rejected("reorder unchanged"));
        }
        if !graph.reorder_stack(&next) {
            return Err(CommandError::Rejected("reorder failed"));
        }
        graph.bump_generation();
        let generation = graph.generation;
        history.push_graph_applied(
            crate::GraphCommand::SetStackOrder { prev, next },
            if targets.len() == 1 {
                "Reorder layer"
            } else {
                "Reorder layers"
            },
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_layer_group(&mut self) -> Result<CommandEffects, CommandError> {
        let targets = self.structural_target_ids()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        for id in &targets {
            let Some(layer) = graph.get(*id) else {
                return Err(CommandError::Rejected("layer missing"));
            };
            if layer.locks.all {
                return Err(CommandError::Rejected("layer is locked"));
            }
            if layer.kind == LayerKind::Group {
                for other in &targets {
                    if *other != *id && graph.get(*other).is_some_and(|l| l.parent == Some(*id)) {
                        return Err(CommandError::Rejected(
                            "cannot group a group with its children",
                        ));
                    }
                }
            }
        }
        if graph.layer_count() >= MAX_LAYERS {
            return Err(CommandError::Document(DocumentError::layer_limit(
                MAX_LAYERS,
            )));
        }

        let insert_at = graph
            .index_of(targets[targets.len() - 1])
            .map(|i| i + 1)
            .unwrap_or(graph.layer_count())
            .min(graph.layer_count());

        let group_id = graph.add_group_top(Some("Group".into()))?;
        let _ = graph.move_layer(group_id, insert_at);
        let group_index = graph.index_of(group_id).unwrap_or(0);

        let mut parent_cmds = Vec::new();
        for id in &targets {
            let Some(prev) = graph.set_parent(*id, Some(group_id)) else {
                for cmd in parent_cmds.iter().rev() {
                    if let crate::GraphCommand::SetParent { id, prev, .. } = cmd {
                        let _ = graph.set_parent(*id, *prev);
                    }
                }
                let _ = graph.remove_layer(group_id);
                return Err(CommandError::Rejected("set parent failed"));
            };
            parent_cmds.push(crate::GraphCommand::SetParent {
                id: *id,
                prev,
                next: Some(group_id),
            });
        }

        let mut batch = vec![crate::GraphCommand::AddLayer {
            id: group_id,
            index: group_index,
            layer: graph
                .get(group_id)
                .cloned()
                .ok_or(CommandError::Document(DocumentError::LayerMissingAfterAdd))?,
        }];
        batch.extend(parent_cmds);

        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_graph_applied(
            crate::GraphCommand::Batch(batch),
            "Group layers",
            generation,
        );
        let _ = graph.set_active(group_id);
        self.selected_layer_ids = vec![group_id];
        self.announce("Grouped layers");
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_layer_ungroup(&mut self) -> Result<CommandEffects, CommandError> {
        let targets = self.structural_target_ids()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let groups: Vec<LayerId> = targets
            .iter()
            .copied()
            .filter(|id| graph.get(*id).is_some_and(|l| l.kind == LayerKind::Group))
            .collect();
        if groups.is_empty() {
            return Err(CommandError::Rejected("no group selected"));
        }
        for id in &groups {
            if graph.get(*id).is_some_and(|l| l.locks.all) {
                return Err(CommandError::Rejected("group is locked"));
            }
        }

        let prev_active = graph.active_id();
        let mut batch = Vec::new();
        for group_id in &groups {
            let children: Vec<LayerId> = graph
                .layers()
                .iter()
                .filter(|l| l.parent == Some(*group_id))
                .map(|l| l.id)
                .collect();
            let group_parent = graph.get(*group_id).and_then(|l| l.parent);
            for child in children {
                let Some(prev) = graph.set_parent(child, group_parent) else {
                    return Err(CommandError::Rejected("ungroup parent failed"));
                };
                batch.push(crate::GraphCommand::SetParent {
                    id: child,
                    prev,
                    next: group_parent,
                });
            }
            let Some((index, layer)) = graph.remove_layer(*group_id) else {
                return Err(CommandError::Rejected("ungroup delete failed"));
            };
            batch.push(crate::GraphCommand::DeleteLayer {
                id: *group_id,
                index,
                layer,
                prev_active,
            });
        }
        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_graph_applied(
            crate::GraphCommand::Batch(batch),
            "Ungroup layers",
            generation,
        );
        self.sync_object_selection_to_active();
        self.announce("Ungrouped layers");
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_layer_set_clip(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::LayerSetClip { clips } = args else {
            return Err(CommandError::InvalidArgument("expected clip flag"));
        };
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(prev) = graph.set_clips_to_below(id, clips) else {
            return Err(CommandError::Rejected("set clip failed"));
        };
        if prev == clips {
            return Err(CommandError::Rejected("clip unchanged"));
        }
        history.push_graph_applied(
            crate::GraphCommand::SetClipsToBelow {
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
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_layer_set_locks(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SetLocks {
            pixels,
            position,
            all,
            alpha,
        } = args
        else {
            return Err(CommandError::InvalidArgument("expected SetLocks"));
        };
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        let prev = layer.locks;
        let next = crate::LockFlags {
            pixels,
            position,
            all,
            alpha,
        };
        if prev == next {
            return Err(CommandError::Rejected("locks unchanged"));
        }
        layer.locks = next;
        layer.locked = all;
        history.push_graph_applied(
            crate::GraphCommand::SetLocks { id, prev, next },
            "Set layer locks",
            {
                graph.bump_generation();
                graph.generation
            },
        );
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_view_zoom(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::Zoom { zoom } = args else {
            return Err(CommandError::InvalidArgument("expected zoom"));
        };
        self.set_zoom(zoom);
        let mut e = CommandEffects::view_only();
        e.generation = self.document_generation();
        Ok(e)
    }

    fn cmd_view_pan(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::Pan { world_x, world_y } = args else {
            return Err(CommandError::InvalidArgument("expected pan"));
        };
        self.set_pan(world_x, world_y);
        let mut e = CommandEffects::view_only();
        e.generation = self.document_generation();
        Ok(e)
    }

    fn cmd_view_pan_by(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::PanBy { dx, dy } = args else {
            return Err(CommandError::InvalidArgument("expected pan-by"));
        };
        self.pan_by(dx, dy);
        let mut e = CommandEffects::view_only();
        e.generation = self.document_generation();
        Ok(e)
    }

    fn cmd_view_zoom_at(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::ZoomAt {
            factor,
            anchor_x,
            anchor_y,
        } = args
        else {
            return Err(CommandError::InvalidArgument("expected zoom-at"));
        };
        self.zoom_at(factor, anchor_x, anchor_y);
        let mut e = CommandEffects::view_only();
        e.generation = self.document_generation();
        Ok(e)
    }

    fn cmd_view_set_tool(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::Tool { tool } = args else {
            return Err(CommandError::InvalidArgument("expected tool"));
        };
        if tool.is_empty() {
            return Err(CommandError::InvalidArgument("empty tool id"));
        }
        let _ = tool_id::BRUSH;
        let tool_changed = self.active_tool != tool;
        self.set_active_tool(&tool);
        if tool_changed {
            self.invalidate_filter_preview();
        }
        let mut e = CommandEffects::view_only();
        e.sync_layers = false;
        e.generation = self.document_generation();
        if tool_changed && self.filter_preview.is_none() {
            e.recomposite = true;
        }
        Ok(e)
    }

    fn invalidate_filter_preview(&mut self) {
        if let Some(preview) = self.filter_preview.take() {
            preview.cancel.cancel();
        }
    }

    fn cmd_document_new_preset(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::NewPreset { label } = args else {
            return Err(CommandError::InvalidArgument("expected preset label"));
        };
        let preset = crate::SizePreset::from_label(&label)
            .ok_or(CommandError::InvalidArgument("unknown size preset"))?;
        self.apply_preset(preset);
        if let Some(graph) = self.graph.as_mut() {
            graph.generation = 1;
        }
        self.last_persisted_generation = None;
        self.invalidate_filter_preview();
        self.path_edit_anchor = None;
        Ok(CommandEffects {
            recomposite: true,
            dirty: false,
            sync_layers: true,
            sync_camera: true,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: self.document_generation(),
        })
    }

    fn cmd_document_new_size(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::NewSize { width, height } = args else {
            return Err(CommandError::InvalidArgument("expected size"));
        };
        self.apply_size(crate::DocumentSize::new(width, height));
        if let Some(graph) = self.graph.as_mut() {
            graph.generation = 1;
        }
        self.last_persisted_generation = None;
        Ok(CommandEffects {
            recomposite: true,
            dirty: false,
            sync_layers: true,
            sync_camera: true,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: self.document_generation(),
        })
    }

    fn cmd_document_assign_profile(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::AssignProfile { profile } = args else {
            return Err(CommandError::InvalidArgument("expected profile"));
        };
        if profile.trim().is_empty() {
            return Err(CommandError::InvalidArgument("empty profile"));
        }
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        graph.color.assign_profile(profile);
        graph.bump_generation();
        Ok(CommandEffects {
            recomposite: false,
            dirty: true,
            sync_layers: false,
            sync_camera: false,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: graph.generation,
        })
    }

    fn cmd_document_convert_profile(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::ConvertProfile { profile } = args else {
            return Err(CommandError::InvalidArgument("expected profile"));
        };
        if profile.trim().is_empty() {
            return Err(CommandError::InvalidArgument("empty profile"));
        }
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let from = graph.color.assigned_profile.clone();
        let plan = graph.color.begin_convert(profile.clone());
        if !plan.rewrite_pixels {
            graph.color.mark_converted();
        }
        graph.bump_generation();
        let mut effects = CommandEffects {
            recomposite: plan.rewrite_pixels,
            dirty: true,
            sync_layers: false,
            sync_camera: false,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: graph.generation,
        };
        if plan.rewrite_pixels {
            effects.host_follow_up = HostFollowUp::ConvertPixels { from, to: profile };
        }
        Ok(effects)
    }

    fn cmd_document_set_icc(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SetIcc { bytes } = args else {
            return Err(CommandError::InvalidArgument("expected SetIcc"));
        };
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let prev = graph.color.embedded_icc.clone();
        if prev == bytes {
            return Err(CommandError::Rejected("ICC unchanged"));
        }
        if let Err(reason) = graph.color.set_embedded_icc(bytes) {
            return Err(CommandError::Rejected(reason));
        }
        graph.bump_generation();
        let generation = graph.generation;
        let label = if graph.color.has_embedded_icc() {
            "Embed ICC"
        } else {
            "Clear ICC"
        };
        self.history.push_transform(label, generation);
        self.announce(label);
        Ok(CommandEffects {
            recomposite: false,
            dirty: true,
            sync_layers: false,
            sync_camera: false,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        })
    }

    fn cmd_document_set_soft_proof(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SoftProof { profile, intent } = args else {
            return Err(CommandError::InvalidArgument("expected SoftProof"));
        };
        let (generation, message) = {
            let Some(graph) = self.graph.as_mut() else {
                return Err(CommandError::Document(DocumentError::NoDocument));
            };
            graph.color.set_soft_proof(profile.clone(), intent);
            let message = if graph.color.soft_proof_active() {
                format!("Soft-proof: {profile}")
            } else {
                "Soft-proof off".into()
            };
            (graph.generation, message)
        };
        self.announce(message);
        Ok(CommandEffects {
            recomposite: false,
            dirty: false,
            sync_layers: false,
            sync_camera: false,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        })
    }

    fn cmd_document_crop(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::DocumentCrop { width, height } = args else {
            return Err(CommandError::InvalidArgument("expected crop size"));
        };
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        graph.size = crate::DocumentSize::new(width, height);
        for layer in graph.layers_mut() {
            layer.transform = LayerTransform::identity();
        }
        graph.revision = graph.revision.wrapping_add(1);
        self.size = graph.size;
        self.selection.clear();
        let generation = {
            graph.bump_generation();
            graph.generation
        };
        self.history.push_transform("Crop", generation);
        self.zoom_to_fit();
        let mut effects = CommandEffects::document_edit(generation);
        effects.sync_selection = true;
        effects.sync_camera = true;
        Ok(effects)
    }

    fn cmd_document_rotate_90(&mut self) -> Result<CommandEffects, CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let (w, h) = (graph.size.width, graph.size.height);
        graph.size = crate::DocumentSize::new(h, w);
        graph.revision = graph.revision.wrapping_add(1);
        self.size = graph.size;
        self.selection.clear();
        let generation = {
            graph.bump_generation();
            graph.generation
        };
        self.history.push_transform("Rotate 90° CW", generation);
        self.zoom_to_fit();
        let mut effects = CommandEffects::document_edit(generation);
        effects.sync_selection = true;
        effects.sync_camera = true;
        Ok(effects)
    }

    fn cmd_selection_replace(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SelectionReplace {
            shape,
            combine,
            rect,
            polygon,
            label,
        } = args
        else {
            return Err(CommandError::InvalidArgument("expected selection replace"));
        };
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        match shape {
            SelectionShape::Rect => {
                if rect.width == 0 || rect.height == 0 {
                    return Err(CommandError::Rejected("empty selection"));
                }
                self.selection.set_rect(rect, combine);
            }
            SelectionShape::Ellipse => {
                if rect.width == 0 || rect.height == 0 {
                    return Err(CommandError::Rejected("empty selection"));
                }
                self.selection.set_ellipse(rect, combine);
            }
            SelectionShape::Mask => {
                if polygon.len() < 3 {
                    return Err(CommandError::Rejected("polygon needs 3+ points"));
                }
                let bounds = crate::SelectionState::polygon_bounds(&polygon)
                    .ok_or(CommandError::Rejected("invalid polygon bounds"))?;
                self.selection.set_mask_polygon(bounds, combine);
            }
        }
        let generation = self.bump_document_generation();
        self.history.push_selection(label, generation);
        Ok(CommandEffects::selection_edit(generation))
    }

    fn cmd_selection_deselect(&mut self) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        self.selection.clear();
        let generation = self.bump_document_generation();
        self.history.push_selection("Deselect", generation);
        Ok(CommandEffects::selection_edit(generation))
    }

    fn cmd_selection_invert(&mut self) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        self.selection
            .invert_bounds(self.size.width, self.size.height);
        let generation = self.bump_document_generation();
        self.history.push_selection("Invert selection", generation);
        Ok(CommandEffects::selection_edit(generation))
    }

    fn cmd_selection_select_all(&mut self) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        self.selection.select_all(self.size.width, self.size.height);
        let generation = self.bump_document_generation();
        self.history.push_selection("Select all", generation);
        Ok(CommandEffects::selection_edit(generation))
    }

    fn cmd_selection_modify(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SelectionModify { op, radius } = args else {
            return Err(CommandError::InvalidArgument("expected selection modify"));
        };
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        if op == "feather" {
            self.selection.feather = radius as f32;
        }
        let generation = self.bump_document_generation();
        self.history
            .push_selection(format!("Selection {op}"), generation);
        Ok(CommandEffects::selection_edit(generation))
    }

    fn cmd_selection_to_mask(&mut self) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        if !self.selection.active {
            return Err(CommandError::Rejected("no pixel selection"));
        }
        let _ = self.active_layer_id()?;
        self.announce("Selection → layer mask");
        Ok(CommandEffects::host_chrome(HostFollowUp::SelectionToMask))
    }

    fn cmd_mask_to_selection(&mut self) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        let id = self.active_layer_id()?;
        let has_mask = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .is_some_and(|layer| layer.mask.is_some());
        if !has_mask {
            return Err(CommandError::Rejected("no layer mask"));
        }
        self.announce("Layer mask → selection");
        Ok(CommandEffects::host_chrome(HostFollowUp::MaskToSelection))
    }

    fn cmd_mask_apply(&mut self) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        let id = self.active_layer_id()?;
        let Some(layer) = self.graph.as_ref().and_then(|g| g.get(id)) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.mask.is_none() {
            return Err(CommandError::Rejected("no layer mask"));
        }
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected("apply mask requires raster layer"));
        }
        if layer.paint_blocked() {
            return Err(CommandError::Rejected("layer is locked"));
        }
        self.announce("Apply layer mask");
        let mut effects = CommandEffects::host_chrome(HostFollowUp::ApplyMask);
        effects.recomposite = true;
        effects.dirty = true;
        effects.sync_layers = true;
        effects.generation = self.document_generation();
        Ok(effects)
    }

    fn cmd_mask_create(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let SessionState {
            graph,
            history,
            mask_edit_layer,
            ..
        } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if graph.get(id).is_some_and(|l| l.mask.is_some()) {
            return Err(CommandError::Rejected("mask already present"));
        }
        let prev = graph.get(id).and_then(|l| l.mask.clone());
        let next = Some(LayerMask::default());
        if graph.set_mask(id, next.clone()).is_none() {
            return Err(CommandError::Rejected("set mask failed"));
        }
        history.push_graph_applied(
            crate::GraphCommand::SetMask { id, prev, next },
            "Add layer mask",
            {
                graph.bump_generation();
                graph.generation
            },
        );
        *mask_edit_layer = Some(id);
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_mask_delete(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let SessionState {
            graph,
            history,
            mask_edit_layer,
            ..
        } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if graph.get(id).is_none_or(|l| l.mask.is_none()) {
            return Err(CommandError::Rejected("no mask"));
        }
        let prev = graph.get(id).and_then(|layer| layer.mask.clone());
        if graph.set_mask(id, None).is_none() {
            return Err(CommandError::Rejected("clear mask failed"));
        }
        history.push_graph_applied(
            crate::GraphCommand::SetMask {
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
        if *mask_edit_layer == Some(id) {
            *mask_edit_layer = None;
        }
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_mask_set_enabled(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::MaskSetEnabled { enabled } = args else {
            return Err(CommandError::InvalidArgument("expected mask enabled"));
        };
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(prev) = graph.get(id).and_then(|layer| layer.mask.clone()) else {
            return Err(CommandError::Rejected("no mask"));
        };
        if prev.enabled == enabled {
            return Err(CommandError::Rejected("mask enable unchanged"));
        }
        let mut next = prev.clone();
        next.enabled = enabled;
        if graph.set_mask(id, Some(next.clone())).is_none() {
            return Err(CommandError::Rejected("set mask failed"));
        }
        history.push_graph_applied(
            crate::GraphCommand::SetMask {
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
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_mask_set_attributes(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::MaskAttributes {
            enabled,
            linked,
            density,
            feather,
            inverted,
        } = args
        else {
            return Err(CommandError::InvalidArgument("expected MaskAttributes"));
        };
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(prev) = graph.get(id).and_then(|layer| layer.mask.clone()) else {
            return Err(CommandError::Rejected("no mask"));
        };
        let next = crate::LayerMask {
            enabled,
            linked,
            density: density.clamp(0.0, 1.0),
            feather: feather.max(0.0),
            inverted,
        };
        if prev == next {
            return Err(CommandError::Rejected("mask attributes unchanged"));
        }
        if graph.set_mask(id, Some(next.clone())).is_none() {
            return Err(CommandError::Rejected("set mask failed"));
        }
        history.push_graph_applied(
            crate::GraphCommand::SetMask {
                id,
                prev: Some(prev),
                next: Some(next),
            },
            "Set mask attributes",
            {
                graph.bump_generation();
                graph.generation
            },
        );
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_mask_create_vector(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.vector_mask.is_some() {
            return Err(CommandError::Rejected("vector mask already present"));
        }
        layer.vector_mask = Some(crate::VectorMask::default());
        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_transform("Add vector mask", generation);
        self.announce("Vector mask added (path edit deferred)");
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_text_create(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::TextCreate { text } = args else {
            return Err(CommandError::InvalidArgument("expected text"));
        };
        let content = TextContent {
            text,
            ..TextContent::default()
        };
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = graph.add_text_top(None, content)?;
        let index = graph.index_of(id).unwrap_or(0);
        let layer = graph
            .get(id)
            .cloned()
            .ok_or(CommandError::Document(DocumentError::LayerMissingAfterAdd))?;
        history.push_graph_applied(
            crate::GraphCommand::AddLayer { id, index, layer },
            "Add text layer",
            {
                graph.bump_generation();
                graph.generation
            },
        );
        let mut effects = CommandEffects::document_edit(graph.generation);
        effects.created_layer = Some(id);
        Ok(effects)
    }

    fn cmd_text_set_content(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::TextSetContent { content } = args else {
            return Err(CommandError::InvalidArgument("expected text content"));
        };
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Text {
            return Err(CommandError::Rejected("not a text layer"));
        }
        layer.text = Some(content);
        graph.bump_generation();
        Ok(CommandEffects {
            recomposite: false,
            dirty: true,
            sync_layers: true,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: graph.generation,
        })
    }

    fn cmd_text_bake(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Text {
            return Err(CommandError::Rejected("not a text layer"));
        }
        layer.kind = LayerKind::Raster;
        layer.text = None;
        if layer.asset_key.is_none() {
            layer.asset_key = Some(format!("layer-{}", id.0));
        }
        graph.bump_generation();
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_shape_create(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::ShapeCreate { content } = args else {
            return Err(CommandError::InvalidArgument("expected shape content"));
        };
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = graph.add_shape_top(None, content)?;
        let index = graph.index_of(id).unwrap_or(0);
        let layer = graph
            .get(id)
            .cloned()
            .ok_or(CommandError::Document(DocumentError::LayerMissingAfterAdd))?;
        history.push_graph_applied(
            crate::GraphCommand::AddLayer { id, index, layer },
            "Add shape layer",
            {
                graph.bump_generation();
                graph.generation
            },
        );
        let mut effects = CommandEffects::document_edit(graph.generation);
        effects.created_layer = Some(id);
        Ok(effects)
    }

    fn cmd_shape_rasterize(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Shape {
            return Err(CommandError::Rejected("not a shape layer"));
        }
        layer.kind = LayerKind::Raster;
        layer.shape = None;
        if layer.asset_key.is_none() {
            layer.asset_key = Some(format!("layer-{}", id.0));
        }
        graph.bump_generation();
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn resolve_boolean_shape_pair(&self) -> Result<(LayerId, LayerId), CommandError> {
        let Some(graph) = self.graph.as_ref() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let selected: Vec<LayerId> = self
            .selected_layer_ids
            .iter()
            .copied()
            .filter(|id| {
                graph
                    .get(*id)
                    .is_some_and(|l| l.kind == LayerKind::Shape && l.shape.is_some())
            })
            .collect();
        if selected.len() >= 2 {
            return Ok((selected[0], selected[1]));
        }
        let active = self.active_layer_id()?;
        let Some(active_layer) = graph.get(active) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if active_layer.kind != LayerKind::Shape || active_layer.shape.is_none() {
            return Err(CommandError::Rejected("active layer is not a shape"));
        }
        let idx = graph
            .index_of(active)
            .ok_or(CommandError::Rejected("layer missing"))?;
        for i in (0..idx).rev() {
            let id = graph.layers()[i].id;
            if graph
                .get(id)
                .is_some_and(|l| l.kind == LayerKind::Shape && l.shape.is_some())
            {
                return Ok((active, id));
            }
        }
        Err(CommandError::Rejected(
            "need two shape layers (select two, or stack a shape below active)",
        ))
    }

    fn cmd_shape_boolean(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::ShapeBoolean { op } = args else {
            return Err(CommandError::InvalidArgument("expected ShapeBoolean"));
        };
        let op = crate::BooleanOp::parse(&op)
            .ok_or(CommandError::InvalidArgument("unknown boolean op"))?;
        let (a, b) = self.resolve_boolean_shape_pair()?;
        let label = format!("Boolean {}", op.as_str());
        self.announce(label.clone());
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if graph.layer_count() >= MAX_LAYERS {
            return Err(CommandError::Document(DocumentError::layer_limit(
                MAX_LAYERS,
            )));
        }
        let result = graph.add_layer_top(Some(label.clone()))?;
        let index = graph.index_of(result).unwrap_or(0);
        let layer = graph
            .get(result)
            .cloned()
            .ok_or(CommandError::Document(DocumentError::LayerMissingAfterAdd))?;
        let generation = {
            graph.bump_generation();
            graph.generation
        };
        history.push_graph_applied(
            crate::GraphCommand::AddLayer {
                id: result,
                index,
                layer,
            },
            &label,
            generation,
        );
        let mut effects = CommandEffects::document_edit(generation);
        effects.created_layer = Some(result);
        effects.host_follow_up = HostFollowUp::ShapeBoolean { op, a, b, result };
        Ok(effects)
    }

    fn cmd_filter_add_adjustment(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::FilterAdjustment { kind } = args else {
            return Err(CommandError::InvalidArgument("expected adjustment kind"));
        };
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
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = graph.add_adjustment_top(Some(name.into()), params)?;
        let index = graph.index_of(id).unwrap_or(0);
        let layer = graph
            .get(id)
            .cloned()
            .ok_or(CommandError::Document(DocumentError::LayerMissingAfterAdd))?;
        history.push_graph_applied(
            crate::GraphCommand::AddLayer { id, index, layer },
            "Add adjustment",
            {
                graph.bump_generation();
                graph.generation
            },
        );
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_filter_set_parameters(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::FilterParameters { p0, p1, p2 } = args else {
            return Err(CommandError::InvalidArgument("expected filter params"));
        };
        let id = self.active_layer_id()?;
        let prev = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .and_then(|l| l.adjustment.clone())
            .ok_or(CommandError::Rejected("no adjustment"))?;
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
            return Err(CommandError::Rejected("adjustment unchanged"));
        }
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if graph.set_adjustment(id, Some(next.clone())).is_none() {
            return Err(CommandError::Rejected("set adjustment failed"));
        }
        history.push_graph_applied(
            crate::GraphCommand::SetAdjustment {
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
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_filter_add_effect(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::FilterEffect { kind } = args else {
            return Err(CommandError::InvalidArgument("expected effect kind"));
        };
        let id = self.active_layer_id()?;
        let is_raster = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .is_some_and(|l| l.kind == LayerKind::Raster);
        if !is_raster {
            return Err(CommandError::Rejected("effect requires raster layer"));
        }
        let Some((prev, _)) = (match kind.as_str() {
            "gaussian" => self
                .graph
                .as_mut()
                .and_then(|g| g.add_gaussian_blur(id, 4.0)),
            "motion" => self
                .graph
                .as_mut()
                .and_then(|g| g.add_motion_blur(id, 8.0, 0.0)),
            "emboss" => self
                .graph
                .as_mut()
                .and_then(|g| g.add_emboss(id, 1.0, 135.0)),
            "sharpen" => self.graph.as_mut().and_then(|g| g.add_sharpen(id, 1.0)),
            _ => None,
        }) else {
            return Err(CommandError::InvalidArgument("unknown effect kind"));
        };
        let next = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.effects.clone())
            .unwrap_or_default();
        let label = match kind.as_str() {
            "gaussian" => "Gaussian Blur",
            "motion" => "Motion Blur",
            "emboss" => "Emboss",
            "sharpen" => "Sharpen",
            other => other,
        };
        let generation = self.bump_document_generation();
        self.history.push_graph_applied(
            crate::GraphCommand::SetEffects { id, prev, next },
            label,
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_filter_set_gaussian_radius(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::FilterGaussianRadius { radius } = args else {
            return Err(CommandError::InvalidArgument("expected radius"));
        };
        let id = self.active_layer_id()?;
        let Some(prev) = self
            .graph
            .as_mut()
            .and_then(|g| g.set_gaussian_radius(id, radius))
        else {
            return Err(CommandError::Rejected("no gaussian blur"));
        };
        let next = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.effects.clone())
            .unwrap_or_default();
        if next == prev {
            return Err(CommandError::Rejected("radius unchanged"));
        }
        let generation = self.bump_document_generation();
        self.history.push_graph_applied(
            crate::GraphCommand::SetEffects { id, prev, next },
            "Blur radius",
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_filter_preview(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::FilterPreview { kind } = args else {
            return Err(CommandError::InvalidArgument("expected FilterPreview"));
        };
        if !crate::kind_is_supported(&kind) {
            return Err(CommandError::InvalidArgument("unsupported gallery kind"));
        }
        let id = self.active_layer_id()?;
        let is_raster = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .is_some_and(|l| l.kind == LayerKind::Raster);
        if !is_raster {
            return Err(CommandError::Rejected("effect requires raster layer"));
        }
        let generation = self.document_generation();
        self.filter_preview = Some(crate::FilterPreviewSession::new(id, kind, generation));
        self.announce("Filter preview");
        Ok(CommandEffects {
            recomposite: true,
            dirty: false,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        })
    }

    fn cmd_filter_set_preview_params(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::FilterPreviewParams { p0, p1, p2 } = args else {
            return Err(CommandError::InvalidArgument("expected preview params"));
        };
        let generation = self.document_generation();
        let Some(preview) = self.filter_preview.as_mut() else {
            return Err(CommandError::Rejected("no filter preview"));
        };
        if preview.is_cancelled() {
            return Err(CommandError::Rejected("filter preview cancelled"));
        }
        if preview.is_stale(generation) {
            return Err(CommandError::Rejected("filter preview stale"));
        }
        preview.set_params(p0, p1, p2);
        Ok(CommandEffects {
            recomposite: true,
            dirty: false,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        })
    }

    fn cmd_filter_cancel_preview(&mut self) -> Result<CommandEffects, CommandError> {
        let had = self.filter_preview.is_some();
        self.invalidate_filter_preview();
        if !had {
            return Err(CommandError::Rejected("no filter preview"));
        }
        self.announce("Filter preview cancelled");
        Ok(CommandEffects {
            recomposite: true,
            dirty: false,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: self.document_generation(),
        })
    }

    fn cmd_filter_commit(&mut self) -> Result<CommandEffects, CommandError> {
        let preview = self
            .filter_preview
            .take()
            .ok_or(CommandError::Rejected("no filter preview"))?;
        if preview.is_cancelled() {
            return Err(CommandError::Rejected("filter preview cancelled"));
        }
        let generation_now = self.document_generation();
        if preview.is_stale(generation_now) {
            return Err(CommandError::Rejected("filter preview stale"));
        }
        let id = preview.layer_id;
        let effect = preview
            .to_effect()
            .ok_or(CommandError::InvalidArgument("unsupported gallery kind"))?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected("effect requires raster layer"));
        }
        let prev_effects = layer.effects.clone();
        let prev_plan = layer.filter_plan.clone();
        let effect_id = prev_effects
            .iter()
            .map(|e| e.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut committed = effect;
        committed.id = effect_id;
        let mut next_effects = prev_effects.clone();
        next_effects.push(committed);
        let mut next_plan = prev_plan.clone();
        let mut params = serde_json::Map::new();
        params.insert("p0".into(), serde_json::json!(preview.p0));
        params.insert("p1".into(), serde_json::json!(preview.p1));
        params.insert("p2".into(), serde_json::json!(preview.p2));
        next_plan.push_node(crate::FilterPlanNode {
            id: format!("fx-{effect_id}"),
            kind: preview.kind.clone(),
            enabled: true,
            params,
        });
        let _ = graph.set_effects(id, next_effects.clone());
        if let Some(layer) = graph.get_mut(id) {
            layer.filter_plan = next_plan.clone();
        }
        let label = preview.label();
        let generation = self.bump_document_generation();
        self.history.push_graph_applied(
            crate::GraphCommand::Batch(vec![
                crate::GraphCommand::SetEffects {
                    id,
                    prev: prev_effects,
                    next: next_effects,
                },
                crate::GraphCommand::SetFilterPlan {
                    id,
                    prev: prev_plan,
                    next: next_plan,
                },
            ]),
            label,
            generation,
        );
        self.announce(format!("{label} applied"));
        Ok(CommandEffects::document_edit(generation))
    }

    /// Resolve mutable path target: shape layer path when active is Shape, else document paths.
    fn with_active_path_mut(
        &mut self,
        f: impl FnOnce(&mut crate::paths::VectorPath) -> Result<(), CommandError>,
    ) -> Result<(Option<LayerId>, crate::GraphCommand), CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let active = graph
            .active_id()
            .ok_or(CommandError::Rejected("no active layer"))?;
        let Some(layer) = graph.get(active) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.locked || layer.locks.all || layer.locks.position {
            return Err(CommandError::Rejected("layer locked"));
        }
        if layer.kind == LayerKind::Shape {
            let prev = layer
                .shape
                .clone()
                .ok_or(CommandError::Rejected("shape missing path"))?;
            let mut next = prev.clone();
            f(&mut next.path)?;
            if let Some(layer) = graph.get_mut(active) {
                layer.shape = Some(next.clone());
            }
            Ok((
                Some(active),
                crate::GraphCommand::SetShape {
                    id: active,
                    prev: Some(prev),
                    next: Some(next),
                },
            ))
        } else {
            let prev = graph.paths.clone();
            let idx = graph
                .paths
                .active
                .ok_or(CommandError::Rejected("no active path"))?;
            if idx >= graph.paths.paths.len() {
                return Err(CommandError::Rejected("no active path"));
            }
            f(&mut graph.paths.paths[idx])?;
            let next = graph.paths.clone();
            Ok((None, crate::GraphCommand::SetPaths { prev, next }))
        }
    }

    fn cmd_path_set_closed(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::PathSetClosed { closed } = args else {
            return Err(CommandError::InvalidArgument("expected PathSetClosed"));
        };
        let (shape_id, cmd) = self.with_active_path_mut(|path| {
            path.closed = closed;
            Ok(())
        })?;
        let generation = self.bump_document_generation();
        self.history.push_graph_applied(
            cmd,
            if closed { "Close path" } else { "Open path" },
            generation,
        );
        let mut effects = CommandEffects::document_edit(generation);
        if let Some(id) = shape_id {
            effects.host_follow_up = HostFollowUp::RasterizeShape { id };
        }
        Ok(effects)
    }

    fn cmd_path_move_anchor(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::PathMoveAnchor { index, x, y } = args else {
            return Err(CommandError::InvalidArgument("expected PathMoveAnchor"));
        };
        let (shape_id, cmd) = self.with_active_path_mut(|path| {
            let anchor = path
                .anchors
                .get_mut(index)
                .ok_or(CommandError::Rejected("anchor missing"))?;
            anchor.x = x;
            anchor.y = y;
            Ok(())
        })?;
        self.path_edit_anchor = Some(index);
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(cmd, "Move path anchor", generation);
        let mut effects = CommandEffects::document_edit(generation);
        if let Some(id) = shape_id {
            effects.host_follow_up = HostFollowUp::RasterizeShape { id };
        }
        Ok(effects)
    }

    fn cmd_path_add_anchor(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::PathAddAnchor { x, y, index } = args else {
            return Err(CommandError::InvalidArgument("expected PathAddAnchor"));
        };
        let mut inserted_at = 0usize;
        let (shape_id, cmd) = self.with_active_path_mut(|path| {
            let point = crate::paths::PathPoint { x, y };
            inserted_at = index.unwrap_or(path.anchors.len()).min(path.anchors.len());
            path.anchors.insert(inserted_at, point);
            if !path.controls.is_empty() {
                path.controls.insert(
                    inserted_at,
                    (
                        crate::paths::PathPoint { x, y },
                        crate::paths::PathPoint { x, y },
                    ),
                );
            }
            Ok(())
        })?;
        self.path_edit_anchor = Some(inserted_at);
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(cmd, "Add path anchor", generation);
        let mut effects = CommandEffects::document_edit(generation);
        if let Some(id) = shape_id {
            effects.host_follow_up = HostFollowUp::RasterizeShape { id };
        }
        Ok(effects)
    }

    fn cmd_path_delete_anchor(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::PathDeleteAnchor { index } = args else {
            return Err(CommandError::InvalidArgument("expected PathDeleteAnchor"));
        };
        let (shape_id, cmd) = self.with_active_path_mut(|path| {
            if index >= path.anchors.len() {
                return Err(CommandError::Rejected("anchor missing"));
            }
            if path.anchors.len() <= 2 {
                return Err(CommandError::Rejected("need at least two anchors"));
            }
            path.anchors.remove(index);
            if index < path.controls.len() {
                path.controls.remove(index);
            }
            Ok(())
        })?;
        self.path_edit_anchor = None;
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(cmd, "Delete path anchor", generation);
        let mut effects = CommandEffects::document_edit(generation);
        if let Some(id) = shape_id {
            effects.host_follow_up = HostFollowUp::RasterizeShape { id };
        }
        Ok(effects)
    }

    fn cmd_effect_reorder(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::EffectReorder {
            effect_id,
            to_index,
        } = args
        else {
            return Err(CommandError::InvalidArgument("expected EffectReorder"));
        };
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        let prev = layer.effects.clone();
        let Some(from) = prev.iter().position(|e| e.id == effect_id) else {
            return Err(CommandError::Rejected("effect missing"));
        };
        let mut next = prev.clone();
        let effect = next.remove(from);
        let to = (to_index.max(0) as usize).min(next.len());
        next.insert(to, effect);
        if next.iter().map(|e| e.id).eq(prev.iter().map(|e| e.id)) {
            return Err(CommandError::Rejected("effect order unchanged"));
        }
        let _ = graph.set_effects(id, next.clone());
        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_graph_applied(
            crate::GraphCommand::SetEffects { id, prev, next },
            "Reorder effects",
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_effect_set_enabled(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::EffectSetEnabled { effect_id, enabled } = args else {
            return Err(CommandError::InvalidArgument("expected EffectSetEnabled"));
        };
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        let prev = layer.effects.clone();
        let mut next = prev.clone();
        let Some(effect) = next.iter_mut().find(|e| e.id == effect_id) else {
            return Err(CommandError::Rejected("effect missing"));
        };
        if effect.enabled == enabled {
            return Err(CommandError::Rejected("effect enable unchanged"));
        }
        effect.enabled = enabled;
        let _ = graph.set_effects(id, next.clone());
        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_graph_applied(
            crate::GraphCommand::SetEffects { id, prev, next },
            if enabled {
                "Enable effect"
            } else {
                "Disable effect"
            },
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_style_add_drop_shadow(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected("drop shadow requires raster"));
        }
        layer.styles.push(LayerStyle::drop_shadow_default());
        graph.bump_generation();
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_style_add_stroke(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected("stroke style requires raster"));
        }
        layer.styles.push(LayerStyle::stroke_default());
        graph.bump_generation();
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_style_add_outer_glow(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected("outer glow requires raster"));
        }
        layer.styles.push(LayerStyle::outer_glow_default());
        graph.bump_generation();
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_style_add_color_overlay(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected("color overlay requires raster"));
        }
        layer.styles.push(LayerStyle::color_overlay_default());
        graph.bump_generation();
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_clipboard_paste_layer(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let name = match args {
            CommandArgs::PasteLayer { name } => Some(name),
            CommandArgs::None => Some("Pasted".into()),
            _ => return Err(CommandError::InvalidArgument("expected paste args")),
        };
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = undo_actions::add_layer(graph, history, name)?;
        let mut effects = CommandEffects::document_edit(graph.generation);
        effects.created_layer = Some(id);
        Ok(effects)
    }

    fn cmd_path_stroke_to_layer(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::PathStroke { layer_name } = args else {
            return Err(CommandError::InvalidArgument("expected path stroke"));
        };
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = undo_actions::add_layer(graph, history, Some(layer_name))?;
        let mut effects = CommandEffects::document_edit(graph.generation);
        effects.created_layer = Some(id);
        Ok(effects)
    }

    fn cmd_raster_transform_commit(&mut self) -> Result<CommandEffects, CommandError> {
        let Some(session) = self.transform_session.take() else {
            return Err(CommandError::Rejected("no transform session"));
        };
        if let Some(graph) = self.graph.as_ref() {
            if let Some(layer) = graph.get(session.layer_id) {
                if layer.position_blocked() {
                    self.transform_session = Some(session);
                    return Err(CommandError::Rejected("layer position locked"));
                }
            }
        }
        if let Some(graph) = self.graph.as_mut() {
            if let Some(layer) = graph.get_mut(session.layer_id) {
                layer.transform = LayerTransform::identity();
            }
            graph.revision = graph.revision.wrapping_add(1);
        }
        let generation = self.bump_document_generation();
        self.history.push_transform("Free Transform", generation);
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_raster_flip(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::RasterFlip { horizontal } = args else {
            return Err(CommandError::InvalidArgument("expected flip"));
        };
        self.assert_active_movable()?;
        let generation = self.bump_document_generation();
        self.history.push_transform(
            if horizontal {
                "Flip Horizontal"
            } else {
                "Flip Vertical"
            },
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_raster_history(&mut self, label: &str) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        self.assert_active_paintable()?;
        let generation = self.bump_document_generation();
        self.history.push_transform(label, generation);
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_raster_paint_stroke(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let label = match args {
            CommandArgs::RasterPaintStroke { label } => label,
            CommandArgs::None => "Brush stroke".into(),
            _ => return Err(CommandError::InvalidArgument("expected stroke label")),
        };
        if self.paint_target() == PaintTarget::LayerPixels {
            self.assert_active_paintable()?;
        }
        let generation = self.bump_document_generation();
        self.history.push_stroke(label, generation);
        Ok(CommandEffects {
            recomposite: false,
            dirty: true,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SizePreset;

    #[test]
    fn registry_lists_builtins() {
        assert!(SessionState::command_known(command_id::LAYER_CREATE));
        assert!(SessionState::command_known(command_id::SELECTION_REPLACE));
        assert!(SessionState::command_known(command_id::MASK_CREATE));
        assert!(!SessionState::command_known("layer.nope"));
    }

    #[test]
    fn layer_create_via_command() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let n = s.layer_count();
        let effects = s
            .invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("create");
        assert!(effects.dirty);
        assert!(effects.recomposite);
        assert_eq!(s.layer_count(), n + 1);
        assert!(s.document_generation() >= 1);
        assert!(s.can_undo());
    }

    #[test]
    fn undo_graph_via_command() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let n = s.layer_count();
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("create");
        let effects = s
            .invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo");
        assert!(effects.host_history.is_none());
        assert_eq!(s.layer_count(), n);
    }

    #[test]
    fn opacity_via_command() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(
            command_id::LAYER_SET_OPACITY,
            CommandArgs::SetOpacity { opacity: 0.4 },
        )
        .expect("opacity");
        let opacity = s
            .graph
            .as_ref()
            .and_then(|g| {
                let id = g.active_id()?;
                g.get(id).map(|layer| layer.opacity)
            })
            .expect("layer");
        assert!((opacity - 0.4).abs() < 1e-5);
    }

    #[test]
    fn selection_replace_via_command() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(
            command_id::SELECTION_REPLACE,
            CommandArgs::SelectionReplace {
                shape: SelectionShape::Rect,
                combine: SelectionCombine::Replace,
                rect: SelectionRect {
                    x: 10,
                    y: 10,
                    width: 40,
                    height: 40,
                },
                polygon: Vec::new(),
                label: "Rectangular selection".into(),
            },
        )
        .expect("select");
        assert!(s.selection.active);
        assert!(s.can_undo());
    }

    #[test]
    fn mask_create_via_command() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(command_id::MASK_CREATE, CommandArgs::None)
            .expect("mask");
        let id = s.graph.as_ref().and_then(|g| g.active_id()).expect("id");
        assert!(
            s.graph
                .as_ref()
                .and_then(|g| g.get(id))
                .unwrap()
                .mask
                .is_some()
        );
        assert_eq!(s.mask_edit_layer, Some(id));
    }

    #[test]
    fn layer_group_via_command() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let n = s.layer_count();
        s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
            .expect("group");
        assert_eq!(s.layer_count(), n + 1);
    }

    #[test]
    fn multi_delete_is_atomic_one_undo() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("l1");
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("l2");
        let ids: Vec<_> = s
            .graph
            .as_ref()
            .unwrap()
            .layers()
            .iter()
            .map(|l| l.id)
            .collect();
        // Keep bottom layer; delete the two above.
        s.set_object_selection(vec![ids[1], ids[2]]);
        let n = s.layer_count();
        s.invoke(command_id::LAYER_DELETE, CommandArgs::None)
            .expect("delete");
        assert_eq!(s.layer_count(), n - 2);
        s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo");
        assert_eq!(s.layer_count(), n);
    }

    #[test]
    fn multi_delete_rejects_locked() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("l1");
        let n = s.layer_count();
        let ids: Vec<_> = s
            .graph
            .as_ref()
            .unwrap()
            .layers()
            .iter()
            .map(|l| l.id)
            .collect();
        s.set_object_selection(vec![ids[0], ids[1]]);
        let _ = s.graph.as_mut().unwrap().get_mut(ids[1]).map(|l| {
            l.locks.all = true;
            l.locked = true;
        });
        let err = s
            .invoke(command_id::LAYER_DELETE, CommandArgs::None)
            .expect_err("locked");
        assert!(matches!(err, CommandError::Rejected(_)));
        assert_eq!(s.layer_count(), n);
    }

    #[test]
    fn group_selection_reparents() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("l1");
        let ids: Vec<_> = s
            .graph
            .as_ref()
            .unwrap()
            .layers()
            .iter()
            .map(|l| l.id)
            .collect();
        s.set_object_selection(ids.clone());
        s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
            .expect("group");
        let g = s.graph.as_ref().unwrap();
        let group_id = g.active_id().unwrap();
        assert_eq!(g.get(group_id).unwrap().kind, LayerKind::Group);
        for id in &ids {
            assert_eq!(g.get(*id).unwrap().parent, Some(group_id));
        }
        s.invoke(command_id::LAYER_UNGROUP, CommandArgs::None)
            .expect("ungroup");
        let g = s.graph.as_ref().unwrap();
        assert!(g.get(group_id).is_none());
        for id in &ids {
            assert!(g.get(*id).unwrap().parent.is_none());
        }
    }

    #[test]
    fn delete_clip_base_breaks_clip() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let ids = s.graph.as_ref().unwrap().stack_order();
        let base = ids[0];
        let top = ids[1];
        s.set_object_selection(vec![top]);
        s.invoke(
            command_id::LAYER_SET_CLIP,
            CommandArgs::LayerSetClip { clips: true },
        )
        .expect("clip");
        assert!(s.graph.as_ref().unwrap().get(top).unwrap().clips_to_below);
        s.set_object_selection(vec![base]);
        s.invoke(command_id::LAYER_DELETE, CommandArgs::None)
            .expect("delete base");
        assert!(s.graph.as_ref().unwrap().get(base).is_none());
        assert!(!s.graph.as_ref().unwrap().get(top).unwrap().clips_to_below);
        s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo");
        assert!(s.graph.as_ref().unwrap().get(base).is_some());
        assert!(s.graph.as_ref().unwrap().get(top).unwrap().clips_to_below);
    }

    #[test]
    fn effect_reorder_preserves_ids() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(
            command_id::FILTER_ADD_EFFECT,
            CommandArgs::FilterEffect {
                kind: "gaussian".into(),
            },
        )
        .expect("blur1");
        s.invoke(
            command_id::FILTER_ADD_EFFECT,
            CommandArgs::FilterEffect {
                kind: "sharpen".into(),
            },
        )
        .expect("sharpen");
        s.invoke(
            command_id::FILTER_ADD_EFFECT,
            CommandArgs::FilterEffect {
                kind: "motion".into(),
            },
        )
        .expect("motion");
        let id = s.graph.as_ref().unwrap().active_id().unwrap();
        let effects = s.graph.as_ref().unwrap().get(id).unwrap().effects.clone();
        assert_eq!(effects.len(), 3);
        let mid = effects[1].id;
        s.invoke(
            command_id::EFFECT_REORDER,
            CommandArgs::EffectReorder {
                effect_id: mid,
                to_index: 0,
            },
        )
        .expect("reorder");
        let next = &s.graph.as_ref().unwrap().get(id).unwrap().effects;
        assert_eq!(next[0].id, mid);
        assert_eq!(next.len(), 3);
    }

    #[test]
    fn create_fill_layer_via_command() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let n = s.layer_count();
        s.invoke(
            command_id::LAYER_CREATE_FILL,
            CommandArgs::FillCreate {
                color_rgba: [1.0, 0.0, 0.0, 1.0],
            },
        )
        .expect("fill");
        assert_eq!(s.layer_count(), n + 1);
        let id = s.graph.as_ref().unwrap().active_id().unwrap();
        let layer = s.graph.as_ref().unwrap().get(id).unwrap();
        assert_eq!(layer.kind, LayerKind::Fill);
        assert_eq!(layer.fill.as_ref().unwrap().color_rgba[0], 1.0);
        s.invoke(
            command_id::LAYER_SET_FILL_COLOR,
            CommandArgs::FillColor {
                color_rgba: [0.0, 1.0, 0.0, 1.0],
            },
        )
        .expect("recolor");
        let layer = s.graph.as_ref().unwrap().get(id).unwrap();
        assert_eq!(layer.fill.as_ref().unwrap().color_rgba[1], 1.0);
    }

    #[test]
    fn multi_reorder_atomic() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("a");
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("b");
        let ids: Vec<_> = s.graph.as_ref().unwrap().stack_order();
        assert!(ids.len() >= 3);
        // Select bottom two; move above the remaining top layer.
        s.set_object_selection(vec![ids[0], ids[1]]);
        s.invoke(
            command_id::LAYER_REORDER,
            CommandArgs::Reorder { to_index: 1 },
        )
        .expect("reorder");
        let next = s.graph.as_ref().unwrap().stack_order();
        assert_eq!(next.len(), ids.len());
        assert_eq!(next[1], ids[0]);
        assert_eq!(next[2], ids[1]);
    }

    #[test]
    fn unknown_command_errors() {
        let mut s = SessionState::default();
        let err = s
            .invoke("not.a.command", CommandArgs::None)
            .expect_err("unknown");
        assert!(matches!(err, CommandError::Unknown(_)));
    }

    #[test]
    fn convert_profile_requests_host_rewrite() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let effects = s
            .invoke(
                command_id::DOCUMENT_CONVERT_PROFILE,
                CommandArgs::ConvertProfile {
                    profile: "Display-P3".into(),
                },
            )
            .expect("convert");
        assert!(matches!(
            effects.host_follow_up,
            HostFollowUp::ConvertPixels { .. }
        ));
    }

    #[test]
    fn document_set_icc_embeds_and_clears() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let icc = crate::minimal_icc_fixture();
        s.invoke(
            command_id::DOCUMENT_SET_ICC,
            CommandArgs::SetIcc {
                bytes: Some(icc.clone()),
            },
        )
        .expect("embed");
        assert_eq!(
            s.graph.as_ref().unwrap().color.embedded_icc.as_ref(),
            Some(&icc)
        );
        let err = s
            .invoke(
                command_id::DOCUMENT_SET_ICC,
                CommandArgs::SetIcc {
                    bytes: Some(vec![1, 2, 3]),
                },
            )
            .expect_err("bad");
        assert!(matches!(err, CommandError::Rejected(_)));
        s.invoke(
            command_id::DOCUMENT_SET_ICC,
            CommandArgs::SetIcc { bytes: None },
        )
        .expect("clear");
        assert!(s.graph.as_ref().unwrap().color.embedded_icc.is_none());
    }

    #[test]
    fn filter_preview_does_not_dirty_until_commit() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.mark_persisted(s.document_generation());
        assert!(!s.is_dirty_vs_persisted());
        let generation_before = s.document_generation();
        let effects = s
            .invoke(
                command_id::FILTER_PREVIEW,
                CommandArgs::FilterPreview {
                    kind: "gaussian".into(),
                },
            )
            .expect("preview");
        assert!(!effects.dirty);
        assert_eq!(s.document_generation(), generation_before);
        assert!(!s.is_dirty_vs_persisted());
        let id = s.graph.as_ref().unwrap().active_id().unwrap();
        assert!(
            s.graph
                .as_ref()
                .unwrap()
                .get(id)
                .unwrap()
                .effects
                .is_empty()
        );
        s.invoke(
            command_id::FILTER_SET_PREVIEW_PARAMS,
            CommandArgs::FilterPreviewParams {
                p0: 6.0,
                p1: 0.0,
                p2: 0.0,
            },
        )
        .expect("params");
        assert!(!s.is_dirty_vs_persisted());
        s.invoke(command_id::FILTER_COMMIT, CommandArgs::None)
            .expect("commit");
        assert!(s.is_dirty_vs_persisted());
        let layer = s.graph.as_ref().unwrap().get(id).unwrap();
        assert_eq!(layer.effects.len(), 1);
        assert_eq!(layer.filter_plan.nodes.len(), 1);
        assert_eq!(layer.filter_plan.nodes[0].kind, "gaussian");
    }

    #[test]
    fn filter_cancel_clears_preview_without_effects() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.mark_persisted(s.document_generation());
        s.invoke(
            command_id::FILTER_PREVIEW,
            CommandArgs::FilterPreview {
                kind: "sharpen".into(),
            },
        )
        .expect("preview");
        s.invoke(command_id::FILTER_CANCEL_PREVIEW, CommandArgs::None)
            .expect("cancel");
        assert!(s.filter_preview.is_none());
        assert!(!s.is_dirty_vs_persisted());
        let id = s.graph.as_ref().unwrap().active_id().unwrap();
        assert!(
            s.graph
                .as_ref()
                .unwrap()
                .get(id)
                .unwrap()
                .effects
                .is_empty()
        );
    }

    #[test]
    fn filter_commit_rejects_stale_generation() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(
            command_id::FILTER_PREVIEW,
            CommandArgs::FilterPreview {
                kind: "motion".into(),
            },
        )
        .expect("preview");
        // Advance authority under the preview.
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("mutate");
        let err = s
            .invoke(command_id::FILTER_COMMIT, CommandArgs::None)
            .expect_err("stale");
        assert!(matches!(err, CommandError::Rejected(msg) if msg.contains("stale")));
        assert!(s.filter_preview.is_none());
    }

    #[test]
    fn filter_commit_rejects_cancelled_token() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        s.invoke(
            command_id::FILTER_PREVIEW,
            CommandArgs::FilterPreview {
                kind: "emboss".into(),
            },
        )
        .expect("preview");
        s.filter_preview.as_ref().expect("session").cancel.cancel();
        let err = s
            .invoke(command_id::FILTER_COMMIT, CommandArgs::None)
            .expect_err("cancelled");
        assert!(matches!(err, CommandError::Rejected(msg) if msg.contains("cancelled")));
    }

    #[test]
    fn path_edit_round_trip_on_shape() {
        let mut s = SessionState::default();
        s.apply_preset(SizePreset::P720);
        let path = crate::paths::rect_path("R", 10.0, 10.0, 40.0, 30.0);
        s.invoke(
            command_id::SHAPE_CREATE,
            CommandArgs::ShapeCreate {
                content: ShapeContent {
                    path,
                    ..ShapeContent::default()
                },
            },
        )
        .expect("shape");
        s.invoke(
            command_id::PATH_MOVE_ANCHOR,
            CommandArgs::PathMoveAnchor {
                index: 0,
                x: 5.0,
                y: 7.0,
            },
        )
        .expect("move");
        s.invoke(
            command_id::PATH_ADD_ANCHOR,
            CommandArgs::PathAddAnchor {
                x: 20.0,
                y: 20.0,
                index: Some(1),
            },
        )
        .expect("add");
        s.invoke(
            command_id::PATH_SET_CLOSED,
            CommandArgs::PathSetClosed { closed: false },
        )
        .expect("open");
        let id = s.graph.as_ref().unwrap().active_id().unwrap();
        let shape = s
            .graph
            .as_ref()
            .unwrap()
            .get(id)
            .unwrap()
            .shape
            .as_ref()
            .unwrap();
        assert!(!shape.path.closed);
        assert_eq!(shape.path.anchors.len(), 5);
        assert!((shape.path.anchors[0].x - 5.0).abs() < f32::EPSILON);
        s.invoke(
            command_id::PATH_DELETE_ANCHOR,
            CommandArgs::PathDeleteAnchor { index: 1 },
        )
        .expect("delete");
        assert_eq!(
            s.graph
                .as_ref()
                .unwrap()
                .get(id)
                .unwrap()
                .shape
                .as_ref()
                .unwrap()
                .path
                .anchors
                .len(),
            4
        );
        s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo delete");
        s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo open");
        s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo add");
        s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo move");
        let restored = &s
            .graph
            .as_ref()
            .unwrap()
            .get(id)
            .unwrap()
            .shape
            .as_ref()
            .unwrap()
            .path;
        assert!(restored.closed);
        assert_eq!(restored.anchors.len(), 4);
        assert!((restored.anchors[0].x - 10.0).abs() < f32::EPSILON);
    }
}
