//! Minimal dock topology (handbook 04) — canvas + one right stack.

use serde::{Deserialize, Serialize};

use crate::shell::default_panels;

/// Versioned dock layout for the desktop shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockTopology {
    pub version: u32,
    /// Ordered panel ids in the right dock stack (top → bottom).
    pub right_stack: Vec<String>,
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
        }
    }

    /// Index of `panel_id` in the right stack, if present.
    pub fn stack_index(&self, panel_id: &str) -> Option<usize> {
        self.right_stack.iter().position(|id| id == panel_id)
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
        for id in &self.right_stack {
            if !known.iter().any(|k| k == id) {
                return Err("unknown panel id in right_stack");
            }
            if !seen.insert(id.as_str()) {
                return Err("duplicate panel id in right_stack");
            }
        }
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
}
