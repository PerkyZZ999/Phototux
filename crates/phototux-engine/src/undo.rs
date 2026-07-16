//! Gesture-level undo stack (ADR-013 G16).

use crate::document::DocumentGraph;
use crate::layer::{BlendMode, Layer, LayerId, LayerMask};

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
    SetClipsToBelow {
        id: LayerId,
        prev: bool,
        next: bool,
    },
}

impl GraphCommand {
    pub fn apply(&self, graph: &mut DocumentGraph) {
        match self {
            Self::AddLayer { index, layer, .. } => {
                graph.insert_layer_at(*index, layer.clone());
            }
            Self::DeleteLayer { id, .. } => {
                let _ = graph.remove_layer(*id);
            }
            Self::MoveLayer { id, to, .. } => {
                let _ = graph.move_layer(*id, *to);
            }
            Self::SetVisibility { id, next, .. } => {
                let _ = graph.set_visibility(*id, *next);
            }
            Self::SetOpacity { id, next, .. } => {
                let _ = graph.set_opacity(*id, *next);
            }
            Self::SetBlend { id, next, .. } => {
                let _ = graph.set_blend(*id, *next);
            }
            Self::Rename { id, next, .. } => {
                let _ = graph.rename(*id, next.clone());
            }
            Self::SetActive { next, .. } => match next {
                Some(id) => {
                    let _ = graph.set_active(*id);
                }
                None => graph.clear_active(),
            },
            Self::SetMask { id, next, .. } => {
                let _ = graph.set_mask(*id, next.clone());
            }
            Self::SetClipsToBelow { id, next, .. } => {
                let _ = graph.set_clips_to_below(*id, *next);
            }
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
            Self::SetClipsToBelow { id, prev, next } => Self::SetClipsToBelow {
                id: *id,
                prev: *next,
                next: *prev,
            },
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

    /// Drop the oldest undo command without applying it (timeline budget sync).
    pub fn drop_oldest(&mut self) -> Option<GraphCommand> {
        if self.undo.is_empty() {
            None
        } else {
            Some(self.undo.remove(0))
        }
    }

    /// Record a command that was **already applied** to the graph.
    pub fn push_applied(&mut self, cmd: GraphCommand) {
        self.undo.push(cmd);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, graph: &mut DocumentGraph) -> bool {
        let Some(cmd) = self.undo.pop() else {
            return false;
        };
        let inv = cmd.invert();
        inv.apply(graph);
        // Restore active after delete invert special-case
        if let GraphCommand::DeleteLayer {
            prev_active: Some(id),
            ..
        } = &cmd
        {
            let _ = graph.set_active(*id);
        }
        self.redo.push(cmd);
        true
    }

    pub fn redo(&mut self, graph: &mut DocumentGraph) -> bool {
        let Some(cmd) = self.redo.pop() else {
            return false;
        };
        cmd.apply(graph);
        self.undo.push(cmd);
        true
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
        history.push_graph_applied(GraphCommand::AddLayer { id, index, layer }, "Add layer");
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
        history.push_graph_applied(
            GraphCommand::DeleteLayer {
                id,
                index,
                layer,
                prev_active,
            },
            "Delete layer",
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
        history.push_graph_applied(
            GraphCommand::SetVisibility {
                id,
                prev,
                next: visible,
            },
            if visible { "Show layer" } else { "Hide layer" },
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
        history.push_graph_applied(GraphCommand::SetOpacity { id, prev, next }, "Opacity");
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
        history.push_graph_applied(
            GraphCommand::SetBlend {
                id,
                prev,
                next: blend,
            },
            "Blend mode",
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
        history.push_graph_applied(GraphCommand::MoveLayer { id, from, to }, "Reorder layer");
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
}
