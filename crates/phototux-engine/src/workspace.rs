//! Semantic workspace state (handbook 03 / DR-015) — separate from document dirty.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dock::DockTopology;
use crate::shell::{default_panels, essentials_panel_visibility};
use crate::workspace_preset::WorkspacePreset;

/// Focus / view / panel context — distinct from document selection (handbook 03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFocus {
    /// Active view id (`canvas` for the single-doc shell).
    pub active_view: String,
    /// Semantic focus path (`canvas`, `panel.layers`, …).
    pub focus_path: String,
    /// Panel that owns contextual chrome (may differ from keyboard focus).
    pub panel_context: String,
}

impl Default for WorkspaceFocus {
    fn default() -> Self {
        Self {
            active_view: "canvas".into(),
            focus_path: "canvas".into(),
            panel_context: String::new(),
        }
    }
}

/// Panel visibility, dock topology, and workspace revision.
/// Layout edits MUST NOT dirty the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub revision: u64,
    /// Panel descriptor id → visible.
    pub panel_visibility: BTreeMap<String, bool>,
    pub dock: DockTopology,
    #[serde(default)]
    pub focus: WorkspaceFocus,
    /// Last applied built-in / user preset id (empty = none).
    #[serde(default)]
    pub active_preset_id: String,
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
            focus: WorkspaceFocus::default(),
            active_preset_id: "workspace.preset.essentials".into(),
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

    /// Tab a group should actually show, given what is currently visible.
    ///
    /// `DockTopology` records which tab the user last raised, but it has no
    /// view of panel visibility — that lives here. Hiding the raised tab must
    /// fall through to a visible sibling rather than blanking the whole group,
    /// which is what happens when the stored selection is used unconditionally.
    /// Returns `None` when the group has no visible tab left, and the group is
    /// then legitimately absent from the dock.
    pub fn effective_active_tab(&self, panel_id: &str) -> Option<String> {
        let group = self
            .dock
            .right_groups()
            .into_iter()
            .find(|g| g.iter().any(|id| id == panel_id))?;
        let stored = self.dock.active_tab_of_group(panel_id);
        if let Some(stored) = stored.filter(|id| self.is_visible(id)) {
            return Some(stored);
        }
        group.into_iter().find(|id| self.is_visible(id))
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
        if self.panel_visibility != next.panel_visibility
            || self.dock != next.dock
            || self.active_preset_id != next.active_preset_id
        {
            self.panel_visibility = next.panel_visibility;
            self.dock = next.dock;
            self.active_preset_id = next.active_preset_id;
            self.revision = self.revision.saturating_add(1);
        }
    }

    /// Apply a named layout preset (visibility + dock). Preserves focus fields.
    pub fn apply_preset(&mut self, preset: &WorkspacePreset) {
        let changed = self.panel_visibility != preset.panel_visibility
            || self.dock != preset.dock
            || self.active_preset_id != preset.id;
        if !changed {
            return;
        }
        self.panel_visibility = preset.panel_visibility.clone();
        self.dock = preset.dock.clone();
        self.active_preset_id = preset.id.clone();
        self.revision = self.revision.saturating_add(1);
    }

    pub fn set_focus_path(&mut self, path: impl Into<String>) {
        let path = path.into();
        if self.focus.focus_path != path {
            self.focus.focus_path = path;
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub fn set_panel_context(&mut self, panel_id: impl Into<String>) {
        let panel_id = panel_id.into();
        if self.focus.panel_context != panel_id {
            self.focus.panel_context = panel_id;
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub fn set_active_view(&mut self, view_id: impl Into<String>) {
        let view_id = view_id.into();
        if self.focus.active_view != view_id {
            self.focus.active_view = view_id;
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

    /// Tear a docked panel into a floating window (layout-only).
    ///
    /// # Errors
    /// Returns a static reason when tear-off is invalid.
    pub fn tear_off_panel(
        &mut self,
        panel_id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        display_hint: &str,
    ) -> Result<(), &'static str> {
        self.dock
            .tear_off(panel_id, x, y, width, height, display_hint)?;
        let _ = self.set_visible(panel_id, true);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    /// Redock a floating panel into the right stack.
    ///
    /// # Errors
    /// Returns a static reason when the panel is not floating.
    pub fn redock_panel(&mut self, panel_id: &str) -> Result<(), &'static str> {
        self.dock.redock(panel_id, None)?;
        let _ = self.set_visible(panel_id, true);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    /// Persist floating window geometry after move/resize.
    ///
    /// # Errors
    /// Returns a static reason when the panel is not floating.
    pub fn set_floating_geometry(
        &mut self,
        panel_id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<(), &'static str> {
        self.dock
            .set_floating_geometry(panel_id, x, y, width, height)?;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    /// Clamp floating windows to a screen rect (restore / display change).
    pub fn clamp_floating(&mut self, screen: crate::dock::ScreenRect) {
        let before = self.dock.floating.clone();
        self.dock.clamp_floating_to_screen(screen);
        if self.dock.floating != before {
            self.revision = self.revision.saturating_add(1);
        }
    }

    /// Toggle auto-hide for a docked panel (layout-only).
    ///
    /// # Errors
    /// Returns a static reason when the panel cannot auto-hide.
    pub fn toggle_auto_hide(&mut self, panel_id: &str) -> Result<(), &'static str> {
        self.dock.toggle_auto_hide(panel_id)?;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    /// Pin (reveal) an auto-hidden panel.
    ///
    /// # Errors
    /// Returns a static reason when the panel is not auto-hidden.
    pub fn pin_panel(&mut self, panel_id: &str) -> Result<(), &'static str> {
        self.dock.pin(panel_id)?;
        self.revision = self.revision.saturating_add(1);
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

    /// Hiding the raised tab of a group must reveal a sibling, not blank the
    /// group. Regression: `dock.active_tab_of_group` has no view of visibility,
    /// so using it directly hid Swatches whenever Navigator was hidden.
    #[test]
    fn hiding_the_raised_tab_falls_through_to_a_visible_sibling() {
        let mut ws = WorkspaceState::essentials();
        let group = ws
            .dock
            .right_groups()
            .into_iter()
            .find(|g| g.len() > 1)
            .expect("essentials has a multi-tab group");
        let raised = group[0].clone();
        let sibling = group[1].clone();
        ws.dock.set_active_tab(&raised).expect("raise first tab");
        assert_eq!(ws.effective_active_tab(&raised), Some(raised.clone()));

        assert!(ws.set_visible(&raised, false));
        assert_eq!(
            ws.effective_active_tab(&sibling),
            Some(sibling.clone()),
            "hiding the raised tab must fall through to the visible sibling"
        );
    }

    #[test]
    fn a_group_with_no_visible_tab_has_no_active_tab() {
        let mut ws = WorkspaceState::essentials();
        let group = ws
            .dock
            .right_groups()
            .into_iter()
            .find(|g| g.len() > 1)
            .expect("essentials has a multi-tab group");
        for id in &group {
            assert!(ws.set_visible(id, false));
        }
        assert_eq!(ws.effective_active_tab(&group[0]), None);
    }

    #[test]
    fn effective_active_tab_prefers_the_raised_tab_while_it_is_visible() {
        let mut ws = WorkspaceState::essentials();
        let group = ws
            .dock
            .right_groups()
            .into_iter()
            .find(|g| g.len() > 1)
            .expect("essentials has a multi-tab group");
        ws.dock.set_active_tab(&group[1]).expect("raise second tab");
        assert_eq!(ws.effective_active_tab(&group[0]), Some(group[1].clone()));
    }
}
