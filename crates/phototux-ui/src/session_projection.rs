//! Engine → QML projection.
//!
//! Everything that pushes engine state out to published properties lives here:
//! `sync_from_engine` and the helpers it calls, plus the `emit_*` family that
//! republishes one concern after a targeted edit.
//!
//! It is a separate module because it is one concern and 878 lines of it, not
//! because `lib.rs` was long. The host's own `impl` blocks can live in any
//! module of this crate — the `#[qobject]` block cannot be split, since the
//! macro processes it whole, but this one carries no slots and moves freely.
//!
//! The ordering rule that governs `sync_from_engine` is stated on the function
//! itself and is the reason these helpers are contiguous runs rather than a
//! tidier taxonomy: each `publish!` emits a Qt notify a QML binding may react
//! to synchronously, so where a projection sits in the sequence is observable.

use crate::publish;
use crate::{AppSession, blend_if_state_json, shape_state_json, smart_state_json};
use phototux_engine::LayerKind;

impl AppSession {
    // `pub(crate)` rather than private: these are the same internal helpers
    // they were, in a different file, and privacy in Rust flows to a module's
    // *descendants* — the parent that calls them is an ancestor. The visibility
    // widened because the code moved, not because the surface did.

    pub(crate) fn sync_camera_from_engine(&mut self) {
        self.zoom = self.engine.camera.zoom;
        self.pan_x = self.engine.camera.pan_x;
        self.pan_y = self.engine.camera.pan_y;
        self.status_text = self.engine.status_summary();
    }

    pub(crate) fn publish_notices(&mut self) {
        let json = self.notices.to_json();
        publish!(self, notices_json, json, notices_json_changed);
    }

    /// Republish everything QML reads from the engine, in one pass.
    ///
    /// A list of calls, and it has to stay one. Each `publish!` emits a Qt
    /// notify that a QML binding may react to *synchronously*, so the order
    /// these run in is observable: moving a projection earlier or later lets a
    /// binding see an intermediate state it never saw before. The helpers below
    /// are therefore contiguous runs of the original body, called in the order
    /// they were written in — a move, not a rearrangement.
    ///
    /// `sync_from_engine_is_a_list_of_calls` keeps it that way.
    pub(crate) fn sync_from_engine(&mut self) {
        self.sync_camera_fields();
        self.sync_brush_fields();
        self.sync_frame_metrics();
        self.sync_tool_and_history_fields();
        self.sync_layer_rows();
        self.sync_active_layer_locks();
        self.sync_mask_fields();
        self.sync_edit_target_fields();
        self.publish_history_projection();
        self.sync_brush_preset_fields();
        self.sync_color_management_fields();
        self.sync_accessibility_fields();
        self.sync_selection_fields();
        self.sync_transform_fields();
        self.sync_path_and_revision_fields();
        self.sync_active_layer_style_fields();
        self.sync_inspector_mixed_fields();
        self.sync_color_fields();
        self.sync_viewport_fields();
        self.sync_adjustment_fields();
        self.sync_text_fields();
        self.sync_path_edit_fields();
        self.sync_filter_preview_fields();
        self.sync_guides_fields();
        self.refresh_pref_effective_json();
        self.pref_effective_json_changed();
        self.refresh_inspector_badges();
        self.sync_status_field();
    }

    /// Document size and where the camera is looking.
    pub(crate) fn sync_camera_fields(&mut self) {
        self.doc_width = self.engine.size.width as i32;
        self.doc_height = self.engine.size.height as i32;
        self.zoom = self.engine.camera.zoom;
        self.pan_x = self.engine.camera.pan_x;
        self.pan_y = self.engine.camera.pan_y;
    }

    /// The brush's own parameters.
    ///
    /// `brush_r`/`g`/`b` are assigned again by `sync_color_fields` further
    /// down, from `colors.foreground`. Every writer keeps the two in step —
    /// `set_brush_color` pushes into both, and `adopt_foreground_without_recording`
    /// writes `brush_color` explicitly — so the second assignment is redundant
    /// rather than contradictory. It is left in place here because this split
    /// is a move and nothing else; removing it is a separate question.
    pub(crate) fn sync_brush_fields(&mut self) {
        self.brush_size = self.engine.brush_size;
        self.brush_hardness = self.engine.brush_hardness;
        self.brush_texture_strength = self.engine.brush.texture_strength;
        self.brush_r = self.engine.brush_color[0];
        self.brush_g = self.engine.brush_color[1];
        self.brush_b = self.engine.brush_color[2];
    }

    /// What the last frame cost, and what it touched.
    ///
    /// `publish!` rather than bare assignment for most of these: they are
    /// recomputed every sync, and emitting unconditionally is what flooded
    /// AT-SPI and killed a session (T-009).
    pub(crate) fn sync_frame_metrics(&mut self) {
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
    }

    /// The active tool, whether there is a document, and what undo can offer.
    pub(crate) fn sync_tool_and_history_fields(&mut self) {
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
    }

    /// Bring the layer model's rows up to date.
    pub(crate) fn sync_layer_rows(&mut self) {
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
    }

    /// What the active layer will and will not accept.
    ///
    /// The four locks are separate properties because the Layers panel draws
    /// them as four toggles, and `active_layer_locked` is the composite the
    /// menus and panel controls gate on.
    pub(crate) fn sync_active_layer_locks(&mut self) {
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
        publish!(
            self,
            active_layer_locked,
            self.engine.active_layer_change_blocked(),
            active_layer_locked_changed
        );
        publish!(
            self,
            active_lock_pixels,
            self.engine.active_lock_pixels(),
            active_lock_pixels_changed
        );
        publish!(
            self,
            active_lock_position,
            self.engine.active_lock_position(),
            active_lock_position_changed
        );
        publish!(
            self,
            active_lock_alpha,
            self.engine.active_lock_alpha(),
            active_lock_alpha_changed
        );
    }

    /// The active layer's mask, and whether the mask is what painting edits.
    ///
    /// Starts by clearing a stale `mask_edit_layer`: the layer it points at may
    /// have lost its mask since the target was chosen, and painting would then
    /// be aimed at something that no longer exists.
    pub(crate) fn sync_mask_fields(&mut self) {
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
    }

    /// What the inspector is describing, and what an edit would land on.
    pub(crate) fn sync_edit_target_fields(&mut self) {
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
    }

    /// The brush preset names, for the options bar's combo.
    pub(crate) fn sync_brush_preset_fields(&mut self) {
        publish!(
            self,
            brush_preset_names,
            self.engine.brush_presets.names_joined(),
            brush_preset_names_changed
        );
    }

    /// Soft proof and embedded profile state.
    pub(crate) fn sync_color_management_fields(&mut self) {
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
    }

    /// The semantic tree, and its AT-SPI projection.
    ///
    /// The projection is rebuilt only when the tree actually changed — it is
    /// derived from a string this function has just published, and rebuilding
    /// it every sync is the same flood `publish!` exists to prevent.
    pub(crate) fn sync_accessibility_fields(&mut self) {
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
    }

    /// Where the document came from, and which revision this is.
    pub(crate) fn sync_path_and_revision_fields(&mut self) {
        publish!(
            self,
            document_path,
            self.engine.document_path.clone().unwrap_or_default(),
            document_path_changed
        );
        publish!(
            self,
            source_path,
            self.engine.source_path.clone().unwrap_or_default(),
            source_path_changed
        );
        publish!(
            self,
            graph_revision,
            self.engine.graph_revision() as i32,
            graph_revision_changed
        );
    }

    /// The active layer's opacity, blend mode and fill colour.
    ///
    /// The defaults are the interesting half: they are what the panel shows
    /// with no layer active, and stating them beside the real values keeps the
    /// two arms honest.
    pub(crate) fn sync_active_layer_style_fields(&mut self) {
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
    }

    /// The size of the viewport the canvas is presented into.
    pub(crate) fn sync_viewport_fields(&mut self) {
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
    }

    /// The status bar's summary line.
    ///
    /// Last, deliberately: it summarises state the rest of this sync has just
    /// finished publishing.
    pub(crate) fn sync_status_field(&mut self) {
        publish!(
            self,
            status_text,
            self.engine.status_summary(),
            status_text_changed
        );
    }

    pub(crate) fn sync_color_fields(&mut self) {
        use phototux_engine::ColorState;
        self.foreground_hex = ColorState::to_hex(self.engine.colors.foreground);
        self.background_hex = ColorState::to_hex(self.engine.colors.background);
        self.recent_colors = self.engine.colors.recent_hex_joined();
        let fg = self.engine.colors.foreground;
        self.brush_r = fg[0];
        self.brush_g = fg[1];
        self.brush_b = fg[2];
    }

    /// Everything the Properties panel shows about the active layer's
    /// adjustment, styles, blending and effects.
    ///
    /// The layer is read once. It used to be looked up twice — the second time
    /// only to reach the effects, because the first borrow had to be dropped
    /// before anything could be published — and the fields after that second
    /// lookup were the ones that never joined the publish protocol: they were
    /// assigned bare here and announced blind from `emit_layer_fields`, three
    /// hundred lines away, where the two halves could drift apart unseen.
    ///
    /// The no-layer arm states the empty value for every field rather than
    /// returning early. Returning early is what left `layer_styles_json` and
    /// `effects_joined` describing a layer that was no longer active.
    pub(crate) fn sync_adjustment_fields(&mut self) {
        let layer = self
            .engine
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)));
        // Copied out before publishing: the layer is borrowed from `self`, and
        // the publisher needs `self` mutably.
        let (adjustment, styles, blend_if, shape, smart, gaussian) = match layer {
            Some(layer) => (
                layer.adjustment.as_ref().map(|params| {
                    (
                        params.kind_key(),
                        params.slots(),
                        params.editor_slots().len(),
                    )
                }),
                phototux_engine::layer_styles_json(&layer.styles),
                blend_if_state_json(layer.blend_if),
                shape_state_json(layer.shape.as_ref()),
                smart_state_json(
                    layer.smart.as_ref(),
                    self.smart_sources.contains_key(&layer.id),
                ),
                layer.effects.iter().find_map(|effect| match effect.params {
                    phototux_engine::FilterParams::GaussianBlur { radius } if effect.enabled => {
                        Some(radius)
                    }
                    _ => None,
                }),
            ),
            None => (
                None,
                phototux_engine::layer_styles_json(&[]),
                blend_if_state_json(phototux_engine::BlendIf::default()),
                shape_state_json(None),
                smart_state_json(None, false),
                None,
            ),
        };

        publish!(self, blend_if_json, blend_if, blend_if_json_changed);
        publish!(self, shape_json, shape, shape_json_changed);
        publish!(self, smart_json, smart, smart_json_changed);
        publish!(self, layer_styles_json, styles, layer_styles_json_changed);
        match adjustment {
            Some((kind, slots, used)) => {
                publish!(
                    self,
                    adjustment_kind,
                    kind.to_owned(),
                    adjustment_kind_changed
                );
                self.set_adjustment_slot_values(&slots[..used]);
            }
            None => {
                publish!(
                    self,
                    adjustment_kind,
                    String::new(),
                    adjustment_kind_changed
                );
                self.set_adjustment_slot_values(&[]);
            }
        }
        publish!(
            self,
            has_gaussian_blur,
            gaussian.is_some(),
            has_gaussian_blur_changed
        );
        publish!(
            self,
            gaussian_radius,
            gaussian.unwrap_or(0.0),
            gaussian_radius_changed
        );
        let effects = self.engine.active_effects_joined();
        publish!(self, effects_joined, effects, effects_joined_changed);
    }

    pub(crate) fn sync_text_fields(&mut self) {
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

    pub(crate) fn sync_path_edit_fields(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            self.path_closed = false;
            self.path_anchor_count = 0;
            self.path_edit_selected = -1;
            self.clear_path_geometry();
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
                // `publish!` rather than an unconditional emit: this runs on
                // every sync, and a projection republished per frame is what
                // flooded AT-SPI and killed a session in T-009.
                let geometry = path.anchors_json();
                publish!(
                    self,
                    path_geometry_json,
                    geometry,
                    path_geometry_json_changed
                );
            }
            None => {
                self.path_closed = false;
                self.path_anchor_count = 0;
                self.path_edit_selected = -1;
                self.clear_path_geometry();
            }
        }
    }

    pub(crate) fn sync_filter_preview_fields(&mut self) {
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

    pub(crate) fn sync_guides_fields(&mut self) {
        self.pref_show_guides = self.engine.guides.show_guides;
        self.pref_show_grid = self.engine.guides.show_grid;
        self.pref_show_rulers = self.engine.guides.show_rulers;
        self.pref_snap = self.engine.guides.snap;
        self.grid_spacing = self.engine.guides.grid_spacing;
        self.guides_json = self.engine.guides.guides_json();
    }

    pub(crate) fn emit_text_fields(&mut self) {
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

    pub(crate) fn emit_path_edit_fields(&mut self) {
        self.path_closed_changed();
        self.path_anchor_count_changed();
        self.path_edit_selected_changed();
    }

    pub(crate) fn emit_filter_preview_fields(&mut self) {
        self.filter_preview_active_changed();
        self.filter_preview_kind_changed();
        self.filter_preview_p0_changed();
        self.filter_preview_p1_changed();
        self.filter_preview_p2_changed();
    }

    pub(crate) fn emit_guides_fields(&mut self) {
        self.pref_show_guides_changed();
        self.pref_show_grid_changed();
        self.pref_show_rulers_changed();
        self.pref_snap_changed();
        self.guides_json_changed();
        self.grid_spacing_changed();
    }

    pub(crate) fn emit_camera_fields(&mut self) {
        self.zoom_changed();
        self.pan_x_changed();
        self.pan_y_changed();
        self.status_text_changed();
    }

    pub(crate) fn emit_color_fields(&mut self) {
        self.brush_color_changed();
        self.foreground_hex_changed();
        self.background_hex_changed();
        self.recent_colors_changed();
    }

    pub(crate) fn sync_inspector_mixed_fields(&mut self) {
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

    /// Bring the history model up to date.
    ///
    /// The model decides for itself whether anything moved and tells Qt only
    /// what did, so callers no longer need to guess. Three index-aligned
    /// strings used to describe this one list, and both callers rebuilt all
    /// three regardless of which had changed.
    pub(crate) fn publish_history_projection(&mut self) {
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
    pub(crate) fn emit_layer_fields(&mut self) {
        self.sync_inspector_mixed_fields();
        self.emit_selection_fields();
        self.emit_transform_fields();
        self.foreground_hex_changed();
        self.background_hex_changed();
        self.recent_colors_changed();
        self.brush_color_changed();
        self.emit_text_fields();
        self.emit_path_edit_fields();
        self.emit_filter_preview_fields();
        self.emit_guides_fields();
    }

    pub(crate) fn sync_selection_fields(&mut self) {
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

    pub(crate) fn emit_selection_fields(&mut self) {
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

    pub(crate) fn sync_transform_fields(&mut self) {
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

    pub(crate) fn emit_transform_fields(&mut self) {
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

    pub(crate) fn emit_doc_fields(&mut self) {
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
        // The tab strip is a third view of `document_name` and `dirty`, and it
        // is pushed rather than bound — so it goes stale wherever those two
        // change and only the properties are re-emitted. Save As renamed the
        // window title and left the tab reading "Untitled"; a save left the
        // tab's dirty dot on. Refreshed here so a caller cannot forget.
        self.refresh_document_tabs_json();
    }

    pub(crate) fn emit_workspace_presets_json(&mut self) {
        self.workspace_presets_json_changed();
    }

    pub(crate) fn emit_shortcut_maps(&mut self) {
        self.shortcuts_json_changed();
        self.action_shortcuts_json_changed();
    }

    pub(crate) fn sync_pref_fields_from_store(&mut self) {
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

    pub(crate) fn sync_panel_visibility_from_workspace(&mut self) {
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

    pub(crate) fn emit_pref_fields(&mut self) {
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
}
