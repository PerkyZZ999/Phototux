//! Named document/session command spine (handbook 08, DR-003).
//!
//! Paint-worker dab traffic stays in [`crate::command::EngineCommand`].
//! User-visible semantic mutations enter here via [`SessionState::invoke`].

use thiserror::Error;

use crate::document::MAX_LAYERS;
use crate::error::DocumentError;
use crate::history::HistoryKind;
use crate::layer::BlendMode;
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

    pub const VIEW_ZOOM_TO: &str = "view.zoom-to";
    pub const VIEW_ZOOM_TO_FIT: &str = "view.zoom-to-fit";
    pub const VIEW_PAN_TO: &str = "view.pan-to";
    pub const VIEW_SET_TOOL: &str = "view.set-tool";

    pub const DOCUMENT_NEW_PRESET: &str = "document.new-preset";
    pub const DOCUMENT_NEW_SIZE: &str = "document.new-size";
    pub const DOCUMENT_ASSIGN_PROFILE: &str = "document.assign-profile";
    pub const DOCUMENT_CONVERT_PROFILE: &str = "document.convert-profile";

    /// Built-in commands registered for discovery / headless tests.
    pub const ALL: &[&str] = &[
        HISTORY_UNDO,
        HISTORY_REDO,
        LAYER_CREATE,
        LAYER_DELETE,
        LAYER_SET_ACTIVE,
        LAYER_SET_VISIBILITY,
        LAYER_SET_OPACITY,
        LAYER_SET_BLEND,
        LAYER_REORDER,
        VIEW_ZOOM_TO,
        VIEW_ZOOM_TO_FIT,
        VIEW_PAN_TO,
        VIEW_SET_TOOL,
        DOCUMENT_NEW_PRESET,
        DOCUMENT_NEW_SIZE,
        DOCUMENT_ASSIGN_PROFILE,
        DOCUMENT_CONVERT_PROFILE,
    ];
}

/// Parameters for [`SessionState::invoke`].
#[derive(Debug, Clone)]
pub enum CommandArgs {
    None,
    LayerIndex(i32),
    SetVisibility { index: i32, visible: bool },
    SetOpacity { opacity: f32 },
    SetBlend { blend: String },
    Reorder { to_index: i32 },
    Zoom { zoom: f32 },
    Pan { world_x: f32, world_y: f32 },
    Tool { tool: String },
    NewPreset { label: String },
    NewSize { width: u32, height: u32 },
    AssignProfile { profile: String },
    ConvertProfile { profile: String },
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
    pub host_history: Option<HostHistoryAction>,
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
            host_history: None,
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
            host_history: None,
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
            command_id::LAYER_CREATE => self.cmd_layer_create(),
            command_id::LAYER_DELETE => self.cmd_layer_delete(),
            command_id::LAYER_SET_ACTIVE => self.cmd_layer_set_active(args),
            command_id::LAYER_SET_VISIBILITY => self.cmd_layer_set_visibility(args),
            command_id::LAYER_SET_OPACITY => self.cmd_layer_set_opacity(args),
            command_id::LAYER_SET_BLEND => self.cmd_layer_set_blend(args),
            command_id::LAYER_REORDER => self.cmd_layer_reorder(args),
            command_id::VIEW_ZOOM_TO => self.cmd_view_zoom(args),
            command_id::VIEW_ZOOM_TO_FIT => {
                self.zoom_to_fit();
                let mut e = CommandEffects::view_only();
                e.generation = self.document_generation();
                Ok(e)
            }
            command_id::VIEW_PAN_TO => self.cmd_view_pan(args),
            command_id::VIEW_SET_TOOL => self.cmd_view_set_tool(args),
            command_id::DOCUMENT_NEW_PRESET => self.cmd_document_new_preset(args),
            command_id::DOCUMENT_NEW_SIZE => self.cmd_document_new_size(args),
            command_id::DOCUMENT_ASSIGN_PROFILE => self.cmd_document_assign_profile(args),
            command_id::DOCUMENT_CONVERT_PROFILE => self.cmd_document_convert_profile(args),
            other => Err(CommandError::Unknown(other.to_owned())),
        }
    }

    pub fn document_generation(&self) -> u64 {
        self.graph.as_ref().map(|g| g.generation).unwrap_or(0)
    }

    fn bump_generation(&mut self) {
        if let Some(graph) = self.graph.as_mut() {
            graph.bump_generation();
        }
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
        let id = self
            .graph
            .as_ref()
            .and_then(|g| g.active_id())
            .ok_or(CommandError::Rejected("no active layer"))?;
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
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !graph.set_active_index(index.max(0) as usize) {
            return Err(CommandError::Rejected("invalid layer index"));
        }
        // Active switch is presentation selection — no dirty/history (matches prior UI).
        Ok(CommandEffects {
            recomposite: false,
            dirty: false,
            sync_layers: true,
            sync_camera: false,
            sync_doc: false,
            host_history: None,
            generation: graph.generation,
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
        let id = self
            .graph
            .as_ref()
            .and_then(|g| g.active_id())
            .ok_or(CommandError::Rejected("no active layer"))?;
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
        let id = self
            .graph
            .as_ref()
            .and_then(|g| g.active_id())
            .ok_or(CommandError::Rejected("no active layer"))?;
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
        let id = self
            .graph
            .as_ref()
            .and_then(|g| g.active_id())
            .ok_or(CommandError::Rejected("no active layer"))?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !undo_actions::move_layer(graph, history, id, to_index.max(0) as usize) {
            return Err(CommandError::Rejected("reorder failed"));
        }
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

    fn cmd_view_set_tool(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::Tool { tool } = args else {
            return Err(CommandError::InvalidArgument("expected tool"));
        };
        if tool.is_empty() {
            return Err(CommandError::InvalidArgument("empty tool id"));
        }
        // Accept known tool ids; allow forward-compatible custom strings.
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
            host_history: None,
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
            host_history: None,
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
            host_history: None,
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
        let plan = graph.color.begin_convert(profile);
        if !plan.rewrite_pixels {
            graph.color.mark_converted();
        }
        graph.bump_generation();
        Ok(CommandEffects {
            recomposite: plan.rewrite_pixels,
            dirty: true,
            sync_layers: false,
            sync_camera: false,
            sync_doc: true,
            host_history: None,
            generation: graph.generation,
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
    fn unknown_command_errors() {
        let mut s = SessionState::default();
        let err = s
            .invoke("not.a.command", CommandArgs::None)
            .expect_err("unknown");
        assert!(matches!(err, CommandError::Unknown(_)));
    }
}
