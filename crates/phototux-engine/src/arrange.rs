//! Moving a layer through the stack by one step or all the way (handbook 11).
//!
//! The engine has had `layer.reorder` since the stack existed, and it takes an
//! absolute destination index. Nothing in the shell ever called it: the panel
//! has no drag, and the arrows in its header move the *panel*, not the layer.
//! So a fully implemented, fully undoable command sat unreachable, and the only
//! way to change the stacking order of a document was to delete and re-add
//! layers in the order you wanted them.
//!
//! Photoshop's Layer ▸ Arrange is four entries and four chords, and the
//! arithmetic that turns "forward" into a destination index is small enough to
//! get wrong quietly — an off-by-one here swaps the wrong pair, and a clamp
//! that saturates the wrong way makes Bring to Front a no-op on the layer that
//! most needs it. It lives here so it can be tested without a stack, a session
//! or a device (DR-022).

/// Which way through the stack, and how far.
///
/// Named variants rather than a raw string with a `_` arm, for the reason
/// [`crate::ShapePreset::parse`] gives: an unrecognised op must refuse rather
/// than silently pick a direction, because a layer moved somewhere the user did
/// not ask for is a document change they then have to notice and undo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrangeOp {
    /// Up one place in the stack — Photoshop's Bring Forward.
    Forward,
    /// Down one place.
    Backward,
    /// To the very top.
    Front,
    /// To the very bottom.
    Back,
}

impl ArrangeOp {
    /// Every op, in menu order, for exhaustiveness checks against the registry.
    pub const ALL: [Self; 4] = [Self::Front, Self::Forward, Self::Backward, Self::Back];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Backward => "backward",
            Self::Front => "front",
            Self::Back => "back",
        }
    }

    /// Display name for the menu, without its accelerator marker.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "Bring Forward",
            Self::Backward => "Send Backward",
            Self::Front => "Bring to Front",
            Self::Back => "Send to Back",
        }
    }

    /// Photoshop's default chord for this entry.
    #[must_use]
    pub fn shortcut(self) -> &'static str {
        match self {
            Self::Forward => "Ctrl+]",
            Self::Backward => "Ctrl+[",
            Self::Front => "Ctrl+Shift+]",
            Self::Back => "Ctrl+Shift+[",
        }
    }

    /// Parse a registry argument; `None` when it names no op.
    #[must_use]
    pub fn parse(op: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == op)
    }

    /// Where a run of `moving` layers goes, given the stack around it.
    ///
    /// `from` is the index the run currently starts at once the moving layers
    /// have been lifted out, `moving` how many of them there are, and `rest`
    /// how many layers are left behind. The answer is an index into `rest`,
    /// which is what [`crate::CommandArgs::Reorder`] takes.
    ///
    /// A step is one place, not one index: lifting the run out already shifts
    /// everything above it down by however many are moving, so "up one" lands
    /// at `from + 1` in the shortened list and "down one" at `from - 1`.
    /// Writing that out at the call site is exactly where the off-by-one lives.
    #[must_use]
    pub fn destination(self, from: usize, rest: usize) -> usize {
        match self {
            Self::Forward => (from + 1).min(rest),
            Self::Backward => from.saturating_sub(1),
            Self::Front => rest,
            Self::Back => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_op_round_trips_through_its_wire_name() {
        for op in ArrangeOp::ALL {
            assert_eq!(ArrangeOp::parse(op.as_str()), Some(op));
        }
        assert_eq!(ArrangeOp::parse("sideways"), None);
    }

    #[test]
    fn every_op_has_its_own_label_chord_and_name() {
        for (i, op) in ArrangeOp::ALL.iter().enumerate() {
            for other in &ArrangeOp::ALL[i + 1..] {
                assert_ne!(op.as_str(), other.as_str());
                assert_ne!(op.label(), other.label());
                assert_ne!(op.shortcut(), other.shortcut());
            }
        }
    }

    /// One step moves one place, in the list the layer has been lifted out of.
    #[test]
    fn a_step_moves_one_place() {
        // Five layers, the middle one moving: rest is four.
        assert_eq!(ArrangeOp::Forward.destination(2, 4), 3);
        assert_eq!(ArrangeOp::Backward.destination(2, 4), 1);
    }

    /// The ends are where a step has nowhere to go, and where the command must
    /// report "already there" rather than silently doing nothing.
    #[test]
    fn a_step_at_the_end_stays_put() {
        assert_eq!(ArrangeOp::Forward.destination(4, 4), 4, "already on top");
        assert_eq!(
            ArrangeOp::Backward.destination(0, 4),
            0,
            "already at the bottom"
        );
    }

    #[test]
    fn the_ends_go_all_the_way() {
        assert_eq!(ArrangeOp::Front.destination(1, 6), 6);
        assert_eq!(ArrangeOp::Back.destination(5, 6), 0);
    }

    /// A run of several layers moves as one, so the arithmetic must not depend
    /// on how many are moving — `rest` already excludes them.
    #[test]
    fn a_run_moves_the_same_way_a_single_layer_does() {
        assert_eq!(ArrangeOp::Forward.destination(1, 2), 2);
        assert_eq!(ArrangeOp::Front.destination(0, 2), 2);
        assert_eq!(ArrangeOp::Back.destination(2, 2), 0);
    }
}
