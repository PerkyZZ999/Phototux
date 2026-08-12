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
}

impl Default for DockTopology {
    fn default() -> Self {
        Self::essentials()
    }
}

impl DockTopology {
    pub const CURRENT_VERSION: u32 = 1;

    /// Essentials order matching the historical QML ColumnLayout.
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
        }
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
        self.right_stack.remove(idx);
        self.floating.push(FloatingPanelPlacement {
            id: panel_id.to_owned(),
            x,
            y,
            width,
            height,
            display_hint: display_hint.into(),
        });
        self.validate()
    }

    /// Return a floating panel to the right stack (append, or `at` when in range).
    ///
    /// # Errors
    /// Returns a static reason when the panel is not floating.
    pub fn redock(&mut self, panel_id: &str, at: Option<usize>) -> Result<(), &'static str> {
        let pos = self
            .floating
            .iter()
            .position(|f| f.id == panel_id)
            .ok_or("panel not floating")?;
        self.floating.remove(pos);
        let insert_at = at
            .unwrap_or(self.right_stack.len())
            .min(self.right_stack.len());
        self.right_stack.insert(insert_at, panel_id.to_owned());
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
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate().map_err(str::to_owned)?;
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let topo: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
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
    fn cannot_tear_off_last() {
        let mut topo = DockTopology {
            version: 1,
            right_stack: vec!["panel.layers".into()],
            floating: Vec::new(),
            auto_hidden: Vec::new(),
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
}
