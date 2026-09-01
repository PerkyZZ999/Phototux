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
