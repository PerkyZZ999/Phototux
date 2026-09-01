//! Command router tests (handbook 08).

use super::*;
// Types the parent no longer imports now that the vocabulary lives beside it.
use crate::SizePreset;
use crate::layer::ShapeContent;
use crate::selection::{SelectionCombine, SelectionRect};

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
    // Object selection must track the newly active layer (checklist §5.1).
    let active_name = s
        .layer_rows()
        .into_iter()
        .find(|row| row.active)
        .map(|row| row.name)
        .unwrap_or_default();
    assert_eq!(s.object_selection_names_joined(), active_name);
    assert!(
        s.status_summary()
            .contains(&format!("object: {active_name}"))
    );
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
fn multi_delete_is_atomic_one_undo() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("l1");
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("l2");
    let ids: Vec<_> = s
        .graph
        .as_ref()
        .unwrap()
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    // Keep bottom layer; delete the two above.
    s.set_object_selection(vec![ids[1], ids[2]]);
    let n = s.layer_count();
    s.invoke(command_id::LAYER_DELETE, CommandArgs::None)
        .expect("delete");
    assert_eq!(s.layer_count(), n - 2);
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo");
    assert_eq!(s.layer_count(), n);
}

#[test]
fn multi_delete_rejects_locked() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("l1");
    let n = s.layer_count();
    let ids: Vec<_> = s
        .graph
        .as_ref()
        .unwrap()
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.set_object_selection(vec![ids[0], ids[1]]);
    let _ = s.graph.as_mut().unwrap().get_mut(ids[1]).map(|l| {
        l.locks.all = true;
        l.locked = true;
    });
    let err = s
        .invoke(command_id::LAYER_DELETE, CommandArgs::None)
        .expect_err("locked");
    assert!(matches!(err, CommandError::Rejected(_)));
    assert_eq!(s.layer_count(), n);
}

#[test]
fn group_selection_reparents() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("l1");
    let ids: Vec<_> = s
        .graph
        .as_ref()
        .unwrap()
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.set_object_selection(ids.clone());
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    let g = s.graph.as_ref().unwrap();
    let group_id = g.active_id().unwrap();
    assert_eq!(g.get(group_id).unwrap().kind, LayerKind::Group);
    for id in &ids {
        assert_eq!(g.get(*id).unwrap().parent, Some(group_id));
    }
    s.invoke(command_id::LAYER_UNGROUP, CommandArgs::None)
        .expect("ungroup");
    let g = s.graph.as_ref().unwrap();
    assert!(g.get(group_id).is_none());
    for id in &ids {
        assert!(g.get(*id).unwrap().parent.is_none());
    }
}

#[test]
fn delete_clip_base_breaks_clip() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let ids = s.graph.as_ref().unwrap().stack_order();
    let base = ids[0];
    let top = ids[1];
    s.set_object_selection(vec![top]);
    s.invoke(
        command_id::LAYER_SET_CLIP,
        CommandArgs::LayerSetClip { clips: true },
    )
    .expect("clip");
    assert!(s.graph.as_ref().unwrap().get(top).unwrap().clips_to_below);
    s.set_object_selection(vec![base]);
    s.invoke(command_id::LAYER_DELETE, CommandArgs::None)
        .expect("delete base");
    assert!(s.graph.as_ref().unwrap().get(base).is_none());
    assert!(!s.graph.as_ref().unwrap().get(top).unwrap().clips_to_below);
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo");
    assert!(s.graph.as_ref().unwrap().get(base).is_some());
    assert!(s.graph.as_ref().unwrap().get(top).unwrap().clips_to_below);
}

#[test]
fn effect_reorder_preserves_ids() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(
        command_id::FILTER_ADD_EFFECT,
        CommandArgs::FilterEffect {
            kind: "gaussian".into(),
        },
    )
    .expect("blur1");
    s.invoke(
        command_id::FILTER_ADD_EFFECT,
        CommandArgs::FilterEffect {
            kind: "sharpen".into(),
        },
    )
    .expect("sharpen");
    s.invoke(
        command_id::FILTER_ADD_EFFECT,
        CommandArgs::FilterEffect {
            kind: "motion".into(),
        },
    )
    .expect("motion");
    let id = s.graph.as_ref().unwrap().active_id().unwrap();
    let effects = s.graph.as_ref().unwrap().get(id).unwrap().effects.clone();
    assert_eq!(effects.len(), 3);
    let mid = effects[1].id;
    s.invoke(
        command_id::EFFECT_REORDER,
        CommandArgs::EffectReorder {
            effect_id: mid,
            to_index: 0,
        },
    )
    .expect("reorder");
    let next = &s.graph.as_ref().unwrap().get(id).unwrap().effects;
    assert_eq!(next[0].id, mid);
    assert_eq!(next.len(), 3);
}

#[test]
fn create_fill_layer_via_command() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let n = s.layer_count();
    s.invoke(
        command_id::LAYER_CREATE_FILL,
        CommandArgs::FillCreate {
            color_rgba: [1.0, 0.0, 0.0, 1.0],
        },
    )
    .expect("fill");
    assert_eq!(s.layer_count(), n + 1);
    let id = s.graph.as_ref().unwrap().active_id().unwrap();
    let layer = s.graph.as_ref().unwrap().get(id).unwrap();
    assert_eq!(layer.kind, LayerKind::Fill);
    assert_eq!(layer.fill.as_ref().unwrap().color_rgba[0], 1.0);
    s.invoke(
        command_id::LAYER_SET_FILL_COLOR,
        CommandArgs::FillColor {
            color_rgba: [0.0, 1.0, 0.0, 1.0],
        },
    )
    .expect("recolor");
    let layer = s.graph.as_ref().unwrap().get(id).unwrap();
    assert_eq!(layer.fill.as_ref().unwrap().color_rgba[1], 1.0);
}

#[test]
fn multi_reorder_atomic() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("a");
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("b");
    let ids: Vec<_> = s.graph.as_ref().unwrap().stack_order();
    assert!(ids.len() >= 3);
    // Select bottom two; move above the remaining top layer.
    s.set_object_selection(vec![ids[0], ids[1]]);
    s.invoke(
        command_id::LAYER_REORDER,
        CommandArgs::Reorder { to_index: 1 },
    )
    .expect("reorder");
    let next = s.graph.as_ref().unwrap().stack_order();
    assert_eq!(next.len(), ids.len());
    assert_eq!(next[1], ids[0]);
    assert_eq!(next[2], ids[1]);
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

#[test]
fn document_set_icc_embeds_and_clears() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let icc = crate::minimal_icc_fixture();
    s.invoke(
        command_id::DOCUMENT_SET_ICC,
        CommandArgs::SetIcc {
            bytes: Some(icc.clone()),
        },
    )
    .expect("embed");
    assert_eq!(
        s.graph.as_ref().unwrap().color.embedded_icc.as_ref(),
        Some(&icc)
    );
    let err = s
        .invoke(
            command_id::DOCUMENT_SET_ICC,
            CommandArgs::SetIcc {
                bytes: Some(vec![1, 2, 3]),
            },
        )
        .expect_err("bad");
    assert!(matches!(err, CommandError::Rejected(_)));
    s.invoke(
        command_id::DOCUMENT_SET_ICC,
        CommandArgs::SetIcc { bytes: None },
    )
    .expect("clear");
    assert!(s.graph.as_ref().unwrap().color.embedded_icc.is_none());
}

#[test]
fn filter_preview_does_not_dirty_until_commit() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.mark_persisted(s.document_generation());
    assert!(!s.is_dirty_vs_persisted());
    let generation_before = s.document_generation();
    let effects = s
        .invoke(
            command_id::FILTER_PREVIEW,
            CommandArgs::FilterPreview {
                kind: "gaussian".into(),
            },
        )
        .expect("preview");
    assert!(!effects.dirty);
    assert_eq!(s.document_generation(), generation_before);
    assert!(!s.is_dirty_vs_persisted());
    let id = s.graph.as_ref().unwrap().active_id().unwrap();
    assert!(
        s.graph
            .as_ref()
            .unwrap()
            .get(id)
            .unwrap()
            .effects
            .is_empty()
    );
    s.invoke(
        command_id::FILTER_SET_PREVIEW_PARAMS,
        CommandArgs::FilterPreviewParams {
            p0: 6.0,
            p1: 0.0,
            p2: 0.0,
        },
    )
    .expect("params");
    assert!(!s.is_dirty_vs_persisted());
    s.invoke(command_id::FILTER_COMMIT, CommandArgs::None)
        .expect("commit");
    assert!(s.is_dirty_vs_persisted());
    let layer = s.graph.as_ref().unwrap().get(id).unwrap();
    assert_eq!(layer.effects.len(), 1);
    assert_eq!(layer.filter_plan.nodes.len(), 1);
    assert_eq!(layer.filter_plan.nodes[0].kind, "gaussian");
}

#[test]
fn filter_cancel_clears_preview_without_effects() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.mark_persisted(s.document_generation());
    s.invoke(
        command_id::FILTER_PREVIEW,
        CommandArgs::FilterPreview {
            kind: "sharpen".into(),
        },
    )
    .expect("preview");
    s.invoke(command_id::FILTER_CANCEL_PREVIEW, CommandArgs::None)
        .expect("cancel");
    assert!(s.filter_preview.is_none());
    assert!(!s.is_dirty_vs_persisted());
    let id = s.graph.as_ref().unwrap().active_id().unwrap();
    assert!(
        s.graph
            .as_ref()
            .unwrap()
            .get(id)
            .unwrap()
            .effects
            .is_empty()
    );
}

#[test]
fn filter_commit_rejects_stale_generation() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(
        command_id::FILTER_PREVIEW,
        CommandArgs::FilterPreview {
            kind: "motion".into(),
        },
    )
    .expect("preview");
    // Advance authority under the preview.
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("mutate");
    let err = s
        .invoke(command_id::FILTER_COMMIT, CommandArgs::None)
        .expect_err("stale");
    assert!(matches!(err, CommandError::Rejected(msg) if msg.contains("stale")));
    assert!(s.filter_preview.is_none());
}

#[test]
fn filter_commit_rejects_cancelled_token() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(
        command_id::FILTER_PREVIEW,
        CommandArgs::FilterPreview {
            kind: "emboss".into(),
        },
    )
    .expect("preview");
    s.filter_preview.as_ref().expect("session").cancel.cancel();
    let err = s
        .invoke(command_id::FILTER_COMMIT, CommandArgs::None)
        .expect_err("cancelled");
    assert!(matches!(err, CommandError::Rejected(msg) if msg.contains("cancelled")));
}

#[test]
fn drop_shadow_rejects_shape_layer() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let path = crate::paths::rect_path("R", 10.0, 10.0, 40.0, 30.0);
    s.invoke(
        command_id::SHAPE_CREATE,
        CommandArgs::ShapeCreate {
            content: Box::new(ShapeContent {
                path,
                ..ShapeContent::default()
            }),
        },
    )
    .expect("shape");
    let err = s
        .invoke(
            command_id::STYLE_ADD,
            CommandArgs::LayerStyleKind {
                kind: "drop-shadow".into(),
            },
        )
        .expect_err("shape drop shadow");
    assert!(
        matches!(err, CommandError::Rejected(msg) if msg.contains("raster")),
        "{err}"
    );
}

#[test]
fn path_edit_round_trip_on_shape() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let path = crate::paths::rect_path("R", 10.0, 10.0, 40.0, 30.0);
    s.invoke(
        command_id::SHAPE_CREATE,
        CommandArgs::ShapeCreate {
            content: Box::new(ShapeContent {
                path,
                ..ShapeContent::default()
            }),
        },
    )
    .expect("shape");
    s.invoke(
        command_id::PATH_MOVE_ANCHOR,
        CommandArgs::PathMoveAnchor {
            index: 0,
            x: 5.0,
            y: 7.0,
        },
    )
    .expect("move");
    s.invoke(
        command_id::PATH_ADD_ANCHOR,
        CommandArgs::PathAddAnchor {
            x: 20.0,
            y: 20.0,
            index: Some(1),
        },
    )
    .expect("add");
    s.invoke(
        command_id::PATH_SET_CLOSED,
        CommandArgs::PathSetClosed { closed: false },
    )
    .expect("open");
    let id = s.graph.as_ref().unwrap().active_id().unwrap();
    let shape = s
        .graph
        .as_ref()
        .unwrap()
        .get(id)
        .unwrap()
        .shape
        .as_ref()
        .unwrap();
    assert!(!shape.path.closed);
    assert_eq!(shape.path.anchors.len(), 5);
    assert!((shape.path.anchors[0].x - 5.0).abs() < f32::EPSILON);
    s.invoke(
        command_id::PATH_DELETE_ANCHOR,
        CommandArgs::PathDeleteAnchor { index: 1 },
    )
    .expect("delete");
    assert_eq!(
        s.graph
            .as_ref()
            .unwrap()
            .get(id)
            .unwrap()
            .shape
            .as_ref()
            .unwrap()
            .path
            .anchors
            .len(),
        4
    );
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo delete");
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo open");
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo add");
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo move");
    let restored = &s
        .graph
        .as_ref()
        .unwrap()
        .get(id)
        .unwrap()
        .shape
        .as_ref()
        .unwrap()
        .path;
    assert!(restored.closed);
    assert_eq!(restored.anchors.len(), 4);
    assert!((restored.anchors[0].x - 10.0).abs() < f32::EPSILON);
}

// —— Align and distribute ——

/// A document with `count` extra layers, and the ids of every layer in it.
fn session_with_layers(count: usize) -> (SessionState, Vec<LayerId>) {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    for _ in 0..count {
        s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
            .expect("create layer");
    }
    let ids = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    (s, ids)
}

fn target(id: LayerId, x: f32, y: f32, w: f32, h: f32) -> crate::AlignTarget {
    crate::AlignTarget::single(id, crate::Rect::new(x, y, w, h))
}

fn translation(s: &SessionState, id: LayerId) -> (f32, f32) {
    let t = s
        .graph
        .as_ref()
        .expect("graph")
        .get(id)
        .expect("layer")
        .transform;
    (t.translate_x, t.translate_y)
}

#[test]
fn aligning_writes_translations_and_one_undo_puts_them_all_back() {
    let (mut s, ids) = session_with_layers(2);
    let targets = vec![
        target(ids[0], 100.0, 0.0, 50.0, 50.0),
        target(ids[1], 300.0, 0.0, 50.0, 50.0),
        target(ids[2], 220.0, 0.0, 50.0, 50.0),
    ];
    s.invoke(
        command_id::LAYER_ALIGN,
        CommandArgs::AlignLayers {
            op: crate::AlignOp::Left,
            targets,
        },
    )
    .expect("align");
    assert_eq!(
        translation(&s, ids[0]),
        (0.0, 0.0),
        "leftmost must not move"
    );
    assert_eq!(translation(&s, ids[1]), (-200.0, 0.0));
    assert_eq!(translation(&s, ids[2]), (-120.0, 0.0));

    // The whole alignment is one history entry, not one per layer.
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo");
    for id in &ids {
        assert_eq!(translation(&s, *id), (0.0, 0.0), "undo left a layer moved");
    }
}

#[test]
fn aligning_adds_to_a_translation_the_layer_already_had() {
    // Offsets are deltas; treating them as absolute positions would throw away
    // whatever the transform gizmo had already put on the layer.
    let (mut s, ids) = session_with_layers(1);
    let graph = s.graph.as_mut().expect("graph");
    graph.set_transform(
        ids[1],
        LayerTransform {
            translate_x: 40.0,
            translate_y: 7.0,
            ..LayerTransform::identity()
        },
    );
    s.invoke(
        command_id::LAYER_ALIGN,
        CommandArgs::AlignLayers {
            op: crate::AlignOp::Left,
            targets: vec![
                target(ids[0], 10.0, 0.0, 20.0, 20.0),
                target(ids[1], 60.0, 0.0, 20.0, 20.0),
            ],
        },
    )
    .expect("align");
    assert_eq!(translation(&s, ids[1]), (-10.0, 7.0));
}

#[test]
fn a_group_moves_as_one_object_and_keeps_its_internal_arrangement() {
    // The compositor does not pass a group's transform to its children, so a
    // group can only move by moving every member the same amount. Members are
    // given one shared box for exactly that reason.
    let (mut s, ids) = session_with_layers(2);
    let group = crate::AlignTarget {
        bounds: crate::Rect::new(300.0, 0.0, 80.0, 40.0),
        members: vec![ids[1], ids[2]],
    };
    s.invoke(
        command_id::LAYER_ALIGN,
        CommandArgs::AlignLayers {
            op: crate::AlignOp::Left,
            targets: vec![target(ids[0], 100.0, 0.0, 50.0, 50.0), group],
        },
    )
    .expect("align");
    assert_eq!(translation(&s, ids[1]), (-200.0, 0.0));
    assert_eq!(
        translation(&s, ids[2]),
        (-200.0, 0.0),
        "both members must move by the same amount"
    );
}

#[test]
fn aligning_already_aligned_layers_is_refused_rather_than_stacking_a_no_op() {
    let (mut s, ids) = session_with_layers(1);
    let targets = vec![
        target(ids[0], 40.0, 0.0, 10.0, 10.0),
        target(ids[1], 40.0, 0.0, 10.0, 10.0),
    ];
    let err = s
        .invoke(
            command_id::LAYER_ALIGN,
            CommandArgs::AlignLayers {
                op: crate::AlignOp::Left,
                targets,
            },
        )
        .expect_err("an alignment that moves nothing must not reach history");
    assert!(matches!(err, CommandError::Rejected(_)), "{err:?}");
}

#[test]
fn distributing_two_layers_is_refused_before_it_touches_the_document() {
    let (mut s, ids) = session_with_layers(1);
    let err = s
        .invoke(
            command_id::LAYER_ALIGN,
            CommandArgs::AlignLayers {
                op: crate::AlignOp::DistributeHorizontal,
                targets: vec![
                    target(ids[0], 0.0, 0.0, 10.0, 10.0),
                    target(ids[1], 90.0, 0.0, 10.0, 10.0),
                ],
            },
        )
        .expect_err("distribute needs three");
    assert!(matches!(err, CommandError::Rejected(_)), "{err:?}");
}

#[test]
fn a_position_locked_layer_blocks_the_whole_alignment() {
    // All-or-nothing: a partial alignment reads as a broken alignment, and the
    // layer left behind is the one the user is least likely to be watching.
    let (mut s, ids) = session_with_layers(1);
    if let Some(layer) = s.graph.as_mut().and_then(|g| g.get_mut(ids[1])) {
        layer.locks.position = true;
    }
    let err = s
        .invoke(
            command_id::LAYER_ALIGN,
            CommandArgs::AlignLayers {
                op: crate::AlignOp::Left,
                targets: vec![
                    target(ids[0], 100.0, 0.0, 10.0, 10.0),
                    target(ids[1], 300.0, 0.0, 10.0, 10.0),
                ],
            },
        )
        .expect_err("a locked position must refuse the command");
    assert!(matches!(err, CommandError::Rejected(_)), "{err:?}");
    assert_eq!(
        translation(&s, ids[0]),
        (0.0, 0.0),
        "nothing may have moved"
    );
}

#[test]
fn a_single_layer_aligns_to_the_canvas() {
    // The one-layer case only does anything because the frame falls back to the
    // document; aligning a layer to its own bounding box is a no-op.
    let (mut s, ids) = session_with_layers(0);
    s.invoke(
        command_id::LAYER_ALIGN,
        CommandArgs::AlignLayers {
            op: crate::AlignOp::HorizontalCenter,
            targets: vec![target(ids[0], 0.0, 0.0, 200.0, 100.0)],
        },
    )
    .expect("align to canvas");
    let (dx, _) = translation(&s, ids[0]);
    assert!((dx - (1280.0 - 200.0) / 2.0).abs() < 0.5, "dx was {dx}");
}

// —— Blend If ——

fn blend_if_of(s: &SessionState) -> crate::BlendIf {
    let graph = s.graph.as_ref().expect("graph");
    graph
        .get(graph.active_id().expect("active"))
        .expect("layer")
        .blend_if
}

#[test]
fn setting_blend_ranges_records_one_undo_entry_per_gesture() {
    let (mut s, _) = session_with_layers(0);
    let mut wanted = crate::BlendIf {
        channel: crate::BlendIfChannel::Red,
        this_layer: crate::BlendRange::from_stops([0.1, 0.2, 0.9, 1.0]),
        underlying: crate::BlendRange::FULL,
    };
    s.invoke(
        command_id::LAYER_SET_BLEND_IF,
        CommandArgs::SetBlendIf { blend_if: wanted },
    )
    .expect("set blend if");
    assert_eq!(blend_if_of(&s), wanted);

    // A drag arrives as a run of commands. They must fold into the entry
    // already on the stack, or one drag fills the timeline.
    let before = s.history.rows_newest_first().len();
    for step in 1..=5 {
        wanted.this_layer.black_end = 0.2 + step as f32 * 0.05;
        s.invoke(
            command_id::LAYER_SET_BLEND_IF,
            CommandArgs::SetBlendIf { blend_if: wanted },
        )
        .expect("drag step");
    }
    assert_eq!(
        s.history.rows_newest_first().len(),
        before,
        "a slider drag stacked one history entry per step"
    );

    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo");
    assert!(
        blend_if_of(&s).is_identity(),
        "one undo must return to the ranges the gesture started from"
    );
}

#[test]
fn out_of_order_stops_are_sorted_before_they_reach_the_document() {
    // The panel has four handles per range and does not stop them crossing;
    // a stored range that inverts itself would hide the whole layer.
    let (mut s, _) = session_with_layers(0);
    s.invoke(
        command_id::LAYER_SET_BLEND_IF,
        CommandArgs::SetBlendIf {
            blend_if: crate::BlendIf {
                channel: crate::BlendIfChannel::Gray,
                this_layer: crate::BlendRange::from_stops([0.9, 0.7, 0.3, 0.1]),
                underlying: crate::BlendRange::FULL,
            },
        },
    )
    .expect("set blend if");
    assert_eq!(
        blend_if_of(&s).this_layer.stops(),
        [0.1, 0.3, 0.7, 0.9],
        "the stops reached the layer out of order"
    );
}

#[test]
fn setting_the_ranges_a_layer_already_has_is_refused() {
    let (mut s, _) = session_with_layers(0);
    let err = s
        .invoke(
            command_id::LAYER_SET_BLEND_IF,
            CommandArgs::SetBlendIf {
                blend_if: crate::BlendIf::default(),
            },
        )
        .expect_err("an unchanged edit must not reach history");
    assert!(matches!(err, CommandError::Rejected(_)), "{err:?}");
}

#[test]
fn blend_ranges_survive_a_document_round_trip() {
    // The field is `#[serde(default)]` so older documents still open; that
    // also means a typo in the field name would round-trip as the default
    // rather than failing, which is why this asserts the value came back.
    let (mut s, _) = session_with_layers(0);
    let wanted = crate::BlendIf {
        channel: crate::BlendIfChannel::Blue,
        this_layer: crate::BlendRange::from_stops([0.0, 0.25, 0.5, 0.75]),
        underlying: crate::BlendRange::from_stops([0.1, 0.1, 1.0, 1.0]),
    };
    s.invoke(
        command_id::LAYER_SET_BLEND_IF,
        CommandArgs::SetBlendIf { blend_if: wanted },
    )
    .expect("set blend if");
    let graph = s.graph.as_ref().expect("graph");
    let json = serde_json::to_string(graph).expect("serialize");
    let back: crate::DocumentGraph = serde_json::from_str(&json).expect("deserialize");
    let id = graph.active_id().expect("active");
    assert_eq!(back.get(id).expect("layer").blend_if, wanted);
}

#[test]
fn a_layer_written_before_blend_if_existed_opens_with_no_ranges() {
    let (s, _) = session_with_layers(0);
    let graph = s.graph.as_ref().expect("graph");
    let id = graph.active_id().expect("active");
    let layer = graph.get(id).expect("layer");
    let mut value = serde_json::to_value(layer).expect("serialize layer");
    // Strip the field the way a document from an older build would lack it.
    value
        .as_object_mut()
        .expect("layer object")
        .remove("blend_if")
        .expect("the field must be there to strip");
    let back: crate::Layer = serde_json::from_value(value).expect("deserialize");
    assert!(
        back.blend_if.is_identity(),
        "a layer with no blend ranges must open hiding nothing"
    );
}
