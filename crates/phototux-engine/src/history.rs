//! Unified transactional history timeline (ADR-013, ADR-017).

use crate::document::DocumentGraph;
use crate::undo::{GraphCommand, UndoStack};

/// Chronological entry kind for the session history panel / undo dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    /// Graph metadata mutation already recorded in [`UndoStack`].
    Graph,
    /// Committed paint stroke; GPU owns the texture snapshot.
    Stroke,
    /// Selection channel mutation.
    Selection,
    /// Committed transform / crop / resize.
    Transform,
}

/// One labeled transaction on the unified timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: u64,
    pub label: String,
    pub kind: HistoryKind,
    /// Document generation at the time the entry was recorded (0 = unknown/legacy).
    pub generation: u64,
}

/// Bounded undo/redo timeline coordinating graph + stroke (+ future) stacks.
#[derive(Debug)]
pub struct HistoryService {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    graph: UndoStack,
    next_id: u64,
    limit: usize,
}

impl HistoryService {
    pub fn new(limit: usize) -> Self {
        let limit = limit.max(1);
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            graph: UndoStack::new(limit),
            next_id: 1,
            limit,
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
        self.graph.clear();
    }

    /// Current retention budget (max undo entries retained).
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Update retention budget and drop oldest entries immediately when over limit.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        while self.undo.len() > self.limit {
            let removed = self.undo.remove(0);
            if removed.kind == HistoryKind::Graph {
                let _ = self.graph.drop_oldest();
            }
        }
        // Capacity for future graph pushes; stacks already aligned above.
        self.graph.set_limit(self.limit);
    }

    pub fn graph_stack_mut(&mut self) -> &mut UndoStack {
        &mut self.graph
    }

    pub fn graph_stack(&self) -> &UndoStack {
        &self.graph
    }

    pub fn entries_undo(&self) -> &[HistoryEntry] {
        &self.undo
    }

    pub fn labels_newest_first(&self) -> Vec<String> {
        self.undo.iter().rev().map(|e| e.label.clone()).collect()
    }

    fn push_entry(&mut self, kind: HistoryKind, label: impl Into<String>, generation: u64) {
        let entry = HistoryEntry {
            id: self.next_id,
            label: label.into(),
            kind,
            generation,
        };
        self.next_id = self.next_id.wrapping_add(1);
        self.undo.push(entry);
        if self.undo.len() > self.limit {
            let removed = self.undo.remove(0);
            if removed.kind == HistoryKind::Graph {
                // Drop oldest graph command to keep stacks aligned.
                let _ = self.graph.drop_oldest();
            }
        }
        self.redo.clear();
    }

    /// Record a graph command that was already applied.
    pub fn push_graph_applied(
        &mut self,
        cmd: GraphCommand,
        label: impl Into<String>,
        generation: u64,
    ) {
        self.graph.push_applied(cmd);
        self.push_entry(HistoryKind::Graph, label, generation);
    }

    /// Record a committed stroke (GPU snapshot already stored).
    pub fn push_stroke(&mut self, label: impl Into<String>, generation: u64) {
        self.push_entry(HistoryKind::Stroke, label, generation);
    }

    pub fn push_selection(&mut self, label: impl Into<String>, generation: u64) {
        self.push_entry(HistoryKind::Selection, label, generation);
    }

    pub fn push_transform(&mut self, label: impl Into<String>, generation: u64) {
        self.push_entry(HistoryKind::Transform, label, generation);
    }

    /// Pop the newest undo entry and describe what the host must reverse.
    pub fn undo_next(&mut self, graph: &mut DocumentGraph) -> Option<HistoryKind> {
        let entry = self.undo.pop()?;
        match entry.kind {
            HistoryKind::Graph => {
                let _ = self.graph.undo(graph);
            }
            HistoryKind::Stroke | HistoryKind::Selection | HistoryKind::Transform => {}
        }
        let kind = entry.kind;
        self.redo.push(entry);
        Some(kind)
    }

    /// Pop the newest redo entry and describe what the host must reapply.
    pub fn redo_next(&mut self, graph: &mut DocumentGraph) -> Option<HistoryKind> {
        let entry = self.redo.pop()?;
        match entry.kind {
            HistoryKind::Graph => {
                let _ = self.graph.redo(graph);
            }
            HistoryKind::Stroke | HistoryKind::Selection | HistoryKind::Transform => {}
        }
        let kind = entry.kind;
        self.undo.push(entry);
        Some(kind)
    }

    /// Number of undo steps needed to make `entry_id` the newest remaining entry.
    ///
    /// Returns `None` when the id is unknown. Jumping onto the current tip returns `Some(0)`.
    pub fn undo_steps_to_entry(&self, entry_id: u64) -> Option<usize> {
        let pos = self.undo.iter().position(|e| e.id == entry_id)?;
        Some(self.undo.len().saturating_sub(pos + 1))
    }

    pub fn entry_ids_newest_first(&self) -> Vec<u64> {
        self.undo.iter().rev().map(|e| e.id).collect()
    }

    pub fn kinds_newest_first(&self) -> Vec<&'static str> {
        self.undo
            .iter()
            .rev()
            .map(|e| match e.kind {
                HistoryKind::Graph => "graph",
                HistoryKind::Stroke => "stroke",
                HistoryKind::Selection => "selection",
                HistoryKind::Transform => "transform",
            })
            .collect()
    }
}

impl Default for HistoryService {
    fn default() -> Self {
        Self::new(128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentSize;
    use crate::undo::actions;

    #[test]
    fn timeline_interleaves_graph_and_stroke() {
        let mut g = DocumentGraph::new(DocumentSize::new(32, 32));
        let mut h = HistoryService::new(64);
        let id = g.add_layer_top(Some("X".into())).expect("add");
        let index = g.index_of(id).expect("index");
        let layer = g.get(id).cloned().expect("layer");
        h.push_graph_applied(
            crate::GraphCommand::AddLayer { id, index, layer },
            "Add layer",
            1,
        );
        h.push_stroke("Brush stroke", 2);
        assert_eq!(h.undo_next(&mut g), Some(HistoryKind::Stroke));
        assert_eq!(h.undo_next(&mut g), Some(HistoryKind::Graph));
        assert_eq!(g.layer_count(), 2);
        let _ = actions::add_layer;
    }
}
