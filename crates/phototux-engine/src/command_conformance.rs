//! Headless command-router conformance suite (handbook 31 / DR-022).

#[cfg(test)]
mod tests {
    use crate::command_id;
    use crate::command_meta;
    use crate::command_meta::{CommandScope, MutationClass};
    use crate::layer_style::LayerStyle;
    use crate::{CommandArgs, CommandError, SessionState, SizePreset};

    /// Styles of the layer the commands actually target.
    fn active_styles(session: &SessionState) -> Vec<LayerStyle> {
        session
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .map(|l| l.styles.clone())
            .expect("an active layer")
    }

    #[test]
    fn every_registered_id_is_known_and_has_meta() {
        for id in command_id::ALL {
            assert!(
                SessionState::command_known(id),
                "command not known to router: {id}"
            );
            assert!(
                command_meta::meta_for(id).is_some(),
                "missing CommandMeta: {id}"
            );
        }
    }

    /// Arguments good enough to invoke each non-document command once.
    ///
    /// Every command whose taxonomy says it does not touch the document has to
    /// appear here, and the test below fails if one does not — otherwise adding
    /// a view command would quietly opt it out of the check it is supposed to
    /// satisfy.
    fn args_for(id: &str) -> CommandArgs {
        match id {
            command_id::VIEW_ZOOM_TO => CommandArgs::Zoom { zoom: 2.0 },
            command_id::VIEW_PAN_TO => CommandArgs::Pan {
                world_x: 10.0,
                world_y: 12.0,
            },
            command_id::VIEW_PAN_BY => CommandArgs::PanBy { dx: 4.0, dy: 6.0 },
            command_id::VIEW_ZOOM_AT => CommandArgs::ZoomAt {
                factor: 1.25,
                anchor_x: 100.0,
                anchor_y: 80.0,
            },
            command_id::VIEW_SET_TOOL => CommandArgs::Tool {
                tool: crate::tool_id::ERASER.to_owned(),
            },
            command_id::FILTER_PREVIEW => CommandArgs::FilterPreview {
                kind: "gaussian-blur".to_owned(),
            },
            command_id::FILTER_SET_PREVIEW_PARAMS => CommandArgs::FilterPreviewParams {
                p0: 4.0,
                p1: 0.0,
                p2: 0.0,
            },
            command_id::WORKSPACE_TOGGLE_PANEL => CommandArgs::TogglePanel {
                panel_id: "panel.layers".to_owned(),
            },
            command_id::WORKSPACE_APPLY_PRESET => CommandArgs::ApplyWorkspacePreset {
                preset_id: "workspace.preset.compact".to_owned(),
            },
            _ => CommandArgs::None,
        }
    }

    /// A command that claims not to touch the document must not touch it.
    ///
    /// This used to invoke three hand-picked commands. Twelve claim view or
    /// workspace scope, so nine of them were asserting nothing, and
    /// `CommandScope` and `MutationClass` were read by no code at all —
    /// declared for every command and believed on trust. That is the shape that
    /// let `UndoPolicy::Mergeable` sit unimplemented for as long as it did.
    ///
    /// Driving the loop off the taxonomy makes the declaration a claim the
    /// suite checks: mark a command ephemeral and it must leave the generation
    /// and the dirty flag alone, or this fails naming it.
    #[test]
    fn commands_that_claim_not_to_touch_the_document_do_not() {
        let non_document: Vec<&'static str> = command_meta::ALL
            .iter()
            .filter(|m| {
                m.mutation == MutationClass::Ephemeral || m.scope == CommandScope::Workspace
            })
            .map(|m| m.id)
            .collect();
        assert!(
            non_document.len() >= 12,
            "expected the view and workspace commands to be declared as such, found {}",
            non_document.len()
        );

        for id in non_document {
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            let generation_before = session.document_generation();
            let dirty_before = session.is_dirty_vs_persisted();

            // A refusal is fine — some need a document state this fixture does
            // not set up. What is not fine is succeeding and dirtying anyway.
            let _ = session.invoke(id, args_for(id));

            assert_eq!(
                session.document_generation(),
                generation_before,
                "{id} is declared not to touch the document but bumped its generation"
            );
            assert_eq!(
                session.is_dirty_vs_persisted(),
                dirty_before,
                "{id} is declared not to touch the document but marked it dirty"
            );
        }
    }

    /// Soft-proof is document-scoped chrome: it changes what the canvas shows
    /// without editing pixels, so it keeps its own case rather than being
    /// folded into the taxonomy loop above.
    #[test]
    fn soft_proof_changes_display_without_dirtying() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let generation_before = session.document_generation();
        let dirty = session.is_dirty_vs_persisted();
        session
            .invoke(
                command_id::DOCUMENT_SET_SOFT_PROOF,
                CommandArgs::SoftProof {
                    profile: "Display-P3".into(),
                    intent: "relative".into(),
                },
            )
            .expect("soft proof");
        assert_eq!(session.document_generation(), generation_before);
        assert_eq!(session.is_dirty_vs_persisted(), dirty);
        assert!(
            session
                .graph
                .as_ref()
                .is_some_and(|g| g.color.soft_proof_active())
        );
    }

    #[test]
    fn locked_layer_rejects_paint_stroke() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        session
            .invoke(
                command_id::LAYER_SET_LOCKS,
                CommandArgs::SetLocks {
                    pixels: true,
                    position: false,
                    all: false,
                    alpha: false,
                },
            )
            .expect("lock");
        let err = session
            .invoke(
                command_id::RASTER_PAINT_STROKE,
                CommandArgs::RasterPaintStroke {
                    label: "stroke".into(),
                },
            )
            .expect_err("locked paint");
        assert!(format!("{err}").contains("locked"));
    }

    /// Layer styles declare `UndoPolicy::Transaction` in `command_meta`. They
    /// bumped the generation and recorded nothing, so Ctrl+Z left the style in
    /// place. This drives every style kind through the real router and asserts
    /// undo puts the graph back exactly as it was — iterating the vocabulary
    /// rather than four command ids, so a new style is covered on arrival.
    #[test]
    fn every_style_kind_round_trips_through_undo() {
        for style in crate::LayerStyle::ALL_KINDS {
            let kind = style.kind_key();
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            let before = active_styles(&session);

            session
                .invoke(
                    command_id::STYLE_ADD,
                    CommandArgs::LayerStyleKind {
                        kind: kind.to_owned(),
                    },
                )
                .expect(kind);
            let after = active_styles(&session);
            assert_eq!(after.len(), before.len() + 1, "{kind} did not add a style");
            assert_eq!(after.last().map(LayerStyle::kind_key), Some(kind));

            session
                .invoke(command_id::HISTORY_UNDO, CommandArgs::None)
                .expect("undo");
            let undone = active_styles(&session);
            assert_eq!(undone, before, "{kind} was not undoable");
        }
    }

    /// A style kind the vocabulary does not name must be refused, not silently
    /// turned into a drop shadow.
    #[test]
    fn an_unknown_style_kind_is_refused() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let err = session
            .invoke(
                command_id::STYLE_ADD,
                CommandArgs::LayerStyleKind {
                    kind: "nonsense".into(),
                },
            )
            .expect_err("unknown style kind");
        assert!(matches!(err, CommandError::InvalidArgument(_)), "{err}");
    }

    /// `mask.create-vector` recorded a `Transform` entry, which `undo_next`
    /// hands to the host as a no-op — undo consumed a step and left the vector
    /// mask attached.
    #[test]
    fn create_vector_mask_round_trips_through_undo() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        session
            .invoke(command_id::MASK_CREATE_VECTOR, CommandArgs::None)
            .expect("create vector mask");
        assert!(
            session
                .graph
                .as_ref()
                .and_then(|g| g.active_id().and_then(|id| g.get(id)))
                .is_some_and(|l| l.vector_mask.is_some()),
            "vector mask was not attached"
        );

        session
            .invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo");
        assert!(
            session
                .graph
                .as_ref()
                .and_then(|g| g.active_id().and_then(|id| g.get(id)))
                .is_some_and(|l| l.vector_mask.is_none()),
            "undo left the vector mask attached"
        );
    }

    /// A command that moves the document generation must leave something for
    /// undo to pop. Catches the "bumped but never recorded" shape generically,
    /// rather than one command at a time.
    #[test]
    fn document_edits_leave_an_undo_entry() {
        // Every layer style, plus a representative of the styles' neighbours.
        let mut cases: Vec<(&str, CommandArgs)> = crate::LayerStyle::ALL_KINDS
            .iter()
            .map(|style| {
                (
                    command_id::STYLE_ADD,
                    CommandArgs::LayerStyleKind {
                        kind: style.kind_key().to_owned(),
                    },
                )
            })
            .collect();
        cases.push((command_id::MASK_CREATE_VECTOR, CommandArgs::None));

        for (id, args) in cases {
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            let generation_before = session.document_generation();
            let undo_before = session.history.entries_undo().len();

            session.invoke(id, args).expect(id);

            assert!(
                session.document_generation() > generation_before,
                "{id} did not move the generation"
            );
            assert!(
                session.history.entries_undo().len() > undo_before,
                "{id} moved the generation without recording an undo entry"
            );
        }
    }

    /// `UndoPolicy::Mergeable` was declared on four commands and read by
    /// nothing: a slider drag wrote one history entry per step, and undo walked
    /// back through every one instead of returning to where the gesture began.
    /// This asserts the declaration is true for the command that is easiest to
    /// drive end to end.
    #[test]
    fn a_mergeable_command_coalesces_a_run_into_one_entry() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        assert_eq!(
            command_meta::meta_for(command_id::LAYER_SET_OPACITY).map(|m| m.undo),
            Some(command_meta::UndoPolicy::Mergeable),
            "this test is only meaningful while the command declares Mergeable"
        );

        let before = session.history.entries_undo().len();
        for step in 1..=5 {
            session
                .invoke(
                    command_id::LAYER_SET_OPACITY,
                    CommandArgs::SetOpacity {
                        opacity: 1.0 - (step as f32) * 0.1,
                    },
                )
                .expect("set opacity");
        }
        assert_eq!(
            session.history.entries_undo().len(),
            before + 1,
            "a run of mergeable edits must be one entry"
        );
    }

    /// The whole point of keeping the oldest `prev`: one undo returns to where
    /// the gesture started, not to the previous step of it.
    #[test]
    fn undoing_a_merged_run_returns_to_where_it_started() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let original = active_opacity(&session);

        for step in 1..=4 {
            session
                .invoke(
                    command_id::LAYER_SET_OPACITY,
                    CommandArgs::SetOpacity {
                        opacity: 1.0 - (step as f32) * 0.15,
                    },
                )
                .expect("set opacity");
        }
        assert!(
            (active_opacity(&session) - original).abs() > 0.01,
            "opacity moved"
        );

        session
            .invoke(command_id::HISTORY_UNDO, CommandArgs::None)
            .expect("undo");
        assert!(
            (active_opacity(&session) - original).abs() < 1e-5,
            "one undo must return to the start of the run, got {}",
            active_opacity(&session)
        );
    }

    /// Merging must not swallow unrelated edits: a different command between
    /// two mergeable ones ends the run.
    #[test]
    fn an_interrupted_run_stays_two_entries() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let before = session.history.entries_undo().len();

        session
            .invoke(
                command_id::LAYER_SET_OPACITY,
                CommandArgs::SetOpacity { opacity: 0.8 },
            )
            .expect("opacity");
        session
            .invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("add layer");
        session
            .invoke(
                command_id::LAYER_SET_OPACITY,
                CommandArgs::SetOpacity { opacity: 0.6 },
            )
            .expect("opacity");

        assert_eq!(
            session.history.entries_undo().len(),
            before + 3,
            "an interrupted run must not fold across the interruption"
        );
    }

    fn active_opacity(session: &SessionState) -> f32 {
        session
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .map(|l| l.opacity)
            .expect("an active layer")
    }
}
