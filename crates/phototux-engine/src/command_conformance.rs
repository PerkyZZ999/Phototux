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

    /// Blank out the monotonic counters in a serialised graph.
    ///
    /// `generation` and each layer's `revision` are meant to move forward, and
    /// undo bumps them again, so a perfectly restored document still
    /// serialises differently. `next_id` is the same shape for a different
    /// reason: layer ids are never recycled, so undoing a layer creation
    /// removes the layer and deliberately leaves the allocator where it was —
    /// reusing the id would let a stale reference alias a new layer.
    ///
    /// Written out rather than pulled in as a dependency — the engine takes
    /// serde and thiserror and nothing else, and this is three fields.
    fn without_counters(json: &str) -> String {
        const KEYS: [&str; 3] = ["\"generation\":", "\"revision\":", "\"next_id\":"];
        let mut out = String::with_capacity(json.len());
        let mut rest = json;
        loop {
            let Some((at, key)) = KEYS
                .iter()
                .filter_map(|k| rest.find(k).map(|at| (at, *k)))
                .min_by_key(|(at, _)| *at)
            else {
                out.push_str(rest);
                return out;
            };
            out.push_str(&rest[..at]);
            out.push_str(key);
            let tail = &rest[at + key.len()..];
            let digits = tail
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(tail.len());
            rest = &tail[digits..];
        }
    }

    /// Every action that edits the document has to be undoable, and undo has
    /// to put the document back.
    ///
    /// `document_edits_leave_an_undo_entry` asks the first half of this for the
    /// layer styles and the vector mask — thirteen cases picked by hand. This
    /// asks both halves of every action the shell actually offers: the menus,
    /// the command palette and the tool chrome all dispatch through
    /// `default_actions`, so walking that registry walks the mutating surface a
    /// user can reach.
    ///
    /// The comparison is the serialised graph rather than the generation
    /// counter, which only ever moves forward — undo bumps it too, so a
    /// generation that "returned" would prove nothing. A refusal is fine: some
    /// commands need a document state this fixture does not set up, and the
    /// registry test above already pins that their arguments are accepted.
    /// What is not allowed is editing the graph and leaving no way back.
    #[test]
    fn every_action_that_edits_the_document_undoes_back_to_where_it_started() {
        // The monotonic counters are excluded on purpose — see
        // `without_counters`. Everything that describes the document stays in.
        let snapshot = |session: &SessionState| {
            let json = serde_json::to_string(session.graph.as_ref().expect("a document"))
                .expect("graph json");
            without_counters(&json)
        };
        let mut checked = 0_usize;
        for action in crate::default_actions() {
            let Some(id) = action.command_id.as_deref() else {
                continue;
            };
            // History's own three, which are what does the undoing, and the
            // two that genuinely cannot be taken back yet. Soft-proof is view
            // chrome that happens to be stored in the graph — Photoshop's
            // Proof Colors is not undoable either — and Convert to Profile
            // rewrites every layer's pixels on the GPU behind a warning, with
            // no snapshot to come back to (QA-014). Named rather than skipped
            // by a `continue` on failure, so adding a command that forgets its
            // undo entry fails here instead of quietly joining them.
            const NOT_UNDOABLE: [&str; 5] = [
                command_id::HISTORY_UNDO,
                command_id::HISTORY_REDO,
                command_id::HISTORY_JUMP,
                command_id::DOCUMENT_SET_SOFT_PROOF,
                command_id::DOCUMENT_CONVERT_PROFILE,
            ];
            if NOT_UNDOABLE.contains(&id) {
                continue;
            }
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            // A second layer, so reorder, merge, group and clip have something
            // to act on rather than refusing on a one-layer document.
            let _ = session.invoke(command_id::LAYER_CREATE, CommandArgs::None);

            let Ok(args) = session.args_for_action(id, action.arg.as_deref()) else {
                continue;
            };
            let before = snapshot(&session);
            let undo_before = session.history.entries_undo().len();
            if session.invoke(id, args).is_err() {
                continue;
            }
            let after = snapshot(&session);
            if after == before {
                continue;
            }
            checked += 1;

            assert!(
                session.history.entries_undo().len() > undo_before,
                "{} ({id}) edited the graph without recording an undo entry",
                action.id
            );
            session
                .invoke(command_id::HISTORY_UNDO, CommandArgs::None)
                .unwrap_or_else(|e| panic!("{} ({id}) could not be undone: {e:?}", action.id));
            assert_eq!(
                snapshot(&session),
                before,
                "{} ({id}) does not undo back to the document it started from",
                action.id
            );
            session
                .invoke(command_id::HISTORY_REDO, CommandArgs::None)
                .unwrap_or_else(|e| panic!("{} ({id}) could not be redone: {e:?}", action.id));
            assert_eq!(
                snapshot(&session),
                after,
                "{} ({id}) does not redo back to the document undo took away",
                action.id
            );
        }
        assert!(
            checked >= 40,
            "only {checked} actions reached the graph — the registry scan broke, \
             not the wiring"
        );
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

        // Entirely beside the page is not a selection, however large the
        // rectangle is. "Empty" used to mean "empty rectangle", so a box
        // dragged into the letterbox reported `pixel selection` and every
        // command that needs one then ran and did nothing.
        let before = session.selection.bounds;
        for (x, y) in [(5_000, 5_000), (-4_000, 10), (10, -4_000), (1_280, 10)] {
            assert!(
                session
                    .invoke(command_id::SELECTION_REPLACE, replace(x, y, 10, 10))
                    .is_err(),
                "a marquee at ({x}, {y}) covers no document pixel and must say so"
            );
        }
        assert_eq!(
            session.selection.bounds, before,
            "a refused marquee must not disturb the selection that stands"
        );

        // One pixel of overlap is still a selection.
        session
            .invoke(command_id::SELECTION_REPLACE, replace(1_279, 719, 10, 10))
            .expect("a marquee that clips the corner is a selection");
    }
    /// Every command is on one side of the lock or the other.
    ///
    /// The partition is the point. A precondition written into thirty command
    /// bodies grows holes silently — that is how Lock All came to mean "cannot
    /// delete, cannot paint, cannot move" while opacity, blend mode and
    /// effects all went through. Adding a command now fails here until it is
    /// classified, and the two lists carry the reasoning next to the names.
    #[test]
    fn every_command_is_classified_against_the_lock() {
        for id in command_id::ALL {
            let changes = command_id::CHANGES_ACTIVE_LAYER.contains(id);
            let keeps = command_id::KEEPS_WORKING_WHEN_LOCKED.contains(id);
            assert!(
                changes || keeps,
                "{id} is in neither CHANGES_ACTIVE_LAYER nor \
                 KEEPS_WORKING_WHEN_LOCKED — decide whether a locked layer \
                 should stand in its way, and say why in the list you add it to"
            );
            assert!(!(changes && keeps), "{id} is in both lock lists");
        }
        for id in command_id::CHANGES_ACTIVE_LAYER
            .iter()
            .chain(command_id::KEEPS_WORKING_WHEN_LOCKED)
        {
            assert!(
                command_id::ALL.contains(id),
                "{id} is classified against the lock but is not a registered command"
            );
        }
    }

    /// A locked layer refuses every command that would change it.
    ///
    /// Drives the list rather than a handful of hand-picked commands, so the
    /// check cannot drift from the classification above.
    #[test]
    fn a_locked_layer_refuses_every_command_that_would_change_it() {
        for id in command_id::CHANGES_ACTIVE_LAYER {
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            session
                .invoke(
                    command_id::LAYER_SET_LOCKS,
                    CommandArgs::SetLocks {
                        pixels: false,
                        position: false,
                        all: true,
                        alpha: false,
                    },
                )
                .expect("lock all");

            let error = session
                .invoke(id, args_for(id))
                .expect_err("a locked layer must refuse this");
            assert!(
                matches!(error, CommandError::Rejected(why) if why.contains("locked")),
                "{id} was refused for the wrong reason: {error:?} — the lock \
                 must be the thing that stops it, not a missing argument"
            );
        }
    }

    /// Locking pixels or position does not lock the blend mode.
    ///
    /// The counterweight to the test above: an over-broad predicate that
    /// refused everything under any lock would pass it and would be wrong.
    /// Photoshop's Lock Pixels stops the brush and leaves opacity and blend
    /// editable, and Lock Position stops the move tool.
    #[test]
    fn the_narrow_locks_leave_a_layer_restylable() {
        for (pixels, position) in [(true, false), (false, true)] {
            let mut session = SessionState::default();
            session.apply_preset(SizePreset::P720);
            session
                .invoke(
                    command_id::LAYER_SET_LOCKS,
                    CommandArgs::SetLocks {
                        pixels,
                        position,
                        all: false,
                        alpha: false,
                    },
                )
                .expect("set locks");
            session
                .invoke(
                    command_id::LAYER_SET_OPACITY,
                    CommandArgs::SetOpacity { opacity: 0.5 },
                )
                .unwrap_or_else(|e| {
                    panic!("pixels={pixels} position={position} blocked opacity: {e:?}")
                });
        }
    }
    /// Lock All lets go of everything it took.
    ///
    /// The three toggles used to look identical whether their lock was on or
    /// off, which hid this: turning Lock All on set pixels and position too,
    /// through an `||` in the arguments the action built, and turning it off
    /// left them set. The layer stayed pinned and unpaintable with every
    /// button showing nothing.
    #[test]
    fn lock_all_releases_what_it_took() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let toggle_all = |session: &mut SessionState| {
            let args = session
                .args_for_action(command_id::LAYER_SET_LOCKS, Some("all"))
                .expect("lock args");
            session
                .invoke(command_id::LAYER_SET_LOCKS, args)
                .expect("set locks");
        };

        toggle_all(&mut session);
        assert!(
            session.active_layer_change_blocked(),
            "Lock All did not lock"
        );
        assert!(session.active_lock_pixels() && session.active_lock_position());

        toggle_all(&mut session);
        assert!(
            !session.active_layer_change_blocked()
                && !session.active_lock_pixels()
                && !session.active_lock_position(),
            "unlocking Lock All left the layer pinned or unpaintable"
        );
    }

    /// Turning one lock off releases Lock All with it.
    ///
    /// `paint_blocked` and `change_blocked` both fold `all` in, so a cleared
    /// pixel lock under a standing Lock All would go on blocking with nothing
    /// on screen saying which lock still held.
    #[test]
    fn clearing_one_lock_releases_lock_all() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        for arg in ["all", "pixels"] {
            let args = session
                .args_for_action(command_id::LAYER_SET_LOCKS, Some(arg))
                .expect("lock args");
            session
                .invoke(command_id::LAYER_SET_LOCKS, args)
                .expect("set locks");
        }
        assert!(!session.active_lock_pixels(), "the pixel lock is off");
        assert!(
            !session.active_layer_change_blocked(),
            "Lock All still holds after one of its locks was cleared"
        );
        assert!(
            session.active_lock_position(),
            "the position lock was not the one the user touched"
        );
    }
    /// The range a slider offers is the range the engine keeps.
    ///
    /// Two literal tables written independently — `editor_slots` for the UI,
    /// a `clamp` per arm for the engine — and nothing compared them. Three
    /// slots disagreed, so the engine could hold a gamma of 5 while its slider
    /// was pinned at 3, and the first touch of that slider changed the
    /// document without being asked to. `clamped` now reads the one table;
    /// this asserts it, slot by slot, for every adjustment that ships.
    #[test]
    fn every_adjustment_slot_keeps_exactly_the_range_its_editor_offers() {
        use crate::AdjustmentParams;
        let mut checked = 0;
        for kind in AdjustmentParams::ALL_KINDS {
            for (index, (label, low, high)) in kind.editor_slots().iter().enumerate() {
                checked += 1;
                let read_back = |value: f32| {
                    let mut slots = kind.slots();
                    slots[index] = value;
                    kind.with_slots(slots).clamped().slots()[index]
                };
                let under = read_back(low - 1_000.0);
                assert!(
                    (under - low).abs() < 1e-3,
                    "{} {label}: a value below the slider's {low} came back as \
                     {under}, so the engine keeps what the slider cannot show",
                    kind.kind_key()
                );
                let over = read_back(high + 1_000.0);
                assert!(
                    (over - high).abs() < 1e-3,
                    "{} {label}: a value above the slider's {high} came back as \
                     {over}, so the engine keeps what the slider cannot show",
                    kind.kind_key()
                );
            }
        }
        assert!(
            checked >= 20,
            "walked {checked} slots — the scan broke rather than the ranges"
        );
    }
    /// The click that creates a text or shape layer is where it lands.
    ///
    /// Both tools used to discard it. `CommandArgs::TextCreate` carried only
    /// the string and `cmd_text_create` built its content from
    /// `TextContent::default()`, so a click at the bottom-right of a 1080p
    /// document put the frame a thousand pixels away at the origin; the shape
    /// presets landed at their fraction of the document whatever the pointer
    /// said. Photoshop places both where the tool is clicked, and that is this
    /// project's placement rule.
    #[test]
    fn a_text_layer_lands_where_the_click_was() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P1080);
        session
            .invoke(
                command_id::TEXT_CREATE,
                CommandArgs::TextCreate {
                    text: "Hello".into(),
                    x: 1600.0,
                    y: 900.0,
                },
            )
            .expect("create text");
        let placed = session
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .map(|layer| (layer.transform.translate_x, layer.transform.translate_y))
            .expect("a text layer");
        assert_eq!(placed, (1600.0, 900.0));

        // A click in the letterbox still has to produce a frame on screen.
        session
            .invoke(
                command_id::TEXT_CREATE,
                CommandArgs::TextCreate {
                    text: "Edge".into(),
                    x: -4000.0,
                    y: 9000.0,
                },
            )
            .expect("create text");
        let clamped = session
            .graph
            .as_ref()
            .and_then(|g| g.active_id().and_then(|id| g.get(id)))
            .map(|layer| (layer.transform.translate_x, layer.transform.translate_y))
            .expect("a text layer");
        assert_eq!(
            clamped,
            (0.0, 1080.0),
            "a click outside the canvas must clamp into it, not place the frame there"
        );
    }

    /// Every shape preset centres on the point it was placed at.
    #[test]
    fn every_shape_preset_centres_on_the_click() {
        use crate::ShapePreset;
        for preset in ShapePreset::ALL {
            let placed = preset.content_at(1920, 1080, 1500.0, 800.0);
            let bounds = placed.path.bounds().expect("a preset draws something");
            let centre = (
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            );
            assert!(
                (centre.0 - 1500.0).abs() < 0.01 && (centre.1 - 800.0).abs() < 0.01,
                "{:?} centred at {centre:?} rather than the click",
                preset
            );
            // The preset without a click is untouched — the menu path has no
            // pointer to honour and must keep landing where it always did.
            let unplaced = preset.content(1920, 1080);
            assert_eq!(
                preset
                    .content_at(
                        1920,
                        1080,
                        unplaced
                            .path
                            .bounds()
                            .map(|b| b.x + b.width / 2.0)
                            .unwrap_or_default(),
                        unplaced
                            .path
                            .bounds()
                            .map(|b| b.y + b.height / 2.0)
                            .unwrap_or_default(),
                    )
                    .path,
                unplaced.path,
                "{preset:?} moved when placed at its own centre"
            );
        }
    }
}
