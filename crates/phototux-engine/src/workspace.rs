//! Semantic workspace state (handbook 03 / DR-015) — separate from document dirty.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dock::DockTopology;
use crate::shell::{default_panels, essentials_panel_visibility};

/// Panel visibility, dock topology, and workspace revision.
/// Layout edits MUST NOT dirty the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub revision: u64,
    /// Panel descriptor id → visible.
    pub panel_visibility: BTreeMap<String, bool>,
    pub dock: DockTopology,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self::essentials()
    }
}

impl WorkspaceState {
    /// Built-in Essentials visibility from panel descriptors.
    pub fn essentials() -> Self {
        let mut panel_visibility = BTreeMap::new();
        for (id, visible) in essentials_panel_visibility() {
            panel_visibility.insert(id, visible);
        }
        Self {
            revision: 0,
            panel_visibility,
            dock: DockTopology::essentials(),
        }
    }

    pub fn is_visible(&self, panel_id: &str) -> bool {
        self.panel_visibility
            .get(panel_id)
            .copied()
            .unwrap_or_else(|| {
                default_panels()
                    .into_iter()
                    .find(|p| p.id == panel_id)
                    .map(|p| p.visible_by_default)
                    .unwrap_or(false)
            })
    }

    pub fn set_visible(&mut self, panel_id: &str, visible: bool) -> bool {
        if !default_panels().iter().any(|p| p.id == panel_id) {
            return false;
        }
        let prev = self.is_visible(panel_id);
        self.panel_visibility.insert(panel_id.to_owned(), visible);
        if prev != visible {
            self.revision = self.revision.saturating_add(1);
        }
        true
    }

    pub fn toggle(&mut self, panel_id: &str) -> bool {
        let next = !self.is_visible(panel_id);
        self.set_visible(panel_id, next)
    }

    pub fn reset_essentials(&mut self) {
        let next = Self::essentials();
        if self.panel_visibility != next.panel_visibility || self.dock != next.dock {
            self.panel_visibility = next.panel_visibility;
            self.dock = next.dock;
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub fn set_dock(&mut self, dock: DockTopology) -> Result<(), &'static str> {
        dock.validate()?;
        if self.dock != dock {
            self.dock = dock;
            self.revision = self.revision.saturating_add(1);
        }
        Ok(())
    }

    /// Reorder a docked panel within the right stack (layout-only; never dirties document).
    ///
    /// # Errors
    /// Returns a static reason when the move is invalid.
    pub fn move_panel_in_stack(&mut self, panel_id: &str, delta: i32) -> Result<(), &'static str> {
        let before = self.dock.clone();
        self.dock.move_in_stack(panel_id, delta)?;
        if self.dock != before {
            self.revision = self.revision.saturating_add(1);
        }
        Ok(())
    }

    /// Absolute reorder by stack indices.
    ///
    /// # Errors
    /// Returns a static reason when indices are invalid.
    pub fn reorder_panel_in_stack(&mut self, from: usize, to: usize) -> Result<(), &'static str> {
        let before = self.dock.clone();
        self.dock.reorder(from, to)?;
        if self.dock != before {
            self.revision = self.revision.saturating_add(1);
        }
        Ok(())
    }

    /// JSON object for prefs / QML: `{ "panel.layers": true, … }`.
    pub fn visibility_json(&self) -> String {
        serde_json::to_string(&self.panel_visibility).unwrap_or_else(|_| "{}".into())
    }

    pub fn from_visibility_map(map: BTreeMap<String, bool>) -> Self {
        let mut ws = Self::essentials();
        for (id, visible) in map {
            let _ = ws.set_visible(&id, visible);
        }
        // set_visible bumps revision; treat loaded state as baseline.
        ws.revision = 0;
        ws
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SessionState, SizePreset};

    #[test]
    fn essentials_matches_descriptors() {
        let ws = WorkspaceState::essentials();
        assert!(ws.is_visible("panel.layers"));
        assert!(!ws.is_visible("panel.paths"));
        assert!(!ws.is_visible("panel.character"));
    }

    #[test]
    fn workspace_ops_do_not_touch_document_generation() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let doc_gen = session.document_generation();
        let dirty = session.is_dirty_vs_persisted();

        let mut ws = WorkspaceState::essentials();
        assert!(ws.set_visible("panel.navigator", false));
        ws.toggle("panel.history");
        ws.reset_essentials();

        assert_eq!(session.document_generation(), doc_gen);
        assert_eq!(session.is_dirty_vs_persisted(), dirty);
        assert!(ws.revision > 0);
    }

    #[test]
    fn rejects_unknown_panel() {
        let mut ws = WorkspaceState::essentials();
        assert!(!ws.set_visible("panel.nope", true));
    }

    #[test]
    fn move_panel_bumps_revision_not_document() {
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let doc_gen = session.document_generation();
        let mut ws = WorkspaceState::essentials();
        let rev = ws.revision;
        ws.move_panel_in_stack("panel.history", -1).expect("move");
        assert!(ws.revision > rev);
        assert_eq!(session.document_generation(), doc_gen);
    }
}
