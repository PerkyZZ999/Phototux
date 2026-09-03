//! Bounded undo/redo pairs for the state the engine does not own.
//!
//! Most undo is the engine's: a command records a transaction and the history
//! timeline replays it (handbook 20). Two things cannot go through that path,
//! because the state they would have to record lives on the GPU rather than in
//! the document graph — the pixel selection mask, and the layer buffers a
//! transform commit overwrites. The host keeps snapshots of those and steps
//! through them when the engine's history asks it to, via
//! [`phototux_engine::HostHistoryAction`].
//!
//! Both were written out by hand, twice each: a `Vec` for undo and one for
//! redo, a `push_*` method restating the bound and the "recording invalidates
//! the redo branch" rule, a `clear_*` method, and then — inside
//! `apply_host_history` — four copies of the same four-step dance, capture the
//! current state, pop from one stack, push what was captured onto the other,
//! restore. Getting the last two the wrong way round in one of the four is a
//! transposition nothing would catch: undo would appear to work and redo would
//! walk the wrong branch. None of it could be tested, because it sat inside a
//! method that talks to `phototux_canvas`.
//!
//! Here the same rules are one small generic type with no GPU in sight, and
//! the tests below are the first that have ever run over them.

use std::collections::VecDeque;

/// A bounded undo/redo pair over snapshots of type `T`.
///
/// The interface is deliberately narrower than two `Vec`s: a caller can record
/// a snapshot, step back, step forward, or drop everything, and cannot reach
/// past those to push onto the wrong side.
pub(crate) struct HostUndoStack<T> {
    undo: VecDeque<T>,
    redo: VecDeque<T>,
    limit: usize,
}

impl<T> HostUndoStack<T> {
    /// A stack that keeps at most `limit` steps of undo.
    ///
    /// The bound is per-stack because the snapshots are not the same size: a
    /// selection is one document-sized coverage mask, while a transform
    /// snapshot carries every layer's pixels.
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limit,
        }
    }

    /// Record the state as it is *before* an edit.
    ///
    /// This is also what discards the redo branch: once the user edits from a
    /// point they had stepped back to, the steps they had stepped back from
    /// are no longer reachable, and keeping them would let redo jump to a
    /// state that never followed this one.
    ///
    /// A `VecDeque` rather than a `Vec` so that trimming the oldest step is
    /// not a shift of everything after it.
    pub(crate) fn record(&mut self, snapshot: T) {
        self.undo.push_back(snapshot);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        self.redo.clear();
    }

    /// Step back, handing over the current state to be redone.
    ///
    /// `None` means there is nothing to go back to; `current` is dropped
    /// rather than stranded on the redo stack, so a caller that answers `None`
    /// with a fallback — clearing the selection, say — does not leave a redo
    /// step pointing at a state the user cannot return to.
    pub(crate) fn undo(&mut self, current: T) -> Option<T> {
        let previous = self.undo.pop_back()?;
        self.redo.push_back(current);
        Some(previous)
    }

    /// Step forward, handing over the current state to be undone.
    pub(crate) fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop_back()?;
        self.undo.push_back(current);
        Some(next)
    }

    /// Withdraw the snapshot just recorded, because the edit it was recorded
    /// for did not happen.
    ///
    /// Merge and flatten take their snapshot before invoking the command, so
    /// that it captures the pixels the command is about to replace. When the
    /// command refuses — a locked layer, nothing below to merge into — the
    /// snapshot describes a step the user never took, and leaving it would put
    /// a no-op on the undo stack.
    pub(crate) fn discard_last(&mut self) {
        self.undo.pop_back();
    }

    /// Forget both branches.
    ///
    /// Parking or closing a document leaves snapshots that describe a GPU no
    /// longer holding those pixels, so they must not survive it.
    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_back_and_forward_returns_to_where_it_started() {
        let mut stack = HostUndoStack::new(8);
        stack.record("a");
        stack.record("b");
        // The caller holds "c"; "b" is what it was before the last edit.
        assert_eq!(stack.undo("c"), Some("b"));
        assert_eq!(stack.undo("b"), Some("a"));
        assert_eq!(stack.redo("a"), Some("b"));
        assert_eq!(stack.redo("b"), Some("c"));
    }

    #[test]
    fn there_is_nothing_to_step_to_at_either_end() {
        let mut stack = HostUndoStack::new(8);
        assert_eq!(stack.undo("a"), None);
        assert_eq!(stack.redo("a"), None);
        stack.record("a");
        assert_eq!(stack.undo("b"), Some("a"));
        assert_eq!(stack.undo("a"), None, "the stack is empty again");
    }

    /// The transposition the four hand-written copies could each have had.
    #[test]
    fn stepping_back_does_not_lose_the_state_it_stepped_from() {
        let mut stack = HostUndoStack::new(8);
        stack.record("a");
        assert_eq!(stack.undo("b"), Some("a"));
        assert_eq!(stack.redo("a"), Some("b"), "b must still be reachable");
    }

    /// Failing to step back must not leave a redo step behind.
    ///
    /// The selection's caller answers `None` by clearing the selection
    /// outright, so a `current` stranded on the redo stack would offer a redo
    /// to a state the undo path never produced.
    #[test]
    fn a_step_back_that_finds_nothing_strands_nothing() {
        let mut stack = HostUndoStack::new(8);
        assert_eq!(stack.undo("a"), None);
        assert_eq!(stack.redo("b"), None, "nothing was recorded to redo to");
    }

    #[test]
    fn recording_after_stepping_back_drops_the_branch_left_behind() {
        let mut stack = HostUndoStack::new(8);
        stack.record("a");
        assert_eq!(stack.undo("b"), Some("a"));
        stack.record("a");
        assert_eq!(
            stack.redo("c"),
            None,
            "b never followed this edit and must not be reachable"
        );
    }

    #[test]
    fn the_oldest_step_falls_off_the_bottom() {
        let mut stack = HostUndoStack::new(2);
        stack.record(1);
        stack.record(2);
        stack.record(3);
        assert_eq!(stack.undo(4), Some(3));
        assert_eq!(stack.undo(3), Some(2));
        assert_eq!(stack.undo(2), None, "1 was trimmed to keep the bound");
    }

    #[test]
    fn a_withdrawn_snapshot_is_not_a_step() {
        let mut stack = HostUndoStack::new(8);
        stack.record("a");
        stack.record("b");
        stack.discard_last();
        assert_eq!(stack.undo("b"), Some("a"), "b was never a step taken");
    }

    #[test]
    fn clearing_forgets_both_branches() {
        let mut stack = HostUndoStack::new(8);
        stack.record("a");
        assert_eq!(stack.undo("b"), Some("a"));
        stack.clear();
        assert_eq!(stack.undo("c"), None);
        assert_eq!(stack.redo("c"), None);
    }
}
