//! Gesture-level undo stack (ADR-013 G16).

use crate::document::DocumentGraph;
use crate::filter_plan::FilterPlan;
use crate::layer::{
    AdjustmentParams, BlendMode, FillContent, FilterEffect, Layer, LayerId, LayerMask,
    LayerTransform, LockFlags, ShapeContent,
};
use crate::layer_style::LayerStyle;
use crate::paths::PathDocument;

/// One undoable gesture applied to the document graph (structure only in Phase 3).
#[derive(Debug, Clone)]
pub enum GraphCommand {
    AddLayer {
        id: LayerId,
        index: usize,
        layer: Layer,
    },
    DeleteLayer {
        id: LayerId,
        index: usize,
        layer: Layer,
        prev_active: Option<LayerId>,
    },
    MoveLayer {
        id: LayerId,
        from: usize,
        to: usize,
    },
    SetVisibility {
        id: LayerId,
        prev: bool,
        next: bool,
    },
    SetOpacity {
        id: LayerId,
        prev: f32,
        next: f32,
    },
    SetBlend {
        id: LayerId,
        prev: BlendMode,
        next: BlendMode,
    },
    Rename {
        id: LayerId,
        prev: String,
        next: String,
    },
    SetActive {
        prev: Option<LayerId>,
        next: Option<LayerId>,
    },
    SetMask {
        id: LayerId,
        prev: Option<LayerMask>,
        next: Option<LayerMask>,
    },
    SetLocks {
        id: LayerId,
        prev: LockFlags,
        next: LockFlags,
    },
    SetClipsToBelow {
        id: LayerId,
        prev: bool,
        next: bool,
    },
    /// Blend ranges — which tones of this layer, and of what is under it,
    /// the layer is allowed to show through.
    SetBlendIf {
        id: LayerId,
        prev: crate::BlendIf,
        next: crate::BlendIf,
    },
    /// Layer placement, written by align/distribute rather than by the gizmo.
    ///
    /// The free-transform gizmo commits by baking pixels and resetting the
    /// transform to the identity, so it needs no undo entry here. Aligning
    /// leaves the transform live, which means the *transform* is the edit and
    /// history has to be able to put the old one back.
    SetTransform {
        id: LayerId,
        prev: LayerTransform,
        next: LayerTransform,
    },
    SetAdjustment {
        id: LayerId,
        prev: Option<AdjustmentParams>,
        next: Option<AdjustmentParams>,
    },
    SetEffects {
        id: LayerId,
        prev: Vec<FilterEffect>,
        next: Vec<FilterEffect>,
    },
    SetFill {
        id: LayerId,
        prev: Option<FillContent>,
        next: Option<FillContent>,
    },
    SetShape {
        id: LayerId,
        prev: Option<ShapeContent>,
        next: Option<ShapeContent>,
    },
    SetFilterPlan {
        id: LayerId,
        prev: FilterPlan,
        next: FilterPlan,
    },
    SetPaths {
        prev: PathDocument,
        next: PathDocument,
    },
    /// Layer effect styles (drop shadow, stroke, outer glow, colour overlay).
    SetStyles {
        id: LayerId,
        prev: Vec<LayerStyle>,
        next: Vec<LayerStyle>,
    },
    SetVectorMask {
        id: LayerId,
        prev: Option<crate::VectorMask>,
        next: Option<crate::VectorMask>,
    },
    SetParent {
        id: LayerId,
        prev: Option<LayerId>,
        next: Option<LayerId>,
    },
    /// Replace stack order (same layer set, new sibling order).
    SetStackOrder {
        prev: Vec<LayerId>,
        next: Vec<LayerId>,
    },
    /// Atomic multi-step graph mutation (one undo entry).
    Batch(Vec<GraphCommand>),
}

impl GraphCommand {
    /// Apply this command to `graph`, reporting whether it landed.
    ///
    /// Every mutation below can miss: the graph accessors return `None` when
    /// the target layer is gone, and this used to discard that. A command that
    /// silently did nothing left the document diverged from what history said
    /// about it, with no way for the caller to notice — the failure was
    /// unrepresentable, because `apply` returned `()`.
    ///
    /// A [`Self::Batch`] reports false if *any* member missed. A partially
    /// applied batch is exactly the case worth surfacing: the entry claims to
    /// be one atomic step.
    #[must_use]
    pub fn apply(&self, graph: &mut DocumentGraph) -> bool {
        match self {
            Self::AddLayer { index, layer, .. } => {
                graph.insert_layer_at(*index, layer.clone());
                true
            }
            Self::DeleteLayer { id, .. } => graph.remove_layer(*id).is_some(),
            Self::MoveLayer { id, to, .. } => graph.move_layer(*id, *to).is_some(),
            Self::SetVisibility { id, next, .. } => graph.set_visibility(*id, *next).is_some(),
            Self::SetOpacity { id, next, .. } => graph.set_opacity(*id, *next).is_some(),
            Self::SetBlend { id, next, .. } => graph.set_blend(*id, *next).is_some(),
            Self::Rename { id, next, .. } => graph.rename(*id, next.clone()).is_some(),
            Self::SetActive { next, .. } => match next {
                Some(id) => graph.set_active(*id),
                None => {
                    graph.clear_active();
                    true
                }
            },
            Self::SetMask { id, next, .. } => graph.set_mask(*id, next.clone()).is_some(),
            Self::SetLocks { id, next, .. } => match graph.get_mut(*id) {
                Some(layer) => {
                    layer.locks = *next;
                    layer.locked = next.all;
                    true
                }
                None => false,
            },
            Self::SetClipsToBelow { id, next, .. } => {
                graph.set_clips_to_below(*id, *next).is_some()
            }
            Self::SetTransform { id, next, .. } => graph.set_transform(*id, *next).is_some(),
            Self::SetBlendIf { id, next, .. } => graph.set_blend_if(*id, *next).is_some(),
            Self::SetAdjustment { id, next, .. } => {
                graph.set_adjustment(*id, next.clone()).is_some()
            }
            Self::SetEffects { id, next, .. } => graph.set_effects(*id, next.clone()).is_some(),
            Self::SetFill { id, next, .. } => graph.set_fill(*id, next.clone()).is_some(),
            Self::SetShape { id, next, .. } => match graph.get_mut(*id) {
                Some(layer) => {
                    layer.shape = next.clone();
                    true
                }
                None => false,
            },
            Self::SetFilterPlan { id, next, .. } => match graph.get_mut(*id) {
                Some(layer) => {
                    layer.filter_plan = next.clone();
                    true
                }
                None => false,
            },
            Self::SetPaths { next, .. } => {
                graph.paths = next.clone();
                true
            }
            Self::SetStyles { id, next, .. } => match graph.get_mut(*id) {
                Some(layer) => {
                    layer.styles = next.clone();
                    true
                }
                None => false,
            },
            Self::SetVectorMask { id, next, .. } => match graph.get_mut(*id) {
                Some(layer) => {
                    layer.vector_mask = next.clone();
                    true
                }
                None => false,
            },
            Self::SetParent { id, next, .. } => graph.set_parent(*id, *next).is_some(),
            Self::SetStackOrder { next, .. } => graph.reorder_stack(next),
            Self::Batch(cmds) => {
                // A loop rather than `all`: every member must be attempted, and
                // short-circuiting would leave the rest of an atomic step
                // unapplied on the first miss.
                let mut applied = true;
                for cmd in cmds {
                    applied &= cmd.apply(graph);
                }
                applied
            }
        }
    }

    /// Fold a later edit of the same field on the same target into this one.
    ///
    /// A slider drag arrives as a run of separate commands, each carrying the
    /// value before and after one step. Collapsing them keeps the *oldest*
    /// `prev` and the *newest* `next`, so one undo returns to where the gesture
    /// started rather than stepping back through every intermediate value.
    ///
    /// Returns `None` when the two are not the same edit — a different variant,
    /// or the same variant on a different layer — in which case they are
    /// separate history entries and must stay that way.
    #[must_use]
    pub fn merged_with(&self, newer: &Self) -> Option<Self> {
        match (self, newer) {
            (
                Self::SetOpacity { id, prev, .. },
                Self::SetOpacity {
                    id: later_id, next, ..
                },
            ) if id == later_id => Some(Self::SetOpacity {
                id: *id,
                prev: *prev,
                next: *next,
            }),
            (
                Self::SetAdjustment { id, prev, .. },
                Self::SetAdjustment {
                    id: later_id, next, ..
                },
            ) if id == later_id => Some(Self::SetAdjustment {
                id: *id,
                prev: prev.clone(),
                next: next.clone(),
            }),
            (
                Self::SetEffects { id, prev, .. },
                Self::SetEffects {
                    id: later_id, next, ..
                },
            ) if id == later_id => Some(Self::SetEffects {
                id: *id,
                prev: prev.clone(),
                next: next.clone(),
            }),
            (
                Self::SetFilterPlan { id, prev, .. },
                Self::SetFilterPlan {
                    id: later_id, next, ..
                },
            ) if id == later_id => Some(Self::SetFilterPlan {
                id: *id,
                prev: prev.clone(),
                next: next.clone(),
            }),
            // Blend If is four slider handles per range; a drag would
            // otherwise leave one history entry per step of the drag.
            (
                Self::SetBlendIf { id, prev, .. },
                Self::SetBlendIf {
                    id: later_id, next, ..
                },
            ) if id == later_id => Some(Self::SetBlendIf {
                id: *id,
                prev: *prev,
                next: *next,
            }),
            _ => None,
        }
    }

    pub fn invert(&self) -> Self {
        match self {
            Self::AddLayer { id, index, layer } => Self::DeleteLayer {
                id: *id,
                index: *index,
                layer: layer.clone(),
                prev_active: None,
            },
            Self::DeleteLayer {
                id,
                index,
                layer,
                prev_active,
            } => {
                let _ = (id, prev_active);
                Self::AddLayer {
                    id: layer.id,
                    index: *index,
                    layer: layer.clone(),
                }
            }
            Self::MoveLayer { id, from, to } => Self::MoveLayer {
                id: *id,
                from: *to,
                to: *from,
            },
            Self::SetVisibility { id, prev, next } => Self::SetVisibility {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::SetOpacity { id, prev, next } => Self::SetOpacity {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::SetBlend { id, prev, next } => Self::SetBlend {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::Rename { id, prev, next } => Self::Rename {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetActive { prev, next } => Self::SetActive {
                prev: *next,
                next: *prev,
            },
            Self::SetMask { id, prev, next } => Self::SetMask {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetLocks { id, prev, next } => Self::SetLocks {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::SetTransform { id, prev, next } => Self::SetTransform {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::SetBlendIf { id, prev, next } => Self::SetBlendIf {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::SetClipsToBelow { id, prev, next } => Self::SetClipsToBelow {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::SetAdjustment { id, prev, next } => Self::SetAdjustment {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetEffects { id, prev, next } => Self::SetEffects {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetFill { id, prev, next } => Self::SetFill {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetShape { id, prev, next } => Self::SetShape {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetFilterPlan { id, prev, next } => Self::SetFilterPlan {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetPaths { prev, next } => Self::SetPaths {
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetStyles { id, prev, next } => Self::SetStyles {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetVectorMask { id, prev, next } => Self::SetVectorMask {
                id: *id,
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::SetParent { id, prev, next } => Self::SetParent {
                id: *id,
                prev: *next,
                next: *prev,
            },
            Self::SetStackOrder { prev, next } => Self::SetStackOrder {
                prev: next.clone(),
                next: prev.clone(),
            },
            Self::Batch(cmds) => Self::Batch(cmds.iter().rev().map(GraphCommand::invert).collect()),
        }
    }
}

#[derive(Debug, Default)]
pub struct UndoStack {
    undo: Vec<GraphCommand>,
    redo: Vec<GraphCommand>,
    limit: usize,
}

impl UndoStack {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Update stack capacity and drop oldest undo commands when over limit.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        while self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    /// Drop the oldest undo command without applying it (timeline budget sync).
    pub fn drop_oldest(&mut self) -> Option<GraphCommand> {
        if self.undo.is_empty() {
            None
        } else {
            Some(self.undo.remove(0))
        }
    }

    /// Replace the newest command with `cmd` folded into it, when they are the
    /// same edit. Returns whether the fold happened.
    pub fn merge_into_newest(&mut self, cmd: &GraphCommand) -> bool {
        let Some(top) = self.undo.last_mut() else {
            return false;
        };
        let Some(merged) = top.merged_with(cmd) else {
            return false;
        };
        *top = merged;
        self.redo.clear();
        true
    }

    /// Record a command that was **already applied** to the graph.
    pub fn push_applied(&mut self, cmd: GraphCommand) {
        self.undo.push(cmd);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Reverse the newest graph command.
    ///
    /// Returns false when there was nothing to reverse *or* when the reversal
    /// did not land — the second case means the graph no longer contains what
    /// the command described, and the caller must not treat the step as done.
    pub fn undo(&mut self, graph: &mut DocumentGraph) -> bool {
        let Some(cmd) = self.undo.pop() else {
            return false;
        };
        let inv = cmd.invert();
        if !inv.apply(graph) {
            // Put it back: a command that could not be reversed is still the
            // newest thing done, and dropping it would strand the redo stack.
            self.undo.push(cmd);
            return false;
        }
        // Restore active after delete invert special-case
        restore_prev_active_after_delete_undo(graph, &cmd);
        self.redo.push(cmd);
        true
    }

    /// Reapply the newest undone graph command. See [`Self::undo`] for what a
    /// false return means.
    pub fn redo(&mut self, graph: &mut DocumentGraph) -> bool {
        let Some(cmd) = self.redo.pop() else {
            return false;
        };
        if !cmd.apply(graph) {
            self.redo.push(cmd);
            return false;
        }
        self.undo.push(cmd);
        true
    }
}

fn restore_prev_active_after_delete_undo(graph: &mut DocumentGraph, cmd: &GraphCommand) {
    match cmd {
        GraphCommand::DeleteLayer {
            prev_active: Some(id),
            ..
        } => {
            let _ = graph.set_active(*id);
        }
        GraphCommand::Batch(cmds) => {
            for nested in cmds.iter().rev() {
                if let GraphCommand::DeleteLayer {
                    prev_active: Some(id),
                    ..
                } = nested
                {
                    let _ = graph.set_active(*id);
                    break;
                }
            }
        }
        _ => {}
    }
}

/// High-level mutations that push unified history entries.
pub mod actions {
    use super::*;
    use crate::history::HistoryService;

    /// Add a layer and record the undo entry.
    ///
    /// # Errors
    /// Returns [`crate::DocumentError`] when the layer cap is reached or the graph is inconsistent.
    pub fn add_layer(
        graph: &mut DocumentGraph,
        history: &mut HistoryService,
        name: Option<String>,
    ) -> Result<LayerId, crate::DocumentError> {
        let id = graph.add_layer_top(name)?;
        let index = graph.index_of(id).unwrap_or(0);
        let layer = graph
            .get(id)
            .cloned()
            .ok_or(crate::DocumentError::LayerMissingAfterAdd)?;
        graph.bump_generation();
        history.push_graph_applied(
            GraphCommand::AddLayer { id, index, layer },
            "Add layer",
            graph.generation,
        );
        Ok(id)
    }

    pub fn delete_layer(
        graph: &mut DocumentGraph,
        history: &mut HistoryService,
        id: LayerId,
    ) -> bool {
        let prev_active = graph.active_id();
        let Some((index, layer)) = graph.remove_layer(id) else {
            return false;
        };
        graph.bump_generation();
        history.push_graph_applied(
            GraphCommand::DeleteLayer {
                id,
                index,
                layer,
                prev_active,
            },
            "Delete layer",
            graph.generation,
        );
        true
    }

    pub fn set_visibility(
        graph: &mut DocumentGraph,
        history: &mut HistoryService,
        id: LayerId,
        visible: bool,
    ) -> bool {
        let Some(prev) = graph.set_visibility(id, visible) else {
            return false;
        };
        if prev == visible {
            return true;
        }
        graph.bump_generation();
        history.push_graph_applied(
            GraphCommand::SetVisibility {
                id,
                prev,
                next: visible,
            },
            if visible { "Show layer" } else { "Hide layer" },
            graph.generation,
        );
        true
    }

    pub fn set_opacity(
        graph: &mut DocumentGraph,
        history: &mut HistoryService,
        id: LayerId,
        opacity: f32,
    ) -> bool {
        let Some(prev) = graph.set_opacity(id, opacity) else {
            return false;
        };
        let next = graph.get(id).map(|l| l.opacity).unwrap_or(opacity);
        if (prev - next).abs() < f32::EPSILON {
            return true;
        }
        graph.bump_generation();
        // `layer.set-opacity` is declared `UndoPolicy::Mergeable`; a slider drag
        // is one gesture, not one entry per step.
        history.push_graph_mergeable(
            GraphCommand::SetOpacity { id, prev, next },
            "Opacity",
            graph.generation,
        );
        true
    }

    pub fn set_blend(
        graph: &mut DocumentGraph,
        history: &mut HistoryService,
        id: LayerId,
        blend: BlendMode,
    ) -> bool {
        let Some(prev) = graph.set_blend(id, blend) else {
            return false;
        };
        if prev == blend {
            return true;
        }
        graph.bump_generation();
        history.push_graph_applied(
            GraphCommand::SetBlend {
                id,
                prev,
                next: blend,
            },
            "Blend mode",
            graph.generation,
        );
        true
    }

    pub fn move_layer(
        graph: &mut DocumentGraph,
        history: &mut HistoryService,
        id: LayerId,
        to_index: usize,
    ) -> bool {
        let Some((from, to)) = graph.move_layer(id, to_index) else {
            return false;
        };
        if from == to {
            return true;
        }
        graph.bump_generation();
        history.push_graph_applied(
            GraphCommand::MoveLayer { id, from, to },
            "Reorder layer",
            graph.generation,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentSize;

    #[test]
    fn undo_add_layer() {
        let mut g = DocumentGraph::new(DocumentSize::new(32, 32));
        let mut h = crate::HistoryService::new(64);
        let n0 = g.layer_count();
        actions::add_layer(&mut g, &mut h, Some("X".into())).expect("add");
        assert_eq!(g.layer_count(), n0 + 1);
        assert_eq!(h.undo_next(&mut g), Some(crate::HistoryKind::Graph));
        assert_eq!(g.layer_count(), n0);
        assert_eq!(h.redo_next(&mut g), Some(crate::HistoryKind::Graph));
        assert_eq!(g.layer_count(), n0 + 1);
    }

    #[test]
    fn undo_visibility() {
        let mut g = DocumentGraph::new(DocumentSize::new(32, 32));
        let mut h = crate::HistoryService::new(64);
        let id = g.layers()[0].id;
        actions::set_visibility(&mut g, &mut h, id, false);
        assert!(!g.get(id).unwrap().visible);
        h.undo_next(&mut g);
        assert!(g.get(id).unwrap().visible);
    }

    #[test]
    fn undo_opacity() {
        let mut g = DocumentGraph::new(DocumentSize::new(32, 32));
        let mut h = crate::HistoryService::new(64);
        let id = g.layers()[1].id;
        actions::set_opacity(&mut g, &mut h, id, 0.25);
        assert!((g.get(id).unwrap().opacity - 0.25).abs() < 1e-5);
        h.undo_next(&mut g);
        assert!((g.get(id).unwrap().opacity - 1.0).abs() < 1e-5);
    }

    #[test]
    fn undo_adjustment_params() {
        use crate::AdjustmentParams;
        let mut g = DocumentGraph::new(DocumentSize::new(32, 32));
        let mut h = crate::HistoryService::new(64);
        let id = g
            .add_adjustment_top(
                Some("BC".into()),
                AdjustmentParams::BrightnessContrast {
                    brightness: 0.0,
                    contrast: 0.0,
                },
            )
            .expect("add");
        let prev = g.get(id).and_then(|l| l.adjustment.clone());
        let next = AdjustmentParams::BrightnessContrast {
            brightness: 0.4,
            contrast: -0.1,
        };
        g.set_adjustment(id, Some(next.clone())).expect("set");
        h.push_graph_applied(
            GraphCommand::SetAdjustment {
                id,
                prev,
                next: Some(next),
            },
            "Adjustment",
            1,
        );
        h.undo_next(&mut g);
        let restored = g.get(id).and_then(|l| l.adjustment.clone());
        assert_eq!(
            restored,
            Some(AdjustmentParams::BrightnessContrast {
                brightness: 0.0,
                contrast: 0.0,
            })
        );
    }

    #[test]
    fn undo_gaussian_effect() {
        use crate::FilterParams;
        let mut g = DocumentGraph::new(DocumentSize::new(32, 32));
        let mut h = crate::HistoryService::new(64);
        let id = g.layers()[0].id;
        let (prev, _) = g.add_gaussian_blur(id, 8.0).expect("blur");
        let next = g.get(id).map(|l| l.effects.clone()).unwrap_or_default();
        h.push_graph_applied(GraphCommand::SetEffects { id, prev, next }, "Blur", 1);
        assert!(g.get(id).unwrap().effects.iter().any(|e| {
            matches!(e.params, FilterParams::GaussianBlur { radius } if (radius - 8.0).abs() < 1e-5)
        }));
        h.undo_next(&mut g);
        assert!(g.get(id).unwrap().effects.is_empty());
    }

    /// A command whose target no longer exists must say so.
    ///
    /// `apply` used to return `()`, so every one of these misses was
    /// unrepresentable: the graph accessor returned `None`, the arm discarded
    /// it, and the caller had no way to learn the document did not change.
    #[test]
    fn applying_to_a_missing_layer_reports_failure() {
        let mut graph = DocumentGraph::new(DocumentSize::new(4, 4));
        let ghost = LayerId(9_999);
        let cases: Vec<GraphCommand> = vec![
            GraphCommand::SetVisibility {
                id: ghost,
                prev: true,
                next: false,
            },
            GraphCommand::SetOpacity {
                id: ghost,
                prev: 1.0,
                next: 0.5,
            },
            GraphCommand::Rename {
                id: ghost,
                prev: "a".into(),
                next: "b".into(),
            },
            GraphCommand::MoveLayer {
                id: ghost,
                from: 0,
                to: 1,
            },
        ];
        for cmd in cases {
            assert!(
                !cmd.apply(&mut graph),
                "{cmd:?} claimed to apply to a layer that is not there"
            );
        }
    }

    /// The same commands must report success against a layer that is present,
    /// or the check above would pass for the wrong reason.
    #[test]
    fn applying_to_a_present_layer_reports_success() {
        let mut graph = DocumentGraph::new(DocumentSize::new(4, 4));
        let id = graph.layers()[0].id;
        assert!(
            GraphCommand::SetVisibility {
                id,
                prev: true,
                next: false,
            }
            .apply(&mut graph)
        );
        assert!(
            GraphCommand::Rename {
                id,
                prev: "a".into(),
                next: "renamed".into(),
            }
            .apply(&mut graph)
        );
        assert_eq!(graph.get(id).expect("layer").name, "renamed");
    }

    /// A batch is one atomic step, so a member that misses fails the whole
    /// thing — while the members that can apply still do.
    #[test]
    fn a_batch_reports_failure_but_still_applies_what_it_can() {
        let mut graph = DocumentGraph::new(DocumentSize::new(4, 4));
        let id = graph.layers()[0].id;
        let batch = GraphCommand::Batch(vec![
            GraphCommand::Rename {
                id,
                prev: "a".into(),
                next: "applied".into(),
            },
            GraphCommand::Rename {
                id: LayerId(9_999),
                prev: "x".into(),
                next: "missed".into(),
            },
        ]);
        assert!(
            !batch.apply(&mut graph),
            "a partial batch must report false"
        );
        assert_eq!(
            graph.get(id).expect("layer").name,
            "applied",
            "the member that could apply still did"
        );
    }

    /// The timeline must not advance past a graph step the stack cannot make.
    ///
    /// The two stacks have to move together; advancing the timeline when the
    /// graph stack had nothing left them describing different documents, with
    /// nothing able to notice.
    #[test]
    fn the_timeline_does_not_advance_when_the_graph_stack_is_empty() {
        use crate::history::{HistoryKind, HistoryService};
        let mut graph = DocumentGraph::new(DocumentSize::new(4, 4));
        let mut history = HistoryService::new(16);
        // A Graph entry on the timeline with no matching command on the stack.
        history.push_graph_applied(
            GraphCommand::SetPaths {
                prev: Default::default(),
                next: Default::default(),
            },
            "paths",
            1,
        );
        history.graph_stack_mut().clear();

        assert!(history.can_undo(), "the timeline still holds the entry");
        assert_eq!(
            history.undo_next(&mut graph),
            None,
            "an unmatched graph step must report nothing undone"
        );
        assert!(
            history.can_undo(),
            "and must leave the entry where it was rather than move it to redo"
        );
        assert!(!history.can_redo());
        let _ = HistoryKind::Graph;
    }
}
