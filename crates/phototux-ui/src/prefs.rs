//! XDG preferences store (handbook 24 — Phase 3).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// User preferences persisted under `$XDG_CONFIG_HOME/phototux/preferences.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub show_guides: bool,
    pub show_grid: bool,
    pub show_rulers: bool,
    pub snap_enabled: bool,
    pub restore_last_tool: bool,
    pub last_tool: String,
    pub panel_navigator: bool,
    pub panel_swatches: bool,
    pub panel_layers: bool,
    pub panel_history: bool,
    pub panel_properties: bool,
    pub schema_version: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_guides: true,
            show_grid: false,
            show_rulers: false,
            snap_enabled: true,
            restore_last_tool: false,
            last_tool: phototux_engine::tool_id::BRUSH.to_owned(),
            panel_navigator: true,
            panel_swatches: true,
            panel_layers: true,
            panel_history: true,
            panel_properties: true,
            schema_version: 1,
        }
    }
}

impl Preferences {
    pub fn config_path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"));
        base.join("phototux").join("preferences.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let Ok(bytes) = fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    /// Write preferences atomically when possible.
    ///
    /// # Errors
    /// Returns an I/O or serialization error string.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, &json).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn reset_workspace_essentials(&mut self) {
        let defaults = Self::default();
        self.panel_navigator = defaults.panel_navigator;
        self.panel_swatches = defaults.panel_swatches;
        self.panel_layers = defaults.panel_layers;
        self.panel_history = defaults.panel_history;
        self.panel_properties = defaults.panel_properties;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip_json() {
        let p = Preferences::default();
        let s = serde_json::to_string(&p).expect("ser");
        let back: Preferences = serde_json::from_str(&s).expect("de");
        assert_eq!(p, back);
    }
}
