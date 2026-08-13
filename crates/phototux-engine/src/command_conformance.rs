//! Headless command-router conformance suite (handbook 31 / DR-022).

#[cfg(test)]
mod tests {
    use crate::command_id;
    use crate::command_meta;
    use crate::layer_style::LayerStyle;
    use crate::{CommandArgs, SessionState, SizePreset};

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

    #[test]
    fn view_and_workspace_commands_do_not_dirty_document() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let generation_before = session.document_generation();
        let dirty = session.is_dirty_vs_persisted();

        session
            .invoke(command_id::VIEW_ZOOM_TO_FIT, CommandArgs::None)
            .expect("zoom fit");
        session
            .invoke(
                command_id::WORKSPACE_APPLY_PRESET,
                CommandArgs::ApplyWorkspacePreset {
                    preset_id: "workspace.preset.compact".into(),
                },
            )
            .expect("preset");
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
    /// place. This drives each of the four through the real router and asserts
    /// undo puts the graph back exactly as it was.
    #[test]
    fn every_style_command_round_trips_through_undo() {
        for id in [
            command_id::STYLE_ADD_DROP_SHADOW,
            command_id::STYLE_ADD_STROKE,
            command_id::STYLE_ADD_OUTER_GLOW,
            command_id::STYLE_ADD_COLOR_OVERLAY,
        ] {
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            let before = active_styles(&session);

            session.invoke(id, CommandArgs::None).expect(id);
            let after = active_styles(&session);
            assert_eq!(after.len(), before.len() + 1, "{id} did not add a style");

            session
                .invoke(command_id::HISTORY_UNDO, CommandArgs::None)
                .expect("undo");
            let undone = active_styles(&session);
            assert_eq!(undone, before, "{id} was not undoable");
        }
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
        for id in [
            command_id::STYLE_ADD_DROP_SHADOW,
            command_id::STYLE_ADD_STROKE,
            command_id::STYLE_ADD_OUTER_GLOW,
            command_id::STYLE_ADD_COLOR_OVERLAY,
            command_id::MASK_CREATE_VECTOR,
        ] {
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            let generation_before = session.document_generation();
            let undo_before = session.history.entries_undo().len();

            session.invoke(id, CommandArgs::None).expect(id);

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
}
