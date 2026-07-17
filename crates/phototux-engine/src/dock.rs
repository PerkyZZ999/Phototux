//! Minimal dock topology (handbook 04) — canvas + one right stack. No float/DnD yet.

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
}
