//! XDG preferences store (handbook 24 — Phase 3).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use phototux_engine::{DockTopology, WorkspaceState};

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
    /// Legacy bools (schema ≤2); migrated into [`Self::panel_visibility`] on load.
    #[serde(default)]
    pub panel_navigator: bool,
    #[serde(default)]
    pub panel_swatches: bool,
    #[serde(default)]
    pub panel_layers: bool,
    #[serde(default)]
    pub panel_history: bool,
    #[serde(default)]
    pub panel_properties: bool,
    /// Panel descriptor id → visible (schema 3+).
    #[serde(default)]
    pub panel_visibility: BTreeMap<String, bool>,
    /// Serialized [`DockTopology`] JSON (schema 3+).
    #[serde(default)]
    pub dock_topology_json: String,
    /// Action id → shortcut chord overrides (empty map = defaults only).
    pub keymap: BTreeMap<String, String>,
    pub schema_version: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        let ws = WorkspaceState::essentials();
        Self {
            show_guides: true,
            show_grid: false,
            show_rulers: false,
            snap_enabled: true,
            restore_last_tool: false,
            last_tool: phototux_engine::tool_id::BRUSH.to_owned(),
            panel_navigator: ws.is_visible("panel.navigator"),
            panel_swatches: ws.is_visible("panel.swatches"),
            panel_layers: ws.is_visible("panel.layers"),
            panel_history: ws.is_visible("panel.history"),
            panel_properties: ws.is_visible("panel.properties"),
            panel_visibility: ws.panel_visibility.clone(),
            dock_topology_json: ws.dock.to_json().unwrap_or_default(),
            keymap: BTreeMap::new(),
            schema_version: 3,
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
        let mut prefs: Self = serde_json::from_slice(&bytes).unwrap_or_default();
        prefs.migrate_panel_visibility();
        prefs
    }

    /// Fill `panel_visibility` from legacy bools when the map is empty.
    pub fn migrate_panel_visibility(&mut self) {
        if self.panel_visibility.is_empty() {
            self.panel_visibility
                .insert("panel.navigator".into(), self.panel_navigator);
            self.panel_visibility
                .insert("panel.swatches".into(), self.panel_swatches);
            self.panel_visibility
                .insert("panel.layers".into(), self.panel_layers);
            self.panel_visibility
                .insert("panel.history".into(), self.panel_history);
            self.panel_visibility
                .insert("panel.properties".into(), self.panel_properties);
        }
        self.sync_legacy_bools_from_map();
        self.schema_version = self.schema_version.max(3);
    }

    fn sync_legacy_bools_from_map(&mut self) {
        self.panel_navigator = *self
            .panel_visibility
            .get("panel.navigator")
            .unwrap_or(&true);
        self.panel_swatches = *self.panel_visibility.get("panel.swatches").unwrap_or(&true);
        self.panel_layers = *self.panel_visibility.get("panel.layers").unwrap_or(&true);
        self.panel_history = *self.panel_visibility.get("panel.history").unwrap_or(&true);
        self.panel_properties = *self
            .panel_visibility
            .get("panel.properties")
            .unwrap_or(&true);
    }

    pub fn apply_workspace(&mut self, workspace: &WorkspaceState) {
        self.panel_visibility = workspace.panel_visibility.clone();
        self.dock_topology_json = workspace.dock.to_json().unwrap_or_default();
        self.sync_legacy_bools_from_map();
    }

    pub fn load_dock_topology(&self) -> DockTopology {
        if self.dock_topology_json.is_empty() {
            return DockTopology::essentials();
        }
        DockTopology::from_json(&self.dock_topology_json)
            .unwrap_or_else(|_| DockTopology::essentials())
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
        let ws = WorkspaceState::essentials();
        self.apply_workspace(&ws);
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
        assert_eq!(p.panel_visibility, back.panel_visibility);
    }

    #[test]
    fn migrate_from_legacy_bools() {
        let mut p = Preferences {
            panel_visibility: BTreeMap::new(),
            panel_navigator: false,
            panel_layers: true,
            schema_version: 2,
            ..Preferences::default()
        };
        p.panel_visibility.clear();
        p.panel_navigator = false;
        p.migrate_panel_visibility();
        assert_eq!(p.panel_visibility.get("panel.navigator"), Some(&false));
        assert_eq!(p.schema_version, 3);
    }
}
