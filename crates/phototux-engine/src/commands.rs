//! Named document/session command spine (handbook 08, DR-003).
//!
//! Paint-worker dab traffic stays in [`crate::command::EngineCommand`].
//! User-visible semantic mutations enter here via [`SessionState::invoke`].
//!
//! GPU pixel ops run in the host; many commands are **GPU-then-commit**
//! (host applies canvas work, then invoke updates graph/selection/history).

use crate::document::MAX_LAYERS;
use crate::error::DocumentError;
use crate::history::HistoryKind;
use crate::layer::{
    AdjustmentParams, BlendMode, FillContent, LayerId, LayerKind, LayerMask, LayerTransform,
    PaintTarget, TextContent,
};
use crate::layer_style::LayerStyle;
use crate::selection::{SelectionModifyOp, SelectionShape};
use crate::undo::actions as undo_actions;
use crate::{SessionState, tool_id};

/// Stable command identifiers (vendor-neutral; see Command Taxonomy).
mod vocabulary;

pub use vocabulary::*;

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
        let generation = {
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
            graph.generation
        };
        self.sync_object_selection_to_active();
        Ok(CommandEffects::document_edit(generation))
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
        reject_locked_layers(graph, &targets)?;

        let mut batch = break_clips_whose_base_deleted(graph, &targets);
        let broke = batch.len();
        let prev_active = graph.active_id();
        let deletes = delete_layers_recording(graph, &targets, prev_active, &batch)?;
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
        reject_group_targets(graph, &targets)?;
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
        let parent_cmds = reparent_into_group(graph, &targets, group_id)?;

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
        // Feather is the one op the document itself remembers: it is a
        // property of the selection channel, not a one-off edit of the mask.
        if op == SelectionModifyOp::Feather {
            self.selection.feather = radius as f32;
        }
        let generation = self.bump_document_generation();
        self.history
            .push_selection(format!("Selection {}", op.as_str()), generation);
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
            contrast,
            shift,
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
        let mut next = crate::LayerMask {
            enabled,
            linked,
            density: density.clamp(0.0, 1.0),
            feather: feather.max(0.0),
            inverted,
            contrast,
            shift,
        };
        next.clamp_refine();
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
        let next = Some(crate::VectorMask::default());
        layer.vector_mask = next.clone();
        // Was `push_transform`, which `HistoryService::undo_next` hands to the
        // host as a no-op: undo consumed a step and left the mask in the graph.
        let effects = self.record_graph_edit(
            crate::GraphCommand::SetVectorMask {
                id,
                prev: None,
                next,
            },
            "Add vector mask",
        )?;
        self.announce("Vector mask added (path edit deferred)");
        Ok(effects)
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
        let id = graph.add_shape_top(None, *content)?;
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
        let Some(params) = AdjustmentParams::default_for_kind(&kind) else {
            return Err(CommandError::InvalidArgument("unknown adjustment kind"));
        };
        let name = params.label();
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
        let CommandArgs::FilterParameters { slots } = args else {
            return Err(CommandError::InvalidArgument("expected filter params"));
        };
        let id = self.active_layer_id()?;
        let prev = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .and_then(|l| l.adjustment.clone())
            .ok_or(CommandError::Rejected("no adjustment"))?;
        let next = prev.with_slots(slots).clamped();
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
        // Declared `UndoPolicy::Mergeable`: dragging an adjustment slider is one
        // gesture, not one history entry per step.
        history.push_graph_mergeable(
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
            "noise" => self.graph.as_mut().and_then(|g| g.add_noise(id, 0.35)),
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
            "noise" => "Noise",
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
        // Declared `UndoPolicy::Mergeable`: a radius drag is one gesture.
        self.history.push_graph_mergeable(
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

    /// Bump the generation, record the edit in history, and produce its effects.
    ///
    /// The tail of every document mutation. Splitting it across call sites is
    /// how four style commands and `mask.create-vector` ended up bumping the
    /// generation without recording anything — declared `UndoPolicy::Transaction`
    /// in `command_meta`, undoable nowhere.
    fn record_graph_edit(
        &mut self,
        command: crate::GraphCommand,
        label: &'static str,
    ) -> Result<CommandEffects, CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        graph.bump_generation();
        let generation = graph.generation;
        self.history.push_graph_applied(command, label, generation);
        Ok(CommandEffects::document_edit(generation))
    }

    /// Append a layer effect style, recording prev/next so undo can restore it.
    fn add_layer_style(
        &mut self,
        style: LayerStyle,
        requires_raster: &'static str,
        label: &'static str,
    ) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected(requires_raster));
        }
        let prev = layer.styles.clone();
        let mut next = prev.clone();
        next.push(style);
        layer.styles = next.clone();
        self.record_graph_edit(crate::GraphCommand::SetStyles { id, prev, next }, label)
    }

    fn cmd_style_add_drop_shadow(&mut self) -> Result<CommandEffects, CommandError> {
        self.add_layer_style(
            LayerStyle::drop_shadow_default(),
            "drop shadow requires raster",
            "Add drop shadow",
        )
    }

    fn cmd_style_add_stroke(&mut self) -> Result<CommandEffects, CommandError> {
        self.add_layer_style(
            LayerStyle::stroke_default(),
            "stroke style requires raster",
            "Add stroke style",
        )
    }

    fn cmd_style_add_outer_glow(&mut self) -> Result<CommandEffects, CommandError> {
        self.add_layer_style(
            LayerStyle::outer_glow_default(),
            "outer glow requires raster",
            "Add outer glow",
        )
    }

    fn cmd_style_add_color_overlay(&mut self) -> Result<CommandEffects, CommandError> {
        self.add_layer_style(
            LayerStyle::color_overlay_default(),
            "color overlay requires raster",
            "Add color overlay",
        )
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

fn reject_locked_layers(
    graph: &crate::DocumentGraph,
    targets: &[LayerId],
) -> Result<(), CommandError> {
    for id in targets {
        let Some(layer) = graph.get(*id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.locks.all {
            return Err(CommandError::Rejected("layer is locked"));
        }
    }
    Ok(())
}

fn break_clips_whose_base_deleted(
    graph: &mut crate::DocumentGraph,
    targets: &[LayerId],
) -> Vec<crate::GraphCommand> {
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
        if old_base.is_some_and(|b| targets.contains(b))
            && let Some(true) = graph.set_clips_to_below(*id, false)
        {
            batch.push(crate::GraphCommand::SetClipsToBelow {
                id: *id,
                prev: true,
                next: false,
            });
        }
    }
    batch
}

fn rollback_clip_breaks(graph: &mut crate::DocumentGraph, batch: &[crate::GraphCommand]) {
    for cmd in batch.iter().rev() {
        if let crate::GraphCommand::SetClipsToBelow { id, prev, .. } = cmd {
            let _ = graph.set_clips_to_below(*id, *prev);
        }
    }
}

fn delete_layers_recording(
    graph: &mut crate::DocumentGraph,
    targets: &[LayerId],
    prev_active: Option<LayerId>,
    clip_batch: &[crate::GraphCommand],
) -> Result<Vec<crate::GraphCommand>, CommandError> {
    let mut deletes = Vec::with_capacity(targets.len());
    for id in targets.iter().rev() {
        let Some((index, layer)) = graph.remove_layer(*id) else {
            for cmd in deletes.iter().rev() {
                if let crate::GraphCommand::DeleteLayer { index, layer, .. } = cmd {
                    graph.insert_layer_at(*index, layer.clone());
                }
            }
            rollback_clip_breaks(graph, clip_batch);
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
    Ok(deletes)
}

fn reject_group_targets(
    graph: &crate::DocumentGraph,
    targets: &[LayerId],
) -> Result<(), CommandError> {
    for id in targets {
        let Some(layer) = graph.get(*id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.locks.all {
            return Err(CommandError::Rejected("layer is locked"));
        }
        if layer.kind == LayerKind::Group {
            reject_grouping_group_with_children(graph, *id, targets)?;
        }
    }
    Ok(())
}

fn reject_grouping_group_with_children(
    graph: &crate::DocumentGraph,
    group_id: LayerId,
    targets: &[LayerId],
) -> Result<(), CommandError> {
    for other in targets {
        if *other != group_id
            && graph
                .get(*other)
                .is_some_and(|l| l.parent == Some(group_id))
        {
            return Err(CommandError::Rejected(
                "cannot group a group with its children",
            ));
        }
    }
    Ok(())
}

fn rollback_parent_cmds(graph: &mut crate::DocumentGraph, cmds: &[crate::GraphCommand]) {
    for cmd in cmds.iter().rev() {
        if let crate::GraphCommand::SetParent { id, prev, .. } = cmd {
            let _ = graph.set_parent(*id, *prev);
        }
    }
}

fn reparent_into_group(
    graph: &mut crate::DocumentGraph,
    targets: &[LayerId],
    group_id: LayerId,
) -> Result<Vec<crate::GraphCommand>, CommandError> {
    let mut parent_cmds = Vec::new();
    for id in targets {
        let Some(prev) = graph.set_parent(*id, Some(group_id)) else {
            rollback_parent_cmds(graph, &parent_cmds);
            let _ = graph.remove_layer(group_id);
            return Err(CommandError::Rejected("set parent failed"));
        };
        parent_cmds.push(crate::GraphCommand::SetParent {
            id: *id,
            prev,
            next: Some(group_id),
        });
    }
    Ok(parent_cmds)
}

#[cfg(test)]
mod tests;
