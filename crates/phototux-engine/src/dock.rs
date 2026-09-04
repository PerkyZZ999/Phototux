//! Minimal dock topology (handbook 04) — canvas + right stack + floating tear-offs.

use serde::{Deserialize, Serialize};

use crate::shell::default_panels;

/// Geometry for a torn-off floating panel (display-local logical pixels).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloatingPanelPlacement {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Optional display / output hint for restore (host-defined).
    #[serde(default)]
    pub display_hint: String,
    /// Where in `right_stack` this panel was, so re-docking is a round trip.
    ///
    /// Without it a re-dock appended, so a panel torn from the middle of the
    /// stack came back at the bottom as a group of its own — tear off and dock
    /// again and the workspace was not what it had been.
    #[serde(default)]
    pub dock_index: usize,
    /// Whether it shared a group with the panel above it.
    #[serde(default)]
    pub tabbed: bool,
}

/// Axis-aligned screen rect used when clamping floating windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Versioned dock layout for the desktop shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockTopology {
    pub version: u32,
    /// Ordered panel ids in the right dock stack (top → bottom).
    pub right_stack: Vec<String>,
    /// Panels torn off into floating windows (not in [`Self::right_stack`]).
    #[serde(default)]
    pub floating: Vec<FloatingPanelPlacement>,
    /// Docked panels collapsed to an edge strip (still in [`Self::right_stack`]).
    #[serde(default)]
    pub auto_hidden: Vec<String>,
    /// Panels that share the tab group of the panel above them in
    /// [`Self::right_stack`].
    ///
    /// Grouping is derived from the stack rather than stored as nested lists so
    /// that ordering, move, tear-off and auto-hide keep operating on one flat
    /// sequence. Empty means every panel is its own group, which is the
    /// pre-grouping layout.
    #[serde(default)]
    pub tabbed_with_previous: Vec<String>,
    /// Selected tab per group, by panel id. A group with no entry shows its
    /// first panel.
    #[serde(default)]
    pub active_tabs: Vec<String>,
    /// Body height in pixels for panels the user has resized, by panel id.
    ///
    /// Absent means "use the shell's default for that panel", which is what
    /// every panel starts with — storing a height for one the user has never
    /// touched would freeze a layout decision the shell should still be free to
    /// change. A `BTreeMap` so the serialised order is stable and two identical
    /// layouts compare equal.
    #[serde(default)]
    pub panel_heights: std::collections::BTreeMap<String, u32>,
}

impl Default for DockTopology {
    fn default() -> Self {
        Self::essentials()
    }
}

impl DockTopology {
    pub const CURRENT_VERSION: u32 = 2;

    /// Essentials layout: three tabbed groups rather than five stacked panels.
    ///
    /// Stacking every panel gave the lower ones no usable height at ordinary
    /// window sizes, and grouping is what raster-editor users expect anyway.
    pub fn essentials() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            right_stack: vec![
                "panel.properties".into(),
                "panel.navigator".into(),
                "panel.swatches".into(),
                "panel.layers".into(),
                "panel.history".into(),
            ],
            floating: Vec::new(),
            auto_hidden: Vec::new(),
            tabbed_with_previous: vec!["panel.swatches".into(), "panel.history".into()],
            active_tabs: Vec::new(),
            panel_heights: std::collections::BTreeMap::new(),
        }
    }

    /// Right dock as tab groups, top to bottom.
    pub fn right_groups(&self) -> Vec<Vec<String>> {
        let mut groups: Vec<Vec<String>> = Vec::new();
        for id in &self.right_stack {
            let joins_previous =
                !groups.is_empty() && self.tabbed_with_previous.iter().any(|t| t == id);
            if joins_previous {
                if let Some(last) = groups.last_mut() {
                    last.push(id.clone());
                }
            } else {
                groups.push(vec![id.clone()]);
            }
        }
        groups
    }

    /// Selected panel of the group containing `panel_id`, or `None` when it is
    /// not docked.
    pub fn active_tab_of_group(&self, panel_id: &str) -> Option<String> {
        let group = self
            .right_groups()
            .into_iter()
            .find(|g| g.iter().any(|id| id == panel_id))?;
        let selected = group
            .iter()
            .find(|id| self.active_tabs.iter().any(|a| a == *id))
            .cloned();
        selected.or_else(|| group.first().cloned())
    }

    /// Smallest and largest body height a panel may be dragged to.
    ///
    /// The floor is enough to show a header's worth of content, so a panel
    /// cannot be collapsed into an unrecoverable sliver — the user would have
    /// no handle left to drag it back out with. The ceiling keeps one panel
    /// from taking the whole dock and leaving the others with nothing.
    pub const MIN_PANEL_HEIGHT: u32 = 64;
    pub const MAX_PANEL_HEIGHT: u32 = 2000;

    /// The stored height for `panel_id`, or `None` to use the shell's default.
    #[must_use]
    pub fn panel_height(&self, panel_id: &str) -> Option<u32> {
        self.panel_heights.get(panel_id).copied()
    }

    /// Record a dragged height, clamped to the usable range.
    ///
    /// # Errors
    /// Returns a static reason when the panel is not docked — a height for a
    /// floating or unknown panel would be dead state that `validate` then has
    /// to tolerate forever.
    pub fn set_panel_height(&mut self, panel_id: &str, height: u32) -> Result<(), &'static str> {
        if !self.is_docked(panel_id) {
            return Err("panel is not docked");
        }
        self.panel_heights.insert(
            panel_id.to_owned(),
            height.clamp(Self::MIN_PANEL_HEIGHT, Self::MAX_PANEL_HEIGHT),
        );
        Ok(())
    }

    /// Raise `panel_id` to be the visible tab of its group.
    ///
    /// # Errors
    /// Returns a static reason when the panel is not docked.
    pub fn set_active_tab(&mut self, panel_id: &str) -> Result<(), &'static str> {
        let group = self
            .right_groups()
            .into_iter()
            .find(|g| g.iter().any(|id| id == panel_id))
            .ok_or("panel not in right_stack")?;
        // At most one selection per group, so clear the group's siblings first.
        self.active_tabs.retain(|id| !group.contains(id));
        self.active_tabs.push(panel_id.to_owned());
        self.normalize_groups();
        self.validate()
    }

    /// Restore the grouping invariants after the stack changes.
    ///
    /// Tearing off, redocking or reordering can leave a group reference to a
    /// panel that is no longer docked, or promote a panel that joins a previous
    /// group to the head of the stack where there is nothing to join. Rather
    /// than repeat that reasoning at every mutation site, each one normalizes.
    pub(crate) fn normalize_groups(&mut self) {
        self.tabbed_with_previous
            .retain(|id| self.right_stack.contains(id));
        self.active_tabs.retain(|id| self.right_stack.contains(id));
        // A height for an undocked panel is dead state that `validate` would
        // then have to tolerate for ever.
        let docked = self.right_stack.clone();
        self.panel_heights.retain(|id, _| docked.contains(id));
        if let Some(first) = self.right_stack.first().cloned() {
            self.tabbed_with_previous.retain(|id| *id != first);
        }
    }

    /// Adopt the current default grouping for a topology saved before groups
    /// existed.
    ///
    /// Presentation state, so migrating rather than preserving the old stack is
    /// safe — and preserving it would leave existing users on the layout this
    /// replaced.
    fn migrate_to_groups(&mut self) {
        if self.version >= 2 {
            return;
        }
        let defaults = Self::essentials();
        self.tabbed_with_previous = defaults
            .tabbed_with_previous
            .into_iter()
            .filter(|id| self.is_docked(id))
            .collect();
        self.version = Self::CURRENT_VERSION;
    }

    /// Index of `panel_id` in the right stack, if present.
    pub fn stack_index(&self, panel_id: &str) -> Option<usize> {
        self.right_stack.iter().position(|id| id == panel_id)
    }

    pub fn is_docked(&self, panel_id: &str) -> bool {
        self.stack_index(panel_id).is_some()
    }

    pub fn is_floating(&self, panel_id: &str) -> bool {
        self.floating.iter().any(|f| f.id == panel_id)
    }

    pub fn is_auto_hidden(&self, panel_id: &str) -> bool {
        self.auto_hidden.iter().any(|id| id == panel_id)
    }

    /// Collapse a docked panel to the edge strip (keyboard/pin reopen required).
    ///
    /// # Errors
    /// Returns a static reason when the panel is not docked or already hidden.
    pub fn auto_hide(&mut self, panel_id: &str) -> Result<(), &'static str> {
        if !self.is_docked(panel_id) {
            return Err("panel not in right_stack");
        }
        if self.is_auto_hidden(panel_id) {
            return Ok(());
        }
        self.auto_hidden.push(panel_id.to_owned());
        self.normalize_groups();
        self.validate()
    }

    /// Pin / reveal an auto-hidden panel back to the full dock body.
    ///
    /// # Errors
    /// Returns a static reason when the panel is not auto-hidden.
    pub fn pin(&mut self, panel_id: &str) -> Result<(), &'static str> {
        let pos = self
            .auto_hidden
            .iter()
            .position(|id| id == panel_id)
            .ok_or("panel not auto-hidden")?;
        self.auto_hidden.remove(pos);
        self.normalize_groups();
        self.validate()
    }

    pub fn toggle_auto_hide(&mut self, panel_id: &str) -> Result<(), &'static str> {
        if self.is_auto_hidden(panel_id) {
            self.pin(panel_id)
        } else {
            self.auto_hide(panel_id)
        }
    }

    /// Move a panel within the right stack by delta (−1 = up, +1 = down).
    ///
    /// # Errors
    /// Returns a static reason when the panel is missing or the move is a no-op at the edge.
    pub fn move_in_stack(&mut self, panel_id: &str, delta: i32) -> Result<(), &'static str> {
        if delta == 0 {
            return Ok(());
        }
        let from = self
            .stack_index(panel_id)
            .ok_or("panel not in right_stack")?;
        let to = from as i32 + delta;
        if to < 0 || to as usize >= self.right_stack.len() {
            return Err("move out of stack bounds");
        }
        let to = to as usize;
        self.right_stack.swap(from, to);
        self.normalize_groups();
        self.validate()
    }

    /// Move panel from `from` index to `to` index (both must be in range).
    ///
    /// # Errors
    /// Returns a static reason when indices are invalid.
    pub fn reorder(&mut self, from: usize, to: usize) -> Result<(), &'static str> {
        let len = self.right_stack.len();
        if from >= len || to >= len {
            return Err("reorder index out of range");
        }
        if from == to {
            return Ok(());
        }
        let id = self.right_stack.remove(from);
        self.right_stack.insert(to, id);
        self.normalize_groups();
        self.validate()
    }

    /// Tear a docked panel into a floating window. Keeps at least one docked panel.
    ///
    /// # Errors
    /// Returns a static reason when tear-off is invalid.
    pub fn tear_off(
        &mut self,
        panel_id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        display_hint: impl Into<String>,
    ) -> Result<(), &'static str> {
        if self.right_stack.len() <= 1 {
            return Err("cannot tear off last docked panel");
        }
        let idx = self
            .stack_index(panel_id)
            .ok_or("panel not in right_stack")?;
        if self.is_floating(panel_id) {
            return Err("panel already floating");
        }
        let width = width.max(200);
        let height = height.max(120);
        let tabbed = self.tabbed_with_previous.iter().any(|id| id == panel_id);
        self.right_stack.remove(idx);
        self.floating.push(FloatingPanelPlacement {
            id: panel_id.to_owned(),
            x,
            y,
            width,
            height,
            display_hint: display_hint.into(),
            dock_index: idx,
            tabbed,
        });
        self.normalize_groups();
        self.validate()
    }

    /// Return a floating panel to the right stack.
    ///
    /// `at` is an explicit drop position — a drag onto the stack. Without one
    /// the panel goes back where it came from, group and all: tearing off and
    /// docking again is a round trip, not a move to the bottom.
    ///
    /// # Errors
    /// Returns a static reason when the panel is not floating.
    pub fn redock(&mut self, panel_id: &str, at: Option<usize>) -> Result<(), &'static str> {
        let pos = self
            .floating
            .iter()
            .position(|f| f.id == panel_id)
            .ok_or("panel not floating")?;
        let placement = self.floating.remove(pos);
        let insert_at = at
            .unwrap_or(placement.dock_index)
            .min(self.right_stack.len());
        self.right_stack.insert(insert_at, panel_id.to_owned());
        // Only for a panel going home. An explicit drop position is the user
        // saying where it belongs now, and joining it to whatever happens to
        // be above that point would be a group they did not ask for.
        if at.is_none()
            && placement.tabbed
            && insert_at > 0
            && !self.tabbed_with_previous.iter().any(|id| id == panel_id)
        {
            self.tabbed_with_previous.push(panel_id.to_owned());
        }
        self.normalize_groups();
        self.validate()
    }

    /// Update floating geometry (after user drag/resize).
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
        let panel = self
            .floating
            .iter_mut()
            .find(|f| f.id == panel_id)
            .ok_or("panel not floating")?;
        panel.x = x;
        panel.y = y;
        panel.width = width.max(200);
        panel.height = height.max(120);
        Ok(())
    }

    /// Clamp all floating windows so a title-bar-sized strip stays on-screen.
    pub fn clamp_floating_to_screen(&mut self, screen: ScreenRect) {
        const TITLE: i32 = 32;
        let max_x = screen.x + screen.width.saturating_sub(80) as i32;
        let max_y = screen.y + screen.height.saturating_sub(TITLE as u32) as i32;
        for panel in &mut self.floating {
            panel.x = panel.x.clamp(screen.x, max_x);
            panel.y = panel.y.clamp(screen.y, max_y);
            let max_w = screen.width.max(200);
            let max_h = screen.height.max(120);
            panel.width = panel.width.clamp(200, max_w);
            panel.height = panel.height.clamp(120, max_h);
        }
    }

    /// Validate panel ids against the built-in catalog.
    ///
    /// # Errors
    /// Returns a static reason when the topology is invalid.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version == 0 || self.version > Self::CURRENT_VERSION {
            return Err("unsupported dock topology version");
        }
        if self.right_stack.is_empty() {
            return Err("right_stack must not be empty");
        }
        let known: Vec<_> = default_panels().into_iter().map(|p| p.id).collect();
        let mut seen = std::collections::HashSet::new();
        validate_right_stack(&self.right_stack, &known, &mut seen)?;
        validate_floating(&self.floating, &known, &mut seen)?;
        validate_auto_hidden(&self.auto_hidden, |id| self.is_docked(id))?;
        if self
            .tabbed_with_previous
            .iter()
            .any(|id| !self.is_docked(id))
        {
            return Err("tabbed_with_previous names a panel that is not docked");
        }
        // The first panel has nothing above it to join.
        if self
            .right_stack
            .first()
            .is_some_and(|first| self.tabbed_with_previous.iter().any(|id| id == first))
        {
            return Err("the first docked panel cannot join a previous group");
        }
        if self.active_tabs.iter().any(|id| !self.is_docked(id)) {
            return Err("active_tabs names a panel that is not docked");
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate().map_err(str::to_owned)?;
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut topo: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        topo.migrate_to_groups();
        topo.validate().map_err(str::to_owned)?;
        Ok(topo)
    }
}

fn validate_right_stack<'a>(
    right_stack: &'a [String],
    known: &[String],
    seen: &mut std::collections::HashSet<&'a str>,
) -> Result<(), &'static str> {
    for id in right_stack {
        if !known.iter().any(|k| k == id) {
            return Err("unknown panel id in right_stack");
        }
        if !seen.insert(id.as_str()) {
            return Err("duplicate panel id in right_stack");
        }
    }
    Ok(())
}

fn validate_floating<'a>(
    floating: &'a [FloatingPanelPlacement],
    known: &[String],
    seen: &mut std::collections::HashSet<&'a str>,
) -> Result<(), &'static str> {
    for panel in floating {
        if !known.iter().any(|k| k == &panel.id) {
            return Err("unknown panel id in floating");
        }
        if !seen.insert(panel.id.as_str()) {
            return Err("panel both docked and floating");
        }
        if panel.width < 200 || panel.height < 120 {
            return Err("floating geometry too small");
        }
    }
    Ok(())
}

fn validate_auto_hidden(
    auto_hidden: &[String],
    is_docked: impl Fn(&str) -> bool,
) -> Result<(), &'static str> {
    let mut auto_seen = std::collections::HashSet::new();
    for id in auto_hidden {
        if !is_docked(id) {
            return Err("auto-hidden panel must be docked");
        }
        if !auto_seen.insert(id.as_str()) {
            return Err("duplicate auto-hidden panel");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn essentials_valid_and_roundtrips() {
        let topo = DockTopology::essentials();
        assert!(topo.validate().is_ok());
        let json = topo.to_json().expect("ser");
        let back = DockTopology::from_json(&json).expect("de");
        assert_eq!(topo, back);
    }

    #[test]
    fn rejects_unknown_and_duplicate() {
        let mut bad = DockTopology::essentials();
        bad.right_stack.push("panel.nope".into());
        assert!(bad.validate().is_err());
        bad = DockTopology::essentials();
        bad.right_stack.push("panel.layers".into());
        assert!(bad.validate().is_err());
    }

    #[test]
    fn move_in_stack_reorders() {
        let mut topo = DockTopology::essentials();
        topo.move_in_stack("panel.layers", -1).expect("up");
        assert_eq!(topo.right_stack[2], "panel.layers");
        assert_eq!(topo.right_stack[3], "panel.swatches");
        assert!(topo.move_in_stack("panel.properties", -1).is_err());
    }

    #[test]
    fn reorder_moves_by_index() {
        let mut topo = DockTopology::essentials();
        topo.reorder(0, 4).expect("to end");
        assert_eq!(topo.right_stack[4], "panel.properties");
        assert_eq!(topo.right_stack[0], "panel.navigator");
    }

    #[test]
    fn tear_off_and_redock() {
        let mut topo = DockTopology::essentials();
        topo.tear_off("panel.history", 40, 40, 320, 240, "")
            .expect("tear");
        assert!(topo.is_floating("panel.history"));
        assert!(!topo.is_docked("panel.history"));
        assert_eq!(topo.right_stack.len(), 4);
        topo.redock("panel.history", None).expect("dock");
        assert!(topo.is_docked("panel.history"));
        assert!(topo.floating.is_empty());
    }

    #[test]
    fn essentials_groups_five_panels_into_three() {
        let topo = DockTopology::essentials();
        let groups = topo.right_groups();
        assert_eq!(groups.len(), 3, "{groups:?}");
        assert_eq!(groups[0], vec!["panel.properties"]);
        assert_eq!(groups[1], vec!["panel.navigator", "panel.swatches"]);
        assert_eq!(groups[2], vec!["panel.layers", "panel.history"]);
        // No explicit selection yet: each group shows its first tab.
        assert_eq!(
            topo.active_tab_of_group("panel.history").as_deref(),
            Some("panel.layers")
        );
    }

    #[test]
    fn raising_a_tab_replaces_only_its_own_group() {
        let mut topo = DockTopology::essentials();
        topo.set_active_tab("panel.history").expect("raise");
        topo.set_active_tab("panel.swatches").expect("raise");
        assert_eq!(
            topo.active_tab_of_group("panel.layers").as_deref(),
            Some("panel.history")
        );
        assert_eq!(
            topo.active_tab_of_group("panel.navigator").as_deref(),
            Some("panel.swatches")
        );
        // Raising the sibling deselects the first without touching other groups.
        topo.set_active_tab("panel.layers").expect("raise");
        assert_eq!(
            topo.active_tab_of_group("panel.history").as_deref(),
            Some("panel.layers")
        );
        assert_eq!(
            topo.active_tab_of_group("panel.swatches").as_deref(),
            Some("panel.swatches")
        );
    }

    /// Tearing off or reordering must not leave a group naming a panel that is
    /// gone, nor leave the head of the stack joining a group above it.
    #[test]
    fn stack_changes_keep_grouping_consistent() {
        let mut topo = DockTopology::essentials();
        topo.set_active_tab("panel.history").expect("raise");
        topo.tear_off("panel.history", 10, 10, 300, 200, "")
            .expect("tear");
        assert!(topo.validate().is_ok());
        assert!(
            !topo
                .tabbed_with_previous
                .iter()
                .any(|id| id == "panel.history")
        );
        assert_eq!(
            topo.active_tab_of_group("panel.layers").as_deref(),
            Some("panel.layers")
        );

        // Promote a grouped panel to the head: it can no longer join anything.
        let mut topo = DockTopology::essentials();
        topo.reorder(2, 0).expect("reorder swatches to head");
        assert!(topo.validate().is_ok());
        assert_eq!(topo.right_groups()[0], vec!["panel.swatches"]);
    }

    /// Presentation state, so a topology saved before grouping adopts the
    /// current default rather than keeping the layout it replaced.
    #[test]
    fn a_pre_group_topology_migrates_to_the_default_grouping() {
        let v1 = r#"{"version":1,"right_stack":["panel.properties","panel.navigator","panel.swatches","panel.layers","panel.history"],"floating":[],"auto_hidden":[]}"#;
        let topo = DockTopology::from_json(v1).expect("v1 loads");
        assert_eq!(topo.version, DockTopology::CURRENT_VERSION);
        assert_eq!(topo.right_groups().len(), 3);

        // A v1 file missing some panels only adopts the grouping that applies.
        let partial = r#"{"version":1,"right_stack":["panel.properties","panel.layers"],"floating":[],"auto_hidden":[]}"#;
        let topo = DockTopology::from_json(partial).expect("partial v1 loads");
        assert!(topo.validate().is_ok());
        assert_eq!(topo.right_groups().len(), 2);
    }

    #[test]
    fn cannot_tear_off_last() {
        let mut topo = DockTopology {
            version: DockTopology::CURRENT_VERSION,
            right_stack: vec!["panel.layers".into()],
            floating: Vec::new(),
            auto_hidden: Vec::new(),
            tabbed_with_previous: Vec::new(),
            active_tabs: Vec::new(),
            panel_heights: std::collections::BTreeMap::new(),
        };
        assert!(topo.tear_off("panel.layers", 0, 0, 300, 200, "").is_err());
    }

    #[test]
    fn clamp_keeps_strip_on_screen() {
        let mut topo = DockTopology::essentials();
        topo.tear_off("panel.navigator", -5000, -5000, 300, 200, "")
            .expect("tear");
        topo.clamp_floating_to_screen(ScreenRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        });
        let f = &topo.floating[0];
        assert!(f.x >= 0);
        assert!(f.y >= 0);
    }

    #[test]
    fn clamp_after_screen_shrink_keeps_panel_visible() {
        let mut topo = DockTopology::essentials();
        topo.tear_off("panel.history", 1600, 900, 320, 240, "")
            .expect("tear");
        // Simulated display change / windowing resize to a smaller screen.
        topo.clamp_floating_to_screen(ScreenRect {
            x: 0,
            y: 0,
            width: 1440,
            height: 900,
        });
        let f = &topo.floating[0];
        assert!(
            f.x + 80 <= 1440,
            "title strip must remain horizontally reachable"
        );
        assert!(
            f.y + 32 <= 900,
            "title strip must remain vertically reachable"
        );
        assert!(f.x >= 0 && f.y >= 0);
        assert!(f.width <= 1440 && f.height <= 900);
    }

    #[test]
    fn legacy_json_without_floating_loads() {
        let json = r#"{"version":1,"right_stack":["panel.properties","panel.layers"]}"#;
        let topo = DockTopology::from_json(json).expect("de");
        assert!(topo.floating.is_empty());
        assert!(topo.auto_hidden.is_empty());
    }

    #[test]
    fn auto_hide_and_pin() {
        let mut topo = DockTopology::essentials();
        topo.auto_hide("panel.navigator").expect("hide");
        assert!(topo.is_auto_hidden("panel.navigator"));
        assert!(topo.is_docked("panel.navigator"));
        topo.pin("panel.navigator").expect("pin");
        assert!(!topo.is_auto_hidden("panel.navigator"));
    }

    #[test]
    fn a_dragged_height_is_clamped_into_a_usable_range() {
        let mut topo = DockTopology::essentials();
        topo.set_panel_height("panel.layers", 1).expect("docked");
        assert_eq!(
            topo.panel_height("panel.layers"),
            Some(DockTopology::MIN_PANEL_HEIGHT),
            "a panel dragged to nothing has no handle left to drag back out"
        );
        topo.set_panel_height("panel.layers", 999_999)
            .expect("docked");
        assert_eq!(
            topo.panel_height("panel.layers"),
            Some(DockTopology::MAX_PANEL_HEIGHT)
        );
    }

    #[test]
    fn an_untouched_panel_stores_no_height() {
        // Absent means "the shell decides". Writing a default for every panel
        // would freeze a layout decision the shell should still be free to
        // change between versions.
        let topo = DockTopology::essentials();
        assert!(topo.panel_heights.is_empty());
        assert_eq!(topo.panel_height("panel.properties"), None);
    }

    #[test]
    fn a_height_cannot_be_set_for_a_panel_that_is_not_docked() {
        let mut topo = DockTopology::essentials();
        assert!(topo.set_panel_height("panel.nope", 200).is_err());
        assert!(topo.panel_heights.is_empty());
    }

    #[test]
    fn undocking_a_panel_forgets_its_height() {
        let mut topo = DockTopology::essentials();
        topo.set_panel_height("panel.history", 200).expect("docked");
        topo.tear_off("panel.history", 0, 0, 300, 200, "")
            .expect("tear off");
        assert_eq!(
            topo.panel_height("panel.history"),
            None,
            "a height for an undocked panel is dead state validate must tolerate"
        );
        topo.validate().expect("still valid");
    }

    #[test]
    fn a_topology_written_before_heights_existed_still_loads() {
        let mut value = serde_json::to_value(DockTopology::essentials()).expect("serialize");
        value
            .as_object_mut()
            .expect("object")
            .remove("panel_heights")
            .expect("the field must be there to strip");
        let back: DockTopology = serde_json::from_value(value).expect("deserialize");
        assert!(back.panel_heights.is_empty());
        back.validate().expect("valid");
    }
    /// Tearing a panel off and docking it again is a round trip.
    ///
    /// It used to append: a panel torn from the middle of the stack came back
    /// at the bottom, in a group of its own, so the workspace after a tear-off
    /// and a dock was not the workspace before it. The placement now records
    /// where the panel was and whether it shared a group, and a redock with no
    /// explicit drop position puts it back there.
    #[test]
    fn a_torn_off_panel_docks_back_where_it_was() {
        let mut dock = DockTopology::essentials();
        let stack_before = dock.right_stack.clone();
        let groups_before = dock.tabbed_with_previous.clone();
        // Swatches is tabbed with the panel above it in Essentials, which is
        // the case a plain append gets wrong twice over.
        let panel = "panel.swatches";
        assert!(
            groups_before.iter().any(|id| id == panel),
            "the fixture no longer has a tabbed panel to tear off"
        );

        dock.tear_off(panel, 40, 40, 320, 280, "")
            .expect("tear off");
        assert!(!dock.right_stack.iter().any(|id| id == panel));

        dock.redock(panel, None).expect("redock");
        assert_eq!(dock.right_stack, stack_before, "the stack order changed");
        let mut after = dock.tabbed_with_previous.clone();
        let mut before = groups_before;
        after.sort();
        before.sort();
        assert_eq!(after, before, "the panel came back in a different group");
    }

    /// An explicit drop position still wins.
    ///
    /// Dragging a floating panel onto the stack is the user saying where it
    /// belongs now; joining it to whatever happens to sit above that point
    /// would be a group they did not ask for.
    #[test]
    fn a_drop_position_overrides_where_the_panel_came_from() {
        let mut dock = DockTopology::essentials();
        let panel = "panel.swatches";
        dock.tear_off(panel, 40, 40, 320, 280, "")
            .expect("tear off");
        dock.redock(panel, Some(0)).expect("redock");
        assert_eq!(dock.right_stack.first().map(String::as_str), Some(panel));
        assert!(
            !dock.tabbed_with_previous.iter().any(|id| id == panel),
            "a panel dropped at the head of the stack has nothing to be tabbed with"
        );
    }
}
