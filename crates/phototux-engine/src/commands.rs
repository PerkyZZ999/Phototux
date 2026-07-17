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
    AdjustmentParams, BlendMode, LayerId, LayerKind, LayerMask, LayerTransform, PaintTarget,
    ShapeContent, TextContent,
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
    pub const LAYER_DELETE: &str = "layer.delete";
    pub const LAYER_SET_ACTIVE: &str = "layer.set-active";
    pub const LAYER_SET_VISIBILITY: &str = "layer.set-visibility";
    pub const LAYER_SET_OPACITY: &str = "layer.set-opacity";
    pub const LAYER_SET_BLEND: &str = "layer.set-blend";
    pub const LAYER_REORDER: &str = "layer.reorder";
    pub const LAYER_GROUP: &str = "layer.group";
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

    pub const TEXT_CREATE: &str = "text.create";
    pub const TEXT_SET_CONTENT: &str = "text.set-content";
    pub const TEXT_BAKE: &str = "text.bake";

    pub const SHAPE_CREATE: &str = "shape.create";
    pub const SHAPE_RASTERIZE: &str = "shape.rasterize";

    pub const FILTER_ADD_ADJUSTMENT: &str = "filter.add-adjustment";
    pub const FILTER_SET_PARAMETERS: &str = "filter.set-parameters";
    pub const FILTER_ADD_EFFECT: &str = "filter.add-effect";
    pub const FILTER_SET_GAUSSIAN_RADIUS: &str = "filter.set-gaussian-radius";

    pub const STYLE_ADD_DROP_SHADOW: &str = "style.add-drop-shadow";
    pub const STYLE_ADD_STROKE: &str = "style.add-stroke";

    pub const CLIPBOARD_PASTE_LAYER: &str = "clipboard.paste-layer";
    pub const PATH_STROKE_TO_LAYER: &str = "path.stroke-to-layer";

    pub const RASTER_TRANSFORM_COMMIT: &str = "raster.transform-commit";
    pub const RASTER_FLIP: &str = "raster.flip";
    pub const RASTER_FILL: &str = "raster.fill";
    pub const RASTER_GRADIENT: &str = "raster.gradient";
    pub const RASTER_PAINT_STROKE: &str = "raster.paint-stroke";

    /// Application chrome — host opens preferences dialog.
    pub const APP_SHOW_PREFERENCES: &str = "app.show-preferences";
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
        LAYER_DELETE,
        LAYER_SET_ACTIVE,
        LAYER_SET_VISIBILITY,
        LAYER_SET_OPACITY,
        LAYER_SET_BLEND,
        LAYER_REORDER,
        LAYER_GROUP,
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
        TEXT_CREATE,
        TEXT_SET_CONTENT,
        TEXT_BAKE,
        SHAPE_CREATE,
        SHAPE_RASTERIZE,
        FILTER_ADD_ADJUSTMENT,
        FILTER_SET_PARAMETERS,
        FILTER_ADD_EFFECT,
        FILTER_SET_GAUSSIAN_RADIUS,
        STYLE_ADD_DROP_SHADOW,
        STYLE_ADD_STROKE,
        CLIPBOARD_PASTE_LAYER,
        PATH_STROKE_TO_LAYER,
        RASTER_TRANSFORM_COMMIT,
        RASTER_FLIP,
        RASTER_FILL,
        RASTER_GRADIENT,
        RASTER_PAINT_STROKE,
        APP_SHOW_PREFERENCES,
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
    /// Undo `steps` times to jump the history timeline (host applies stroke/selection stacks).
    HistoryJump {
        steps: u32,
    },
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
            command_id::LAYER_DELETE => self.cmd_layer_delete(),
            command_id::LAYER_SET_ACTIVE => self.cmd_layer_set_active(args),
            command_id::LAYER_SET_VISIBILITY => self.cmd_layer_set_visibility(args),
            command_id::LAYER_SET_OPACITY => self.cmd_layer_set_opacity(args),
            command_id::LAYER_SET_BLEND => self.cmd_layer_set_blend(args),
            command_id::LAYER_REORDER => self.cmd_layer_reorder(args),
            command_id::LAYER_GROUP => self.cmd_layer_group(),
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
            command_id::TEXT_CREATE => self.cmd_text_create(args),
            command_id::TEXT_SET_CONTENT => self.cmd_text_set_content(args),
            command_id::TEXT_BAKE => self.cmd_text_bake(),
            command_id::SHAPE_CREATE => self.cmd_shape_create(args),
            command_id::SHAPE_RASTERIZE => self.cmd_shape_rasterize(),
            command_id::FILTER_ADD_ADJUSTMENT => self.cmd_filter_add_adjustment(args),
            command_id::FILTER_SET_PARAMETERS => self.cmd_filter_set_parameters(args),
            command_id::FILTER_ADD_EFFECT => self.cmd_filter_add_effect(args),
            command_id::FILTER_SET_GAUSSIAN_RADIUS => self.cmd_filter_set_gaussian_radius(args),
            command_id::STYLE_ADD_DROP_SHADOW => self.cmd_style_add_drop_shadow(),
            command_id::STYLE_ADD_STROKE => self.cmd_style_add_stroke(),
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

    fn cmd_layer_delete(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !undo_actions::delete_layer(graph, history, id) {
            return Err(CommandError::Rejected("delete layer failed"));
        }
        Ok(CommandEffects::document_edit(graph.generation))
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
        let name = self.object_selection_names_joined();
        self.announce(format!("Object selection: {name}"));
        Ok(CommandEffects {
            recomposite: false,
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
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !undo_actions::move_layer(graph, history, id, to_index.max(0) as usize) {
            return Err(CommandError::Rejected("reorder failed"));
        }
        Ok(CommandEffects::document_edit(graph.generation))
    }

    fn cmd_layer_group(&mut self) -> Result<CommandEffects, CommandError> {
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = graph.add_group_top(None)?;
        let index = graph.index_of(id).unwrap_or(0);
        let layer = graph
            .get(id)
            .cloned()
            .ok_or(CommandError::Document(DocumentError::LayerMissingAfterAdd))?;
        history.push_graph_applied(
            crate::GraphCommand::AddLayer { id, index, layer },
            "Add group",
            {
                graph.bump_generation();
                graph.generation
            },
        );
        Ok(CommandEffects::document_edit(graph.generation))
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
        self.set_active_tool(&tool);
        let mut e = CommandEffects::view_only();
        e.sync_layers = false;
        e.generation = self.document_generation();
        Ok(e)
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
}
