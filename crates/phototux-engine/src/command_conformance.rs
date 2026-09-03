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
    /// Every registry action builds arguments its command will accept.
    ///
    /// An action carries a command id and an optional string; the router
    /// destructures a specific [`CommandArgs`] variant. `args_for_action` maps
    /// one to the other and ends in a catch-all that answers
    /// `CommandArgs::None` — so an action naming a command whose variant it
    /// does not build sends `None`, the command refuses with
    /// `InvalidArgument`, and from the shell that is a menu entry that does
    /// nothing. The wiring was written in three places and checked in none.
    ///
    /// `InvalidArgument` is the only error this rejects. A command refusing
    /// because there is no document, no active layer or nothing selected is
    /// answering correctly — that is a *precondition*, and the enablement tags
    /// are what keep it off the screen. Only "you handed me the wrong shape"
    /// is a wiring bug.
    ///
    /// Two sessions, because the two halves of the registry disagree about
    /// what must exist: an empty one catches a command that refuses its own
    /// arguments before it looks at the document, and one with a document and
    /// a layer reaches the arms that destructure first.
    #[test]
    fn every_action_builds_arguments_its_command_accepts() {
        let mut checked = 0_usize;
        for empty in [true, false] {
            let mut session = SessionState::default();
            if !empty {
                session.apply_preset(SizePreset::P720);
            }
            for action in crate::default_actions() {
                let Some(id) = action.command_id.as_deref() else {
                    continue;
                };
                let args = match session.args_for_action(id, action.arg.as_deref()) {
                    Ok(args) => args,
                    Err(error) => panic!(
                        "{} carries arg {:?}, which args_for_action refuses for {id}: {error:?}",
                        action.id, action.arg
                    ),
                };
                if let Err(CommandError::InvalidArgument(why)) = session.invoke(id, args) {
                    panic!(
                        "{} sends {id} arguments it will not take ({why}) — the menu \
                         entry would do nothing",
                        action.id
                    );
                }
                checked += 1;
            }
        }
        assert!(
            checked > 40,
            "walked {checked} actions — the registry scan broke, not the wiring"
        );
    }

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
    /// No adjustment can be made to produce a pixel that is not a number.
    ///
    /// `AdjustmentParams::clamped` used `f32::clamp` in all ten arms, and
    /// `f32::clamp` propagates NaN — so the function whose job is to make a
    /// slot safe passed one through, and `apply_rgb` then returned
    /// `[NaN, NaN, NaN]`. The slots come from QML sliders, where a viewport
    /// division during a drag is enough to make one.
    ///
    /// Two layers, tested together: `filter.set-parameters` refuses a
    /// non-finite slot outright, keeping the value the user already had, and
    /// `clamped` is total for anything that reaches it another way.
    #[test]
    fn no_adjustment_slot_can_produce_a_pixel_that_is_not_a_number() {
        use crate::AdjustmentParams;

        for kind in AdjustmentParams::ALL_KINDS {
            for i in 0..kind.editor_slots().len() {
                for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                    let mut slots = kind.slots();
                    slots[i] = bad;
                    let applied = kind.with_slots(slots).clamped();
                    for value in applied.slots() {
                        assert!(
                            value.is_finite(),
                            "{}: slot {i} of {bad} survived clamped() as {value}",
                            kind.kind_key()
                        );
                    }
                    let out = applied.apply_rgb([0.25, 0.5, 0.75]);
                    assert!(
                        out.iter().all(|c| c.is_finite()),
                        "{}: slot {i} of {bad} produced {out:?}",
                        kind.kind_key()
                    );
                }
            }
        }
    }

    /// And the command refuses one rather than inventing a replacement.
    #[test]
    fn a_filter_refuses_a_slot_that_is_not_a_number() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        session
            .invoke(
                command_id::FILTER_ADD_ADJUSTMENT,
                CommandArgs::FilterAdjustment {
                    kind: "brightness".to_owned(),
                },
            )
            .expect("adjustment layer");
        let mut slots = [0.0f32; crate::MAX_ADJUSTMENT_SLOTS];
        slots[0] = f32::NAN;
        assert!(
            matches!(
                session.invoke(
                    command_id::FILTER_SET_PARAMETERS,
                    CommandArgs::FilterParameters { slots }
                ),
                Err(CommandError::InvalidArgument(_))
            ),
            "a NaN slot must be refused, not clamped to something invented"
        );
    }
    /// The published shortcut reference lists exactly the chords that ship.
    ///
    /// `web/docs/reference/shortcuts.md` is a second copy of the registry's
    /// chord vocabulary, written for users and read by nobody who could notice
    /// it going stale. A chord renamed here leaves the site telling people to
    /// press something that does nothing — the worst kind of documentation
    /// bug, because the reader trusts it over the application.
    ///
    /// Both directions: a documented chord the registry does not bind is a
    /// promise the build does not keep, and a bound chord the reference omits
    /// is a feature nobody can find.
    #[test]
    fn the_published_reference_lists_the_chords_that_ship() {
        let page = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/docs/src/content/docs/reference/shortcuts.md"
        ))
        .expect("the shortcut reference is readable from the engine crate");

        // Only the action tables, which run until the modifier section. What
        // follows that heading is held keys — Space to pan, Alt to zoom out —
        // which the canvas handles directly and the registry never binds, so
        // reading them as action chords reported Space as a promise nothing
        // keeps. The heading is asserted below so that renaming it fails here
        // rather than quietly widening what this test accepts.
        const MODIFIERS: &str = "## Modifiers on the canvas";
        assert!(
            page.contains(MODIFIERS),
            "the modifier section moved; this test no longer knows where the \
             action chords stop"
        );
        let actions_only = page.split(MODIFIERS).next().unwrap_or_default();

        // Rows are `| Label | <kbd>Ctrl</kbd> <kbd>N</kbd> |`; the chord is
        // whichever cell carries `<kbd>`. Table rows only — the page's prose
        // uses `<kbd>` too, and reading that made the first run report "Ctrl"
        // as a binding nothing answers.
        let mut documented: Vec<String> = Vec::new();
        for line in actions_only.lines() {
            if !line.trim_start().starts_with('|') {
                continue;
            }
            let Some(cell) = line.split('|').find(|cell| cell.contains("<kbd>")) else {
                continue;
            };
            let keys: Vec<&str> = cell
                .split("<kbd>")
                .skip(1)
                .filter_map(|rest| rest.split("</kbd>").next())
                .map(str::trim)
                .collect();
            if !keys.is_empty() {
                documented.push(keys.join("+"));
            }
        }
        assert!(
            documented.len() > 40,
            "found {} documented chords — the table parse broke, not the page",
            documented.len()
        );

        let bound: Vec<String> = crate::default_actions()
            .into_iter()
            .filter_map(|action| action.shortcut)
            .collect();

        for chord in &bound {
            assert!(
                documented.contains(chord),
                "{chord} ships but the published reference omits it — a feature \
                 nobody can find"
            );
        }
        for chord in &documented {
            assert!(
                bound.contains(chord),
                "the published reference lists {chord}, which nothing binds — \
                 the site tells people to press something that does nothing"
            );
        }
    }
    /// The degenerate ends of a marquee, which the pointer can reach.
    ///
    /// A zero-area drag is refused. A rect that runs past the canvas is kept
    /// whole, which is right — Photoshop lets you drag past the edge and
    /// intersects. A rect *entirely* outside is also kept, and that is
    /// QA-005: the shell then reports "pixel selection" for a selection
    /// covering no pixels.
    #[test]
    fn a_marquee_that_covers_nothing_is_refused() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let replace = |x, y, width, height| CommandArgs::SelectionReplace {
            shape: crate::SelectionShape::Rect,
            combine: crate::SelectionCombine::Replace,
            rect: crate::SelectionRect {
                x,
                y,
                width,
                height,
            },
            polygon: Vec::new(),
            label: "test".to_owned(),
        };

        assert!(
            session
                .invoke(command_id::SELECTION_REPLACE, replace(10, 10, 0, 0))
                .is_err(),
            "a zero-area drag selects nothing and must say so"
        );
        assert!(!session.selection.active);

        session
            .invoke(command_id::SELECTION_REPLACE, replace(10, 10, 1, 1))
            .expect("one pixel is a selection");
        assert!(session.selection.active);

        // Past the edge is kept whole and intersected downstream.
        session
            .invoke(command_id::SELECTION_REPLACE, replace(0, 0, 99_999, 99_999))
            .expect("a drag past the edge is a selection");
        assert_eq!(
            session.selection.bounds.map(|b| (b.width, b.height)),
            Some((99_999, 99_999)),
            "the rect is kept, not clamped"
        );
    }
}
