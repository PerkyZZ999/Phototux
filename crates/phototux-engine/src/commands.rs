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
    AdjustmentParams, BlendMode, FillContent, FilterParams, LayerId, LayerKind, LayerMask,
    LayerTransform, PaintTarget, TextContent,
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
            command_id::LAYER_DUPLICATE => self.cmd_layer_duplicate(),
            command_id::LAYER_FLATTEN => self.cmd_layer_flatten(),
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
            command_id::LAYER_ALIGN => self.cmd_layer_align(args),
            command_id::LAYER_SET_BLEND_IF => self.cmd_layer_set_blend_if(args),
            command_id::VIEW_ZOOM_TO => self.cmd_view_zoom(args),
            command_id::VIEW_ZOOM_TO_FIT => {
                self.zoom_to_fit();
                Ok(self.view_changed())
            }
            command_id::VIEW_ZOOM_IN => {
                self.zoom_step(true);
                Ok(self.view_changed())
            }
            command_id::VIEW_ZOOM_OUT => {
                self.zoom_step(false);
                Ok(self.view_changed())
            }
            command_id::VIEW_ZOOM_ACTUAL => {
                self.set_zoom(1.0);
                Ok(self.view_changed())
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
            command_id::DOCUMENT_ROTATE => self.cmd_document_rotate(args),
            command_id::DOCUMENT_FLIP => self.cmd_document_flip(args),
            command_id::SELECTION_REPLACE => self.cmd_selection_replace(args),
            command_id::SELECTION_DESELECT => self.cmd_selection_deselect(),
            command_id::SELECTION_INVERT => self.cmd_selection_invert(),
            command_id::SELECTION_SELECT_ALL => self.cmd_selection_select_all(),
            command_id::SELECTION_MODIFY => self.cmd_selection_modify(args),
            command_id::SELECTION_COLOR_SELECT => self.cmd_selection_color_select(args),
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
            command_id::SHAPE_SET_APPEARANCE => self.cmd_shape_set_appearance(args),
            command_id::SMART_CREATE => self.cmd_smart_create(args),
            command_id::SMART_SET_PLACEMENT => self.cmd_smart_set_placement(args),
            command_id::SMART_RASTERIZE => self.cmd_smart_rasterize(),
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
            command_id::STYLE_ADD => self.cmd_style_add(args),
            command_id::STYLE_SET_PARAMS
            | command_id::STYLE_SET_COLOR
            | command_id::STYLE_SET_ENABLED
            | command_id::STYLE_REMOVE => self.cmd_style_edit(args),
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
            .ok_or(CommandError::Rejected("select a layer first"))
    }

    fn assert_active_paintable(&self) -> Result<LayerId, CommandError> {
        let id = self.active_layer_id()?;
        let layer = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .ok_or(CommandError::Rejected("select a layer first"))?;
        if layer.paint_blocked() {
            return Err(CommandError::Rejected(
                "this layer's pixels are locked — unlock it to paint on it",
            ));
        }
        Ok(id)
    }

    fn assert_active_movable(&self) -> Result<LayerId, CommandError> {
        let id = self.active_layer_id()?;
        let layer = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .ok_or(CommandError::Rejected("select a layer first"))?;
        if layer.position_blocked() {
            return Err(CommandError::Rejected(
                "this layer's position is locked — unlock it to move it",
            ));
        }
        Ok(id)
    }

    fn cmd_history_undo(&mut self) -> Result<CommandEffects, CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(kind) = self.history.undo_next(graph) else {
            return Err(CommandError::Rejected("there is nothing to undo"));
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
        // Either side of the present is a legal target: the panel lists the
        // undone steps as well, and clicking one of those means redo.
        let (steps, forward) = match self.history.undo_steps_to_entry(entry_id) {
            Some(steps) => (steps, false),
            None => (
                self.history
                    .redo_steps_to_entry(entry_id)
                    .ok_or(CommandError::Rejected("history entry not found"))?,
                true,
            ),
        };
        if steps == 0 {
            return Err(CommandError::Rejected("already at history entry"));
        }
        let sign = if forward { '+' } else { '−' };
        self.announce(format!("Jump history ({sign}{steps})"));
        Ok(CommandEffects::host_chrome(HostFollowUp::HistoryJump {
            steps: u32::try_from(steps).unwrap_or(u32::MAX),
            forward,
        }))
    }

    fn cmd_history_redo(&mut self) -> Result<CommandEffects, CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(kind) = self.history.redo_next(graph) else {
            return Err(CommandError::Rejected("there is nothing to redo"));
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

    fn cmd_layer_duplicate(&mut self) -> Result<CommandEffects, CommandError> {
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
            let Some(active) = graph.active_id() else {
                return Err(CommandError::Rejected("no active layer"));
            };
            undo_actions::duplicate_layer(graph, history, active)?;
            graph.generation
        };
        self.sync_object_selection_to_active();
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_layer_flatten(&mut self) -> Result<CommandEffects, CommandError> {
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if graph.layer_count() == 0 {
            return Err(CommandError::Rejected("there is nothing to flatten"));
        }
        graph.flatten_to_single_layer("Background");
        graph.bump_generation();
        let generation = graph.generation;
        // A transform entry rather than a graph one: the pixels change as
        // well as the stack, and only the host's document snapshot can put
        // both back in a single undo.
        self.history.push_transform("Flatten Image", generation);
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
            return Err(CommandError::Rejected(
                "the active layer is not a fill layer",
            ));
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
                None => return Err(CommandError::Rejected("select a layer first")),
            }
        } else {
            self.selected_layer_ids.clone()
        };
        ids.retain(|id| graph.get(*id).is_some());
        if ids.is_empty() {
            return Err(CommandError::Rejected("select a layer first"));
        }
        // Stable stack order (bottom → top).
        ids.sort_by_key(|id| graph.index_of(*id).unwrap_or(usize::MAX));
        ids.dedup();
        Ok(ids)
    }

    /// Replace the active layer's blend ranges.
    ///
    /// Declared mergeable and recorded through the coalescing push, so
    /// dragging a slider handle is one history entry rather than one per step
    /// — the same treatment opacity and the layer-style parameters get.
    fn cmd_layer_set_blend_if(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SetBlendIf { blend_if } = args else {
            return Err(CommandError::InvalidArgument("expected blend ranges"));
        };
        let id = self.active_layer_id()?;
        let SessionState { graph, history, .. } = self;
        let Some(graph) = graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        // Stops are sorted on the way in, so a panel that lets the white pair
        // cross the black pair cannot store a range that inverts itself.
        let next = crate::BlendIf {
            channel: blend_if.channel,
            this_layer: blend_if.this_layer.normalized(),
            underlying: blend_if.underlying.normalized(),
        };
        let Some(prev) = graph.set_blend_if(id, next) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if prev == next {
            return Err(CommandError::Rejected("those blend ranges are already set"));
        }
        graph.bump_generation();
        let generation = graph.generation;
        history.push_graph_mergeable(
            crate::GraphCommand::SetBlendIf { id, prev, next },
            "Blend If",
            generation,
        );
        Ok(CommandEffects::document_edit(generation))
    }

    /// Align or distribute layers using boxes the host measured.
    ///
    /// The move is written into each layer's translation rather than baked
    /// into its pixels: the compositor already honours `layer.transform`, so
    /// aligning stays non-destructive and one undo puts every layer back.
    fn cmd_layer_align(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::AlignLayers { op, targets } = args else {
            return Err(CommandError::InvalidArgument("expected align targets"));
        };
        if targets.len() < op.min_targets() {
            return Err(CommandError::Rejected("select more layers to align them"));
        }
        let canvas = crate::Rect::new(0.0, 0.0, self.size.width as f32, self.size.height as f32);
        let ids: Vec<LayerId> = targets
            .iter()
            .flat_map(|t| t.members.iter().copied())
            .collect();
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        reject_position_locked(graph, &ids)?;

        let rects: Vec<crate::Rect> = targets.iter().map(|t| t.bounds).collect();
        let frame = crate::align_frame(&rects, canvas);
        let batch = align_batch(graph, &targets, &crate::align_offsets(op, &rects, frame));
        if batch.is_empty() {
            return Err(CommandError::Rejected("those layers are already aligned"));
        }
        let moved = batch.len();
        graph.bump_generation();
        let generation = graph.generation;
        self.history
            .push_graph_applied(crate::GraphCommand::Batch(batch), op.label(), generation);
        self.announce(format!("{}: moved {moved} layer(s)", op.label()));
        Ok(CommandEffects::document_edit(generation))
    }

    fn cmd_layer_delete(&mut self) -> Result<CommandEffects, CommandError> {
        let targets = self.structural_target_ids()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if targets.len() >= graph.layer_count() {
            return Err(CommandError::Rejected(
                "a document needs at least one layer",
            ));
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
                return Err(CommandError::Rejected(
                    "this layer's position is locked — unlock it to move it",
                ));
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
            return Err(CommandError::Rejected("there are no layers to reorder"));
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
            return Err(CommandError::Rejected("select a group first"));
        }
        for id in &groups {
            if graph.get(*id).is_some_and(|l| l.locks.all) {
                return Err(CommandError::Rejected(
                    "this group is locked — unlock it to change it",
                ));
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

    /// Effects for a command that moved the camera and nothing else.
    fn view_changed(&self) -> CommandEffects {
        let mut effects = CommandEffects::view_only();
        effects.generation = self.document_generation();
        effects
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

    fn cmd_document_rotate(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::Rotate { quarter_turns } = args else {
            return Err(CommandError::InvalidArgument("expected rotate"));
        };
        let turns = quarter_turns % 4;
        if turns == 0 {
            return Err(CommandError::Rejected("a full turn changes nothing"));
        }
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        // Only an odd number of quarter turns swaps the axes.
        if turns % 2 == 1 {
            let (w, h) = (graph.size.width, graph.size.height);
            graph.size = crate::DocumentSize::new(h, w);
        }
        graph.revision = graph.revision.wrapping_add(1);
        self.size = graph.size;
        self.selection.clear();
        let generation = {
            graph.bump_generation();
            graph.generation
        };
        self.history
            .push_transform(rotation_label(turns), generation);
        self.zoom_to_fit();
        let mut effects = CommandEffects::document_edit(generation);
        effects.sync_selection = true;
        effects.sync_camera = true;
        Ok(effects)
    }

    fn cmd_document_flip(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::RasterFlip { horizontal } = args else {
            return Err(CommandError::InvalidArgument("expected flip"));
        };
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        // Unlike a layer flip this does not ask whether the active layer is
        // movable: the canvas is not a layer, and a locked layer no more
        // exempts itself from a canvas flip than from a canvas rotation.
        let generation = self.bump_document_generation();
        self.history.push_transform(
            if horizontal {
                "Flip Canvas Horizontal"
            } else {
                "Flip Canvas Vertical"
            },
            generation,
        );
        self.selection.clear();
        let mut effects = CommandEffects::document_edit(generation);
        effects.sync_selection = true;
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
                    return Err(CommandError::Rejected("the selection is empty"));
                }
                self.selection.set_rect(rect, combine);
            }
            SelectionShape::Ellipse => {
                if rect.width == 0 || rect.height == 0 {
                    return Err(CommandError::Rejected("the selection is empty"));
                }
                self.selection.set_ellipse(rect, combine);
            }
            SelectionShape::Mask => {
                if polygon.len() < 3 {
                    return Err(CommandError::Rejected(
                        "a polygon needs at least three points",
                    ));
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

    /// Record a colour-based selection made by the host.
    ///
    /// The pixels live on the GPU, so the host reads them, runs
    /// [`crate::color_select_mask`] and writes the coverage; this records the
    /// bounds, the history entry and the generation — the same division as
    /// `raster.fill` and the other host-executed edits.
    fn cmd_selection_color_select(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SelectionColorSelect {
            contiguous,
            tolerance,
            combine,
        } = args
        else {
            return Err(CommandError::InvalidArgument("expected colour select"));
        };
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        if !(0.0..=1.0).contains(&tolerance) {
            return Err(CommandError::InvalidArgument("tolerance outside 0..=1"));
        }
        let _ = self.active_layer_id()?;
        // The coverage is arbitrary, so the mask is the authority on its own
        // bounds; the whole canvas is the honest answer until the host reports
        // otherwise.
        let (w, h) = (self.size.width, self.size.height);
        self.selection.select_all(w, h);
        self.selection.shape = crate::SelectionShape::Mask;
        self.selection.combine = combine;
        let label = if contiguous {
            "Magic wand"
        } else {
            "Color range"
        };
        let generation = self.bump_document_generation();
        self.history.push_selection(label, generation);
        Ok(CommandEffects::selection_edit(generation))
    }

    fn cmd_selection_to_mask(&mut self) -> Result<CommandEffects, CommandError> {
        if !self.has_document {
            return Err(CommandError::Document(DocumentError::NoDocument));
        }
        if !self.selection.active {
            return Err(CommandError::Rejected("make a selection first"));
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
            return Err(CommandError::Rejected(
                "applying a mask needs a raster layer",
            ));
        }
        if layer.paint_blocked() {
            return Err(CommandError::Rejected(
                "this layer is locked — unlock it to change it",
            ));
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
            return Err(CommandError::Rejected("this is not a text layer"));
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

    /// Bake a text layer down to pixels, dropping its editable text.
    ///
    /// Recorded in history, like every other command that discards the only
    /// copy of something. It used to write the kind and the payload straight
    /// onto the graph and push nothing, so baking type was permanent — which
    /// the panel had ended up describing as if it were the design.
    fn cmd_text_bake(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Text {
            return Err(CommandError::Rejected("this is not a text layer"));
        }
        let prev_text = layer.text.clone();
        // A text layer carries no raster asset until it becomes one.
        if layer.asset_key.is_none() {
            layer.asset_key = Some(format!("layer-{}", id.0));
        }
        let command = crate::GraphCommand::Batch(vec![
            crate::GraphCommand::SetKind {
                id,
                prev: LayerKind::Text,
                next: LayerKind::Raster,
            },
            crate::GraphCommand::SetText {
                id,
                prev: prev_text,
                next: None,
            },
        ]);
        if !command.apply(graph) {
            return Err(CommandError::Rejected("layer missing"));
        }
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(command, "Bake text", generation);
        Ok(CommandEffects::document_edit(generation))
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

    /// Bake a shape layer down to pixels, dropping its geometry.
    ///
    /// Recorded in history. It used to write the kind and the payload straight
    /// onto the graph and push nothing, so discarding a shape's editable path
    /// — the only copy of it — could not be taken back. The kind and the
    /// payload move as one entry for the same reason a smart object's do: a
    /// graph with one undone and the other not is a raster layer still
    /// carrying a path, or a shape layer with none.
    fn cmd_shape_rasterize(&mut self) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Shape {
            return Err(CommandError::Rejected("this is not a shape layer"));
        }
        let prev_shape = layer.shape.clone();
        // A shape layer carries no raster asset until it becomes one.
        if layer.asset_key.is_none() {
            layer.asset_key = Some(format!("layer-{}", id.0));
        }
        let command = crate::GraphCommand::Batch(vec![
            crate::GraphCommand::SetKind {
                id,
                prev: LayerKind::Shape,
                next: LayerKind::Raster,
            },
            crate::GraphCommand::SetShape {
                id,
                prev: prev_shape,
                next: None,
            },
        ]);
        if !command.apply(graph) {
            return Err(CommandError::Rejected("layer missing"));
        }
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(command, "Rasterize shape", generation);
        Ok(CommandEffects::document_edit(generation))
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
            return Err(CommandError::Rejected("the active layer is not a shape"));
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
            return Err(CommandError::Rejected(
                "filter effects apply to raster layers only",
            ));
        }
        let Some(params) = FilterParams::default_for_kind(&kind) else {
            return Err(CommandError::InvalidArgument("unknown effect kind"));
        };
        let label = params.label();
        let Some((prev, _)) = self.graph.as_mut().and_then(|g| g.add_effect(id, params)) else {
            return Err(CommandError::Rejected("add effect failed"));
        };
        let next = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.effects.clone())
            .unwrap_or_default();
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
            return Err(CommandError::Rejected(
                "filter effects apply to raster layers only",
            ));
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
            return Err(CommandError::Rejected(
                "filter effects apply to raster layers only",
            ));
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
            .ok_or(CommandError::Rejected("select a layer first"))?;
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
                .ok_or(CommandError::Rejected("select a path first"))?;
            if idx >= graph.paths.paths.len() {
                return Err(CommandError::Rejected("select a path first"));
            }
            f(&mut graph.paths.paths[idx])?;
            let next = graph.paths.clone();
            Ok((None, crate::GraphCommand::SetPaths { prev, next }))
        }
    }

    /// Replace a shape layer's fill and stroke, leaving its path alone.
    ///
    /// Reuses `SetShape` for undo, so recolouring is one entry that restores
    /// the whole payload — geometry included — rather than five parameter
    /// deltas that could be replayed out of order against an edited path.
    fn cmd_shape_set_appearance(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::ShapeSetAppearance { appearance } = args else {
            return Err(CommandError::InvalidArgument("expected ShapeSetAppearance"));
        };
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let active = graph
            .active_id()
            .ok_or(CommandError::Rejected("select a layer first"))?;
        let layer = graph
            .get(active)
            .ok_or(CommandError::Rejected("layer missing"))?;
        if layer.locked || layer.locks.all {
            return Err(CommandError::Rejected(
                "this layer is locked — unlock it to change it",
            ));
        }
        if layer.kind != LayerKind::Shape {
            return Err(CommandError::Rejected("select a shape layer first"));
        }
        let prev = layer
            .shape
            .clone()
            .ok_or(CommandError::Rejected("shape missing path"))?;
        let mut next = prev.clone();
        next.set_appearance(appearance);
        if next == prev {
            return Err(CommandError::Rejected("that is already how it is drawn"));
        }
        if let Some(layer) = graph.get_mut(active) {
            layer.shape = Some(next.clone());
        }
        let generation = self.bump_document_generation();
        self.history.push_graph_applied(
            crate::GraphCommand::SetShape {
                id: active,
                prev: Some(prev),
                next: Some(next),
            },
            "Shape appearance",
            generation,
        );
        let mut effects = CommandEffects::document_edit(generation);
        effects.host_follow_up = HostFollowUp::RasterizeShape { id: active };
        Ok(effects)
    }

    /// The active layer, if it is a smart object and is editable.
    fn active_smart_layer(&self) -> Result<(LayerId, crate::SmartObjectContent), CommandError> {
        let Some(graph) = self.graph.as_ref() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = graph
            .active_id()
            .ok_or(CommandError::Rejected("select a layer first"))?;
        let layer = graph
            .get(id)
            .ok_or(CommandError::Rejected("layer missing"))?;
        if layer.kind != LayerKind::SmartObject {
            return Err(CommandError::Rejected("select a smart object first"));
        }
        if layer.locked || layer.locks.all {
            return Err(CommandError::Rejected(
                "this layer is locked — unlock it to change it",
            ));
        }
        let content = layer
            .smart
            .clone()
            .ok_or(CommandError::Rejected("smart object has no source"))?;
        Ok((id, content))
    }

    /// Wrap the active layer's pixels as a smart object.
    ///
    /// The kind and the payload move together as one history entry: a graph
    /// where one has been undone and the other has not is a smart object with
    /// no source, or a raster layer carrying one.
    fn cmd_smart_create(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SmartCreate { content } = args else {
            return Err(CommandError::InvalidArgument("expected SmartCreate"));
        };
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let id = graph
            .active_id()
            .ok_or(CommandError::Rejected("select a layer first"))?;
        let layer = graph
            .get(id)
            .ok_or(CommandError::Rejected("layer missing"))?;
        if layer.locked || layer.locks.all {
            return Err(CommandError::Rejected(
                "this layer is locked — unlock it to change it",
            ));
        }
        match layer.kind {
            LayerKind::Raster => {}
            LayerKind::SmartObject => {
                return Err(CommandError::Rejected("this is already a smart object"));
            }
            // Group, text, shape, adjustment and fill layers all describe
            // themselves rather than owning a pixel buffer, so there is
            // nothing to capture. Photoshop wraps them by flattening first;
            // that is a separate command and not this one pretending.
            other => {
                let _ = other;
                return Err(CommandError::Rejected(
                    "only a pixel layer can become a smart object — rasterize it first",
                ));
            }
        }
        let prev_kind = layer.kind;
        let prev_smart = layer.smart.clone();
        let next = (*content).clone();
        let command = crate::GraphCommand::Batch(vec![
            crate::GraphCommand::SetKind {
                id,
                prev: prev_kind,
                next: LayerKind::SmartObject,
            },
            crate::GraphCommand::SetSmart {
                id,
                prev: prev_smart,
                next: Some(next),
            },
        ]);
        if !command.apply(graph) {
            return Err(CommandError::Rejected("layer missing"));
        }
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(command, "Convert to smart object", generation);
        Ok(CommandEffects::document_edit(generation))
    }

    /// Replace a smart object's placement.
    fn cmd_smart_set_placement(
        &mut self,
        args: CommandArgs,
    ) -> Result<CommandEffects, CommandError> {
        let CommandArgs::SmartSetPlacement { placement } = args else {
            return Err(CommandError::InvalidArgument("expected SmartSetPlacement"));
        };
        let (id, prev) = self.active_smart_layer()?;
        let mut next = prev.clone();
        next.placement = placement.with_usable_scale(false);
        if next == prev {
            return Err(CommandError::Rejected("that is already where it sits"));
        }
        let command = crate::GraphCommand::SetSmart {
            id,
            prev: Some(prev),
            next: Some(next),
        };
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !command.apply(graph) {
            return Err(CommandError::Rejected("layer missing"));
        }
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(command, "Place smart object", generation);
        let mut effects = CommandEffects::document_edit(generation);
        effects.host_follow_up = HostFollowUp::PlaceSmartObject { id };
        Ok(effects)
    }

    /// Bake a smart object to pixels, dropping its source.
    ///
    /// The pixels on screen are already the placed result, so this is a
    /// bookkeeping change: what it costs is the ability to re-place from the
    /// original, which is the whole point of the kind — hence a history entry
    /// rather than a silent kind flip.
    fn cmd_smart_rasterize(&mut self) -> Result<CommandEffects, CommandError> {
        let (id, prev) = self.active_smart_layer()?;
        let command = crate::GraphCommand::Batch(vec![
            crate::GraphCommand::SetKind {
                id,
                prev: LayerKind::SmartObject,
                next: LayerKind::Raster,
            },
            crate::GraphCommand::SetSmart {
                id,
                prev: Some(prev),
                next: None,
            },
        ]);
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        if !command.apply(graph) {
            return Err(CommandError::Rejected("layer missing"));
        }
        let generation = self.bump_document_generation();
        self.history
            .push_graph_applied(command, "Rasterize smart object", generation);
        Ok(CommandEffects::document_edit(generation))
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
                return Err(CommandError::Rejected("a path needs at least two anchors"));
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
    fn add_layer_style(&mut self, style: LayerStyle) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.kind != LayerKind::Raster {
            return Err(CommandError::Rejected(
                "layer styles apply to raster layers only",
            ));
        }
        let prev = layer.styles.clone();
        let mut next = prev.clone();
        next.push(style);
        layer.styles = next.clone();
        self.record_graph_edit(
            crate::GraphCommand::SetStyles { id, prev, next },
            style.label(),
        )
    }

    /// Edit or remove one style on the active layer.
    ///
    /// The four editing commands share a body because they differ only in how
    /// they transform the style list: read it, change one entry (or drop it),
    /// write it back through the same undo record. Splitting them would mean
    /// four copies of the lookup and the history push.
    fn cmd_style_edit(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let id = self.active_layer_id()?;
        let prev = self
            .graph
            .as_ref()
            .and_then(|g| g.get(id))
            .map(|l| l.styles.clone())
            .ok_or(CommandError::Rejected("layer missing"))?;

        let index = match &args {
            CommandArgs::LayerStyleParams { index, .. }
            | CommandArgs::LayerStyleColor { index, .. }
            | CommandArgs::LayerStyleEnabled { index, .. }
            | CommandArgs::LayerStyleIndex { index } => *index,
            _ => return Err(CommandError::InvalidArgument("expected layer style args")),
        };
        if index >= prev.len() {
            return Err(CommandError::Rejected("no style at that index"));
        }

        let mut next = prev.clone();
        // A slider drag coalesces; the discrete edits do not.
        let mut mergeable = false;
        let label = match args {
            CommandArgs::LayerStyleParams { slots, .. } => {
                next[index] = next[index].with_slots(slots);
                mergeable = true;
                "Style parameters"
            }
            CommandArgs::LayerStyleColor {
                color_index, rgba, ..
            } => {
                next[index] = next[index].with_color(color_index, rgba);
                "Style color"
            }
            CommandArgs::LayerStyleEnabled { enabled, .. } => {
                next[index].set_enabled(enabled);
                if enabled {
                    "Enable style"
                } else {
                    "Disable style"
                }
            }
            CommandArgs::LayerStyleIndex { .. } => {
                next.remove(index);
                "Remove style"
            }
            _ => return Err(CommandError::InvalidArgument("expected layer style args")),
        };
        if next == prev {
            return Err(CommandError::Rejected("style unchanged"));
        }
        let Some(graph) = self.graph.as_mut() else {
            return Err(CommandError::Document(DocumentError::NoDocument));
        };
        let Some(layer) = graph.get_mut(id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        layer.styles = next.clone();
        let command = crate::GraphCommand::SetStyles { id, prev, next };
        if mergeable {
            // Declared `UndoPolicy::Mergeable`: dragging a style slider is one
            // gesture, not one history entry per step.
            let Some(graph) = self.graph.as_mut() else {
                return Err(CommandError::Document(DocumentError::NoDocument));
            };
            graph.bump_generation();
            let generation = graph.generation;
            self.history
                .push_graph_mergeable(command, label, generation);
            return Ok(CommandEffects::document_edit(generation));
        }
        self.record_graph_edit(command, label)
    }

    fn cmd_style_add(&mut self, args: CommandArgs) -> Result<CommandEffects, CommandError> {
        let CommandArgs::LayerStyleKind { kind } = args else {
            return Err(CommandError::InvalidArgument("expected layer style kind"));
        };
        let Some(style) = LayerStyle::default_for_kind(&kind) else {
            return Err(CommandError::InvalidArgument("unknown layer style kind"));
        };
        self.add_layer_style(style)
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
            return Err(CommandError::Rejected("no transform is in progress"));
        };
        if let Some(graph) = self.graph.as_ref() {
            if let Some(layer) = graph.get(session.layer_id) {
                if layer.position_blocked() {
                    self.transform_session = Some(session);
                    return Err(CommandError::Rejected(
                        "this layer's position is locked — unlock it to move it",
                    ));
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
            return Err(CommandError::Rejected(
                "this layer is locked — unlock it to change it",
            ));
        }
    }
    Ok(())
}

/// Refuse the whole operation when any target has its position locked.
///
/// All-or-nothing rather than skipping the locked ones: a partial alignment
/// looks like a bug in the alignment, not like a lock doing its job, and the
/// layer left behind is the one the user is least likely to be looking at.
fn reject_position_locked(
    graph: &crate::DocumentGraph,
    targets: &[LayerId],
) -> Result<(), CommandError> {
    for id in targets {
        let Some(layer) = graph.get(*id) else {
            return Err(CommandError::Rejected("layer missing"));
        };
        if layer.position_blocked() {
            return Err(CommandError::Rejected(
                "this layer's position is locked — unlock it to move it",
            ));
        }
    }
    Ok(())
}

/// Apply per-layer offsets, recording an undo entry for each layer that moved.
///
/// Offsets under a twentieth of a pixel are dropped. Alignment arithmetic on
/// measured boxes rarely lands on exactly zero, and without a floor "align
/// left" on already-aligned layers would push a no-op onto the undo stack that
/// the user then has to press undo to clear.
fn align_batch(
    graph: &mut crate::DocumentGraph,
    targets: &[crate::AlignTarget],
    offsets: &[(f32, f32)],
) -> Vec<crate::GraphCommand> {
    const EPSILON: f32 = 0.05;
    let mut batch = Vec::new();
    for (target, (dx, dy)) in targets.iter().zip(offsets) {
        if dx.abs() < EPSILON && dy.abs() < EPSILON {
            continue;
        }
        // Every member of a target takes the same offset, so a group keeps its
        // internal arrangement instead of collapsing onto one edge.
        for id in &target.members {
            let Some(layer) = graph.get(*id) else {
                continue;
            };
            let prev = layer.transform;
            let next = LayerTransform {
                translate_x: prev.translate_x + dx,
                translate_y: prev.translate_y + dy,
                ..prev
            };
            if graph.set_transform(*id, next).is_some() {
                batch.push(crate::GraphCommand::SetTransform {
                    id: *id,
                    prev,
                    next,
                });
            }
        }
    }
    batch
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
            return Err(CommandError::Rejected(
                "this layer is locked — unlock it to change it",
            ));
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

/// History label for a clockwise quarter-turn count.
///
/// Spelled the way the menu spells it, so the history row and the entry the
/// user clicked say the same thing.
fn rotation_label(turns: u32) -> &'static str {
    match turns % 4 {
        1 => "Rotate 90° CW",
        2 => "Rotate 180°",
        _ => "Rotate 90° CCW",
    }
}
