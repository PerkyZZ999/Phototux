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

/// A shape layer with a document open and a rectangle on it.
fn session_with_a_shape() -> SessionState {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(
        command_id::SHAPE_CREATE,
        CommandArgs::ShapeCreate {
            content: Box::new(ShapeContent {
                path: crate::paths::rect_path("R", 10.0, 10.0, 40.0, 30.0),
                ..ShapeContent::default()
            }),
        },
    )
    .expect("shape");
    s
}

fn active_shape(s: &SessionState) -> ShapeContent {
    let graph = s.graph.as_ref().expect("graph");
    let id = graph.active_id().expect("active layer");
    graph
        .get(id)
        .and_then(|l| l.shape.clone())
        .expect("shape payload")
}

#[test]
fn recolouring_a_shape_leaves_its_geometry_alone_and_undoes() {
    let mut s = session_with_a_shape();
    let before = active_shape(&s);
    let wanted = crate::ShapeAppearance {
        fill_rgba: [1.0, 0.0, 0.0, 1.0],
        stroke_rgba: [0.0, 1.0, 0.0, 1.0],
        stroke_width: 9.0,
        filled: true,
        stroked: false,
    };
    s.invoke(
        command_id::SHAPE_SET_APPEARANCE,
        CommandArgs::ShapeSetAppearance { appearance: wanted },
    )
    .expect("recolour");
    let after = active_shape(&s);
    assert_eq!(after.appearance(), wanted);
    assert_eq!(after.path, before.path, "recolouring moved the geometry");

    // Undo is the history service walking the graph back; `SessionState::undo`
    // is the accessor that hands it over.
    let SessionState { graph, history, .. } = &mut s;
    history.undo_next(graph.as_mut().expect("graph"));
    assert_eq!(
        active_shape(&s).appearance(),
        before.appearance(),
        "undo did not restore the appearance"
    );
}

#[test]
fn recolouring_a_shape_to_what_it_already_is_is_refused() {
    // Otherwise every slider release while nothing moves would push a history
    // entry, and the timeline would fill with edits that changed nothing.
    let mut s = session_with_a_shape();
    let same = active_shape(&s).appearance();
    let error = s
        .invoke(
            command_id::SHAPE_SET_APPEARANCE,
            CommandArgs::ShapeSetAppearance { appearance: same },
        )
        .expect_err("no-op recolour");
    assert!(error.is_user_correctable(), "{error:?}");
}

#[test]
fn only_a_shape_layer_can_be_recoloured_this_way() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let error = s
        .invoke(
            command_id::SHAPE_SET_APPEARANCE,
            CommandArgs::ShapeSetAppearance {
                appearance: ShapeContent::default().appearance(),
            },
        )
        .expect_err("raster layer recolour");
    assert!(error.is_user_correctable(), "{error:?}");
}

#[test]
fn an_out_of_range_appearance_is_clamped_rather_than_refused() {
    let mut s = session_with_a_shape();
    s.invoke(
        command_id::SHAPE_SET_APPEARANCE,
        CommandArgs::ShapeSetAppearance {
            appearance: crate::ShapeAppearance {
                stroke_width: 1e9,
                ..active_shape(&s).appearance()
            },
        },
    )
    .expect("clamped recolour");
    assert!(
        (active_shape(&s).stroke_width - crate::ShapeAppearance::MAX_STROKE_WIDTH).abs()
            < f32::EPSILON
    );
}

/// A document whose active layer is a smart object wrapping 64×64 pixels.
fn session_with_a_smart_object() -> SessionState {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(
        command_id::SMART_CREATE,
        CommandArgs::SmartCreate {
            content: Box::new(crate::SmartObjectContent::embedded("Layer 1", 64, 64)),
        },
    )
    .expect("wrap");
    s
}

fn active_layer(s: &SessionState) -> crate::Layer {
    let graph = s.graph.as_ref().expect("graph");
    let id = graph.active_id().expect("active layer");
    graph.get(id).cloned().expect("layer")
}

#[test]
fn wrapping_a_layer_sets_the_kind_and_the_payload_together() {
    let s = session_with_a_smart_object();
    let layer = active_layer(&s);
    assert_eq!(layer.kind, crate::LayerKind::SmartObject);
    let smart = layer.smart.as_ref().expect("payload");
    assert_eq!(smart.source_width, 64);
    assert!(!smart.is_placed(), "a fresh wrap sits where it already was");
}

/// The pair must move as one. A graph with the kind undone but the payload
/// left behind is a raster layer carrying a source; the other way round is a
/// smart object with none, and every reader of `smart` would have to guess.
#[test]
fn undoing_a_wrap_restores_the_kind_and_the_payload_together() {
    let mut s = session_with_a_smart_object();
    let SessionState { graph, history, .. } = &mut s;
    history.undo_next(graph.as_mut().expect("graph"));
    let layer = active_layer(&s);
    assert_eq!(layer.kind, crate::LayerKind::Raster);
    assert!(layer.smart.is_none());
}

#[test]
fn a_placement_replaces_the_last_one_rather_than_composing_with_it() {
    // The whole point of the kind: two scale-downs in a row must leave the
    // second one's factor, not their product, because the host re-applies the
    // placement to the pristine source every time.
    let mut s = session_with_a_smart_object();
    for scale in [0.5_f32, 0.25] {
        s.invoke(
            command_id::SMART_SET_PLACEMENT,
            CommandArgs::SmartSetPlacement {
                placement: crate::LayerTransform {
                    scale_x: scale,
                    scale_y: scale,
                    ..Default::default()
                },
            },
        )
        .expect("place");
    }
    let placement = active_layer(&s).smart.expect("payload").placement;
    assert!(
        (placement.scale_x - 0.25).abs() < f32::EPSILON,
        "{placement:?}"
    );
}

#[test]
fn placing_a_smart_object_asks_the_host_to_re_render_it() {
    let mut s = session_with_a_smart_object();
    let id = s.graph.as_ref().unwrap().active_id().unwrap();
    let effects = s
        .invoke(
            command_id::SMART_SET_PLACEMENT,
            CommandArgs::SmartSetPlacement {
                placement: crate::LayerTransform {
                    rotation_deg: 30.0,
                    ..Default::default()
                },
            },
        )
        .expect("place");
    assert!(matches!(
        effects.host_follow_up,
        crate::HostFollowUp::PlaceSmartObject { id: got } if got == id
    ));
}

#[test]
fn placing_it_where_it_already_is_is_refused() {
    let mut s = session_with_a_smart_object();
    let error = s
        .invoke(
            command_id::SMART_SET_PLACEMENT,
            CommandArgs::SmartSetPlacement {
                placement: crate::LayerTransform::default(),
            },
        )
        .expect_err("no-op placement");
    assert!(error.is_user_correctable(), "{error:?}");
}

#[test]
fn only_a_pixel_layer_can_be_wrapped() {
    let mut s = session_with_a_smart_object();
    // Already wrapped.
    let error = s
        .invoke(
            command_id::SMART_CREATE,
            CommandArgs::SmartCreate {
                content: Box::new(crate::SmartObjectContent::embedded("x", 8, 8)),
            },
        )
        .expect_err("double wrap");
    assert!(error.is_user_correctable(), "{error:?}");

    // A shape describes itself rather than owning pixels, so there is nothing
    // to capture and the command says so instead of wrapping an empty source.
    let mut s = session_with_a_shape();
    let error = s
        .invoke(
            command_id::SMART_CREATE,
            CommandArgs::SmartCreate {
                content: Box::new(crate::SmartObjectContent::embedded("x", 8, 8)),
            },
        )
        .expect_err("wrap a shape");
    assert!(error.is_user_correctable(), "{error:?}");
}

#[test]
fn rasterizing_a_smart_object_drops_the_source_and_undoes() {
    let mut s = session_with_a_smart_object();
    s.invoke(command_id::SMART_RASTERIZE, CommandArgs::None)
        .expect("rasterize");
    let layer = active_layer(&s);
    assert_eq!(layer.kind, crate::LayerKind::Raster);
    assert!(layer.smart.is_none());

    let SessionState { graph, history, .. } = &mut s;
    history.undo_next(graph.as_mut().expect("graph"));
    let layer = active_layer(&s);
    assert_eq!(layer.kind, crate::LayerKind::SmartObject);
    assert!(layer.smart.is_some(), "undo lost the source");
}

#[test]
fn rasterizing_something_that_is_not_a_smart_object_is_refused() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let error = s
        .invoke(command_id::SMART_RASTERIZE, CommandArgs::None)
        .expect_err("raster layer");
    assert!(error.is_user_correctable(), "{error:?}");
}

/// Rasterizing a shape discards the only copy of its editable path, so it has
/// to be recoverable. It used to write the kind and the payload straight onto
/// the graph and push nothing.
#[test]
fn rasterizing_a_shape_undoes() {
    let mut s = session_with_a_shape();
    let before = active_shape(&s);
    s.invoke(command_id::SHAPE_RASTERIZE, CommandArgs::None)
        .expect("rasterize");
    let layer = active_layer(&s);
    assert_eq!(layer.kind, crate::LayerKind::Raster);
    assert!(layer.shape.is_none());
    assert!(
        layer.asset_key.is_some(),
        "a rasterized shape needs somewhere to keep its pixels"
    );

    let SessionState { graph, history, .. } = &mut s;
    history.undo_next(graph.as_mut().expect("graph"));
    let layer = active_layer(&s);
    assert_eq!(layer.kind, crate::LayerKind::Shape);
    assert_eq!(
        layer.shape.as_ref().map(|c| &c.path),
        Some(&before.path),
        "undo did not bring the geometry back"
    );
}

#[test]
fn rasterizing_something_that_is_not_a_shape_is_refused() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let error = s
        .invoke(command_id::SHAPE_RASTERIZE, CommandArgs::None)
        .expect_err("raster layer");
    assert!(error.is_user_correctable(), "{error:?}");
}

/// Every command that converts a layer to pixels must be undoable.
///
/// Each of these discards the only copy of something the user cannot recreate:
/// a text layer's words, a shape's editable path, a smart object's original
/// pixels. Three of them shipped writing the kind and the payload straight
/// onto the graph and pushing no history entry at all, which made the edit
/// permanent — one of them had even been written into the panel's help text as
/// though it were the design.
///
/// A table rather than three tests, so a fourth conversion has somewhere
/// obvious to be added and fails here if it forgets.
#[test]
fn every_conversion_to_pixels_can_be_undone() {
    /// How to build a document whose active layer the command applies to.
    type Fixture = fn() -> SessionState;

    let cases: [(&str, Fixture); 3] = [
        (command_id::TEXT_BAKE, || {
            let mut s = SessionState::default();
            s.apply_preset(SizePreset::P720);
            s.invoke(
                command_id::TEXT_CREATE,
                CommandArgs::TextCreate {
                    text: "Hello".into(),
                },
            )
            .expect("text layer");
            s
        }),
        (command_id::SHAPE_RASTERIZE, session_with_a_shape),
        (command_id::SMART_RASTERIZE, session_with_a_smart_object),
    ];

    for (command, build) in cases {
        let mut s = build();
        let before = active_layer(&s);
        assert_ne!(
            before.kind,
            crate::LayerKind::Raster,
            "{command}: the fixture is already a pixel layer"
        );
        s.invoke(command, CommandArgs::None)
            .unwrap_or_else(|e| panic!("{command}: {e:?}"));

        let after = active_layer(&s);
        assert_eq!(
            after.kind,
            crate::LayerKind::Raster,
            "{command} did not convert the layer"
        );
        assert!(
            after.asset_key.is_some(),
            "{command} left the layer with nowhere to keep its pixels"
        );

        let SessionState { graph, history, .. } = &mut s;
        assert!(
            history.undo_next(graph.as_mut().expect("graph")).is_some(),
            "{command} pushed no history entry — the conversion is permanent"
        );
        let restored = active_layer(&s);
        assert_eq!(restored.kind, before.kind, "{command}: kind not restored");
        assert_eq!(
            (
                restored.text.is_some(),
                restored.shape.is_some(),
                restored.smart.is_some()
            ),
            (
                before.text.is_some(),
                before.shape.is_some(),
                before.smart.is_some()
            ),
            "{command}: the payload did not come back with the kind"
        );
    }
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

// —— Error presentation ——

#[test]
fn a_rejection_reads_as_a_sentence_not_as_a_log_line() {
    let err = CommandError::Rejected("select a layer first");
    assert_eq!(err.user_message(), "Select a layer first.");
    // `Display` keeps the developer framing for logs and for `Debug` output;
    // only the user-facing rendering drops it.
    assert!(err.to_string().starts_with("command rejected:"));
}

#[test]
fn user_facing_text_never_leaks_the_internal_framing() {
    // Every reason the command layer can produce, run through the presentation
    // it will actually get. A reason written as a log fragment reaches the
    // status bar verbatim, so this is the only place it can be caught.
    for reason in [
        "select a layer first",
        "those layers are already aligned",
        "this layer is locked — unlock it to change it",
    ] {
        let text = CommandError::Rejected(reason).user_message();
        let lower = text.to_ascii_lowercase();
        for banned in ["command rejected", "invalid argument", "failed", "err"] {
            assert!(
                !lower.contains(banned),
                "{text:?} contains developer vocabulary {banned:?}"
            );
        }
        assert!(
            text.chars().next().is_some_and(char::is_uppercase),
            "{text:?} does not start with a capital"
        );
        assert!(text.ends_with('.'), "{text:?} does not end in a full stop");
    }
}

#[test]
fn a_wiring_fault_is_not_offered_to_the_user_as_advice() {
    // The split this classification exists for: an unknown command id and a
    // broken document invariant have nothing to tell the person at the
    // keyboard, and must not land in the status bar as though they did.
    assert!(!CommandError::Unknown("layer.nope".into()).is_user_correctable());
    assert!(
        !CommandError::Document(crate::DocumentError::LayerMissingAfterAdd).is_user_correctable()
    );
    assert!(CommandError::Rejected("select a layer first").is_user_correctable());
    assert!(CommandError::InvalidArgument("expected opacity").is_user_correctable());
    // A full document is the user's problem to solve, and the message says how.
    assert!(CommandError::Document(crate::DocumentError::layer_limit(16)).is_user_correctable());
    assert!(CommandError::Document(crate::DocumentError::NoDocument).is_user_correctable());
}

#[test]
fn a_message_that_already_ends_in_punctuation_is_left_alone() {
    assert_eq!(
        CommandError::Rejected("really?").user_message(),
        "Really?",
        "a second full stop would be added by a naive append"
    );
}

#[test]
fn every_rejection_the_command_layer_produces_survives_presentation() {
    // Sweeping the shipped registry rather than a hand-picked list: a reason
    // added later gets the same treatment without anyone remembering to add it
    // here. Only the shape is asserted — the wording is a judgement call, but
    // "starts with a capital and ends in a stop" is not.
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let mut seen = 0;
    for id in command_id::ALL {
        // Most commands need arguments; the ones that reject on `None` are
        // exactly the ones worth checking, and the rest simply do not fire.
        if let Err(error) = s.invoke(id, CommandArgs::None) {
            let text = error.user_message();
            assert!(!text.is_empty(), "{id} produced an empty message");
            assert!(
                text.chars().next().is_some_and(char::is_uppercase),
                "{id}: {text:?} does not start with a capital"
            );
            seen += 1;
        }
    }
    assert!(
        seen > 5,
        "only {seen} commands rejected — the sweep is not running"
    );
}

/// Flatten leaves exactly one layer, and undo puts the stack back.
///
/// The graph half only — the pixels are the host's, and the same transform
/// snapshot that restores them restores this stack, which is why flatten
/// records a transform entry rather than a graph one.
#[test]
fn flatten_leaves_one_layer_and_can_be_undone() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    let before: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    assert!(before.len() > 1, "need a stack to flatten");

    s.invoke(command_id::LAYER_FLATTEN, CommandArgs::None)
        .expect("flatten");
    let graph = s.graph.as_ref().expect("graph");
    assert_eq!(graph.layer_count(), 1);
    assert_eq!(graph.layers()[0].name, "Background");
    assert_eq!(graph.active_id(), Some(graph.layers()[0].id));
    assert!(
        !before.contains(&graph.layers()[0].id),
        "the flattened layer reused an id that other state may still name"
    );

    // Undo is the host's document snapshot, so the timeline entry is a
    // transform rather than a graph command.
    assert_eq!(
        s.history
            .rows_newest_first()
            .first()
            .map(|r| r.kind.clone()),
        Some("transform".to_owned())
    );
}

/// Flattening an empty document is refused rather than silently allowed.
#[test]
fn flatten_needs_a_document() {
    let mut s = SessionState::default();
    assert!(
        s.invoke(command_id::LAYER_FLATTEN, CommandArgs::None)
            .is_err()
    );
}

/// Merge Down consumes the pair and leaves one layer where the lower was.
#[test]
fn merge_down_replaces_the_pair_with_one_layer() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    let before = s.graph.as_ref().expect("graph").layer_count();
    let stack: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    let active = s
        .graph
        .as_ref()
        .expect("graph")
        .active_id()
        .expect("active");
    let index = s
        .graph
        .as_ref()
        .expect("graph")
        .index_of(active)
        .expect("index");
    let lower = stack[index - 1];

    s.invoke(command_id::LAYER_MERGE_DOWN, CommandArgs::None)
        .expect("merge down");
    let graph = s.graph.as_ref().expect("graph");
    assert_eq!(graph.layer_count(), before - 1);
    let merged = graph.active_id().expect("active");
    assert!(
        !stack.contains(&merged),
        "the merged layer reused an id from the pair it replaced"
    );
    assert_eq!(
        graph.index_of(merged),
        Some(index - 1),
        "it took the lower slot"
    );
    assert!(graph.get(lower).is_none() && graph.get(active).is_none());
}

/// The bottom layer has nothing to merge into.
#[test]
fn merge_down_refuses_at_the_bottom_of_the_stack() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let bottom = s.graph.as_ref().expect("graph").layers()[0].id;
    s.invoke(command_id::LAYER_SET_ACTIVE, CommandArgs::LayerIndex(0))
        .expect("select bottom");
    assert_eq!(s.graph.as_ref().expect("graph").active_id(), Some(bottom));
    let error = s
        .invoke(command_id::LAYER_MERGE_DOWN, CommandArgs::None)
        .expect_err("bottom layer");
    assert!(error.user_message().contains("below"), "{error:?}");
}

/// Merge Visible keeps the layers it cannot see.
#[test]
fn merge_visible_leaves_hidden_layers_alone() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    let hidden = s.graph.as_ref().expect("graph").layers()[0].id;
    s.invoke(
        command_id::LAYER_SET_VISIBILITY,
        CommandArgs::SetVisibility {
            index: 0,
            visible: false,
        },
    )
    .expect("hide");

    s.invoke(command_id::LAYER_MERGE_VISIBLE, CommandArgs::None)
        .expect("merge visible");
    let graph = s.graph.as_ref().expect("graph");
    assert_eq!(
        graph.layer_count(),
        2,
        "one merged layer plus the hidden one"
    );
    assert!(graph.get(hidden).is_some(), "the hidden layer was consumed");
}

/// One visible layer is not a merge.
#[test]
fn merge_visible_needs_two_visible_layers() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let count = s.graph.as_ref().expect("graph").layer_count();
    for index in 1..count {
        s.invoke(
            command_id::LAYER_SET_VISIBILITY,
            CommandArgs::SetVisibility {
                index: i32::try_from(index).expect("index"),
                visible: false,
            },
        )
        .expect("hide");
    }
    let error = s
        .invoke(command_id::LAYER_MERGE_VISIBLE, CommandArgs::None)
        .expect_err("one visible layer");
    assert!(error.user_message().contains("two"), "{error:?}");
}

/// A group with two layers in it becomes one layer where the group stood.
#[test]
fn merge_group_replaces_the_group_and_its_contents_with_one_layer() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    let stack: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.selected_layer_ids = stack.clone();
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    let group = s
        .graph
        .as_ref()
        .expect("graph")
        .active_id()
        .expect("the group is active");
    assert_eq!(
        s.graph.as_ref().expect("graph").descendants_of(group).len(),
        stack.len(),
        "both layers went into the group"
    );

    s.invoke(command_id::LAYER_MERGE_GROUP, CommandArgs::None)
        .expect("merge group");
    let graph = s.graph.as_ref().expect("graph");
    assert_eq!(graph.layer_count(), 1, "the group and its contents are one");
    let merged = graph.active_id().expect("active");
    assert_eq!(
        graph.index_of(merged),
        Some(0),
        "it stands where they stood"
    );
    assert!(graph.get(group).is_none(), "the group record survived");
    for id in stack {
        assert!(graph.get(id).is_none(), "a member survived the merge");
    }
    assert_eq!(graph.get(merged).map(|l| l.kind), Some(LayerKind::Raster));
    assert_eq!(
        graph.get(merged).map(|l| l.parent),
        Some(None),
        "a top-level group merges to a top-level layer"
    );
}

/// A group inside a group merges with the outer one.
///
/// Membership is the `parent` chain, so a nested group's children name the
/// *inner* group and would be missed by anything that walked one level.
#[test]
fn merge_group_takes_a_nested_group_with_it() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");

    // Inner group over the top two layers, then an outer group over that.
    let stack: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.selected_layer_ids = stack[1..].to_vec();
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("inner group");
    let inner = s.graph.as_ref().expect("graph").active_id().expect("inner");
    s.selected_layer_ids = vec![inner];
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("outer group");
    let outer = s.graph.as_ref().expect("graph").active_id().expect("outer");
    assert_eq!(
        s.graph.as_ref().expect("graph").descendants_of(outer).len(),
        stack.len(),
        "the inner group and every layer it holds are inside the outer one"
    );

    s.invoke(command_id::LAYER_MERGE_GROUP, CommandArgs::None)
        .expect("merge group");
    let graph = s.graph.as_ref().expect("graph");
    assert!(graph.get(inner).is_none(), "the nested group survived");
    assert!(graph.get(outer).is_none(), "the outer group survived");
    assert_eq!(
        graph.layer_count(),
        2,
        "the bottom layer plus one merged layer"
    );
}

/// Merging the inner group leaves the merged layer inside the outer one.
#[test]
fn merging_a_nested_group_keeps_the_result_in_its_parent() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    let stack: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.selected_layer_ids = stack.clone();
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("inner group");
    let inner = s.graph.as_ref().expect("graph").active_id().expect("inner");
    s.selected_layer_ids = vec![inner];
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("outer group");
    let outer = s.graph.as_ref().expect("graph").active_id().expect("outer");

    let inner_index = s
        .graph
        .as_ref()
        .expect("graph")
        .index_of(inner)
        .expect("the inner group is in the stack");
    s.invoke(
        command_id::LAYER_SET_ACTIVE,
        CommandArgs::LayerIndex(i32::try_from(inner_index).expect("index")),
    )
    .expect("select the inner group");
    s.invoke(command_id::LAYER_MERGE_GROUP, CommandArgs::None)
        .expect("merge the inner group");

    let graph = s.graph.as_ref().expect("graph");
    let merged = graph.active_id().expect("active");
    assert!(graph.get(outer).is_some(), "the outer group was consumed");
    assert_eq!(
        graph.get(merged).map(|l| l.parent),
        Some(Some(outer)),
        "the merged layer left the group it was inside"
    );
}

/// Hidden members are discarded, the way Flatten discards what it cannot see.
#[test]
fn merge_group_discards_hidden_members() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    s.invoke(
        command_id::LAYER_SET_VISIBILITY,
        CommandArgs::SetVisibility {
            index: 0,
            visible: false,
        },
    )
    .expect("hide the bottom one");
    let stack: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.selected_layer_ids = stack.clone();
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");

    s.invoke(command_id::LAYER_MERGE_GROUP, CommandArgs::None)
        .expect("merge group");
    let graph = s.graph.as_ref().expect("graph");
    assert_eq!(graph.layer_count(), 1, "the hidden member was kept");
    for id in stack {
        assert!(graph.get(id).is_none());
    }
}

/// A group whose contents are all hidden has nothing to merge.
#[test]
fn merge_group_refuses_a_group_with_nothing_visible() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let count = s.graph.as_ref().expect("graph").layer_count();
    for index in 0..count {
        s.invoke(
            command_id::LAYER_SET_VISIBILITY,
            CommandArgs::SetVisibility {
                index: i32::try_from(index).expect("index"),
                visible: false,
            },
        )
        .expect("hide");
    }
    let stack: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.selected_layer_ids = stack;
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    let error = s
        .invoke(command_id::LAYER_MERGE_GROUP, CommandArgs::None)
        .expect_err("nothing visible");
    assert!(error.user_message().contains("visible"), "{error:?}");
}

/// Merge Group is for groups, and says so.
#[test]
fn merge_group_refuses_anything_that_is_not_a_group() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    let error = s
        .invoke(command_id::LAYER_MERGE_GROUP, CommandArgs::None)
        .expect_err("a raster layer is not a group");
    assert!(error.user_message().contains("group"), "{error:?}");
}

/// Merge Down's refusal names the command that does the job.
///
/// A dead end that says where to go is worth more than one that says no: the
/// two share `Ctrl+E` in Photoshop, so this is the message a user reaching
/// for muscle memory will see.
#[test]
fn merge_down_names_merge_group_when_it_refuses_one() {
    let mut s = SessionState::default();
    s.apply_preset(SizePreset::P720);
    s.invoke(command_id::LAYER_CREATE, CommandArgs::None)
        .expect("add");
    let stack: Vec<_> = s
        .graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect();
    s.selected_layer_ids = stack;
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    let error = s
        .invoke(command_id::LAYER_MERGE_DOWN, CommandArgs::None)
        .expect_err("a group cannot merge down");
    assert!(
        error.user_message().contains("Merge Group"),
        "the refusal should name Merge Group: {error:?}"
    );
}

/// The stack ids, bottom first.
fn stack_of(s: &SessionState) -> Vec<LayerId> {
    s.graph
        .as_ref()
        .expect("graph")
        .layers()
        .iter()
        .map(|l| l.id)
        .collect()
}

/// Grouping non-adjacent layers has to gather them.
///
/// Setting parents and leaving the stack alone left a non-member sitting
/// between two members. The panel indents by nesting, so that drew a group
/// whose contents were interrupted by a layer not in it — and no group can
/// composite as a unit while something else is stacked through the middle.
#[test]
fn grouping_gathers_members_that_were_not_adjacent() {
    let (mut s, _) = session_with_layers(3);
    let before = stack_of(&s);
    assert_eq!(before.len(), 5, "background, seeded layer, and three added");
    // Skip the layer directly below the top one.
    let (low, high) = (before[1], before[4]);
    let passed_over = before[3];
    s.selected_layer_ids = vec![low, high];
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");

    let after = stack_of(&s);
    let group = s
        .graph
        .as_ref()
        .expect("graph")
        .active_id()
        .expect("the group is active");
    let at = |id: LayerId| after.iter().position(|x| *x == id).expect("in the stack");
    assert_eq!(
        at(high),
        at(low) + 1,
        "the members ended up adjacent to each other"
    );
    assert_eq!(at(group), at(high) + 1, "with the group directly above");
    assert!(
        at(passed_over) < at(low) || at(passed_over) > at(group),
        "the layer that was between them is now outside the run"
    );
}

/// Gathering closes gaps; it does not restack the layers it is grouping.
#[test]
fn gathering_keeps_the_members_in_their_own_order() {
    let (mut s, _) = session_with_layers(3);
    let before = stack_of(&s);
    let (low, high) = (before[1], before[4]);
    s.selected_layer_ids = vec![high, low];
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    let after = stack_of(&s);
    let at = |id: LayerId| after.iter().position(|x| *x == id).expect("in the stack");
    assert!(
        at(low) < at(high),
        "the one that was lower is still the lower of the two, whatever \
         order the selection was made in"
    );
}

/// The gather is part of the same undo step as the group itself.
#[test]
fn undoing_a_group_puts_the_stack_back() {
    let (mut s, _) = session_with_layers(3);
    let before = stack_of(&s);
    s.selected_layer_ids = vec![before[1], before[4]];
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    assert_ne!(stack_of(&s), before, "grouping moved something");
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo");
    assert_eq!(
        stack_of(&s),
        before,
        "undo restores the order as well as removing the group"
    );
}

/// Layers that were already adjacent must not be moved at all.
#[test]
fn grouping_adjacent_layers_leaves_the_stack_where_it_was() {
    let (mut s, _) = session_with_layers(3);
    let before = stack_of(&s);
    s.selected_layer_ids = vec![before[3], before[4]];
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    let after = stack_of(&s);
    assert_eq!(
        after[..3],
        before[..3],
        "everything below the run is untouched"
    );
    assert_eq!(after[3], before[3]);
    assert_eq!(after[4], before[4]);
}
fn arrange(s: &mut SessionState, op: crate::ArrangeOp) -> Result<(), CommandError> {
    s.invoke(
        command_id::LAYER_ARRANGE,
        CommandArgs::Arrange {
            op: op.as_str().to_owned(),
        },
    )
    .map(|_| ())
}

#[test]
fn bring_forward_moves_the_active_layer_up_one_place() {
    let (mut s, _) = session_with_layers(2);
    let before = stack_of(&s);
    s.selected_layer_ids = vec![before[1]];
    let _ = s.graph.as_mut().expect("graph").set_active(before[1]);
    arrange(&mut s, crate::ArrangeOp::Forward).expect("bring forward");
    let after = stack_of(&s);
    assert_eq!(after[1], before[2], "the layer above swapped down");
    assert_eq!(after[2], before[1], "and the moved one is above it");
    assert_eq!(after[0], before[0], "nothing below moved");
}

#[test]
fn send_to_back_puts_the_layer_at_the_bottom() {
    let (mut s, _) = session_with_layers(2);
    let before = stack_of(&s);
    let top = *before.last().expect("a top layer");
    s.selected_layer_ids = vec![top];
    let _ = s.graph.as_mut().expect("graph").set_active(top);
    arrange(&mut s, crate::ArrangeOp::Back).expect("send to back");
    assert_eq!(stack_of(&s)[0], top);
}

#[test]
fn bring_to_front_puts_the_layer_on_top() {
    let (mut s, _) = session_with_layers(2);
    let before = stack_of(&s);
    s.selected_layer_ids = vec![before[0]];
    let _ = s.graph.as_mut().expect("graph").set_active(before[0]);
    arrange(&mut s, crate::ArrangeOp::Front).expect("bring to front");
    assert_eq!(*stack_of(&s).last().expect("a top layer"), before[0]);
}

/// A refusal, not a silent no-op: the menu entry stays enabled at the ends of
/// the stack, so pressing it there has to say why nothing happened.
#[test]
fn arranging_past_the_end_says_so() {
    let (mut s, _) = session_with_layers(2);
    let before = stack_of(&s);
    let top = *before.last().expect("a top layer");
    s.selected_layer_ids = vec![top];
    let _ = s.graph.as_mut().expect("graph").set_active(top);
    let err = arrange(&mut s, crate::ArrangeOp::Forward).expect_err("nowhere to go");
    assert!(
        matches!(err, CommandError::Rejected(m) if m.contains("top")),
        "the refusal names which end it hit"
    );
    assert_eq!(stack_of(&s), before, "and nothing moved");
}

#[test]
fn an_unknown_arrange_op_is_refused() {
    let (mut s, _) = session_with_layers(1);
    let before = stack_of(&s);
    let err = s
        .invoke(
            command_id::LAYER_ARRANGE,
            CommandArgs::Arrange {
                op: "sideways".to_owned(),
            },
        )
        .expect_err("no such op");
    assert!(matches!(err, CommandError::InvalidArgument(_)));
    assert_eq!(stack_of(&s), before);
}

/// Moving a group has to carry its contents.
///
/// The group row on its own would leave its members where they were: stacked
/// around whatever the group landed next to, still naming it as their parent,
/// and drawn indented under a group no longer above them.
#[test]
fn moving_a_group_carries_what_is_inside_it() {
    let (mut s, _) = session_with_layers(3);
    let before = stack_of(&s);
    s.selected_layer_ids = vec![before[3], before[4]];
    s.invoke(command_id::LAYER_GROUP, CommandArgs::None)
        .expect("group");
    let group = s
        .graph
        .as_ref()
        .expect("graph")
        .active_id()
        .expect("the group is active");
    let members = s.graph.as_ref().expect("graph").descendants_of(group);
    assert_eq!(members.len(), 2);

    s.selected_layer_ids = vec![group];
    arrange(&mut s, crate::ArrangeOp::Back).expect("send the group to the back");

    let after = stack_of(&s);
    let at = |id: LayerId| after.iter().position(|x| *x == id).expect("in the stack");
    assert!(
        members.iter().all(|id| at(*id) < at(group)),
        "the members went with it and are still below their group"
    );
    assert_eq!(
        at(group),
        members.len(),
        "the run landed at the bottom, group on top of its own members"
    );
}

#[test]
fn undoing_an_arrange_puts_the_order_back() {
    let (mut s, _) = session_with_layers(2);
    let before = stack_of(&s);
    s.selected_layer_ids = vec![before[0]];
    let _ = s.graph.as_mut().expect("graph").set_active(before[0]);
    arrange(&mut s, crate::ArrangeOp::Front).expect("bring to front");
    assert_ne!(stack_of(&s), before);
    s.invoke(command_id::HISTORY_UNDO, CommandArgs::None)
        .expect("undo");
    assert_eq!(stack_of(&s), before);
}

/// A document bigger than the compositor can hold has to be refused.
///
/// The dialogs stopped at 32768, which is not a limit of anything: past
/// `MAX_DOCUMENT_DIMENSION` wgpu declines the layer textures, the result
/// texture stays invalid, and the editor shows a populated layers panel over a
/// canvas that draws nothing while every frame logs a validation error. A
/// 20000-pixel document did exactly that, with no message of any kind.
#[test]
fn a_document_larger_than_the_gpu_can_hold_is_refused() {
    let mut s = SessionState::default();
    let too_wide = crate::MAX_DOCUMENT_DIMENSION + 1;
    let err = s
        .invoke(
            command_id::DOCUMENT_NEW_SIZE,
            CommandArgs::NewSize {
                width: too_wide,
                height: 1080,
            },
        )
        .expect_err("the compositor cannot hold it");
    assert!(
        matches!(
            err,
            CommandError::Document(crate::DocumentError::DimensionTooLarge { .. })
        ),
        "got {err:?}"
    );
    assert!(!s.has_document, "and no half-made document is left behind");
}

/// The refusal has to say the number the user typed and the number they can.
#[test]
fn the_refusal_names_both_numbers() {
    let err = crate::DocumentError::check_size(crate::DocumentSize::new(20000, 1080))
        .expect_err("too wide");
    let message = err.to_string();
    assert!(message.contains("20000"), "{message}");
    assert!(
        message.contains(&crate::MAX_DOCUMENT_DIMENSION.to_string()),
        "{message}"
    );
    assert!(
        CommandError::Document(err).is_user_correctable(),
        "typing a smaller number fixes it, so it belongs in front of the user"
    );
}

#[test]
fn the_largest_supported_document_is_still_allowed() {
    let mut s = SessionState::default();
    let edge = crate::MAX_DOCUMENT_DIMENSION;
    s.invoke(
        command_id::DOCUMENT_NEW_SIZE,
        CommandArgs::NewSize {
            width: edge,
            height: edge,
        },
    )
    .expect("the limit itself is a size that works");
    assert_eq!(s.size.width, edge);
    assert_eq!(s.size.height, edge);
}

#[test]
fn resizing_the_canvas_past_the_limit_is_refused() {
    let (mut s, _) = session_with_layers(1);
    let before = s.size;
    let err = s
        .invoke(
            command_id::DOCUMENT_CANVAS_SIZE,
            CommandArgs::Resize {
                width: crate::MAX_DOCUMENT_DIMENSION + 1,
                height: 1080,
            },
        )
        .expect_err("too wide");
    assert!(matches!(
        err,
        CommandError::Document(crate::DocumentError::DimensionTooLarge { .. })
    ));
    assert_eq!(s.size, before, "and the canvas did not move");
}
