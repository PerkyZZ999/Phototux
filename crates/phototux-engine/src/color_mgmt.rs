//! Document color profile metadata (handbook 16 / DR-012 — Phase 4.4 foundation).
//!
//! Assign changes interpretation only. Convert (pixel rewrite) is a separate future command.

use serde::{Deserialize, Serialize};

/// Working / document profile identity (not ICC bytes yet).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentColorState {
    /// Profile tag shown in UI (e.g. `sRGB`, `Display-P3`).
    pub assigned_profile: String,
    /// True when pixels were last written assuming `assigned_profile`.
    pub pixels_match_assigned: bool,
}

impl Default for DocumentColorState {
    fn default() -> Self {
        Self {
            assigned_profile: "sRGB".into(),
            pixels_match_assigned: true,
        }
    }
}

impl DocumentColorState {
    /// Assign a profile without rewriting pixels (DR-012).
    pub fn assign_profile(&mut self, profile: impl Into<String>) {
        let next = profile.into();
        if next != self.assigned_profile {
            self.assigned_profile = next;
            self.pixels_match_assigned = false;
        }
    }

    /// Record that a convert operation rewrote pixels to match the assigned profile.
    pub fn mark_converted(&mut self) {
        self.pixels_match_assigned = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_marks_mismatch() {
        let mut c = DocumentColorState::default();
        assert!(c.pixels_match_assigned);
        c.assign_profile("Display-P3");
        assert_eq!(c.assigned_profile, "Display-P3");
        assert!(!c.pixels_match_assigned);
        c.mark_converted();
        assert!(c.pixels_match_assigned);
    }
}
