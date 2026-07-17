//! Headless command-router conformance suite (handbook 31 / DR-022).

#[cfg(test)]
mod tests {
    use crate::command_id;
    use crate::command_meta;
    use crate::{CommandArgs, SessionState, SizePreset};

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
}
