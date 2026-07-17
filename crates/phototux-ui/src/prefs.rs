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
    /// Last explicitly saved layout snapshot (visibility + dock JSON object).
    #[serde(default)]
    pub last_saved_workspace_json: String,
    /// Persisted brush preset library JSON (schema 4+).
    #[serde(default)]
    pub brush_presets_json: String,
    /// UI density: `dense` | `comfortable`.
    #[serde(default = "default_ui_density")]
    pub ui_density: String,
    /// High-contrast chrome preference.
    #[serde(default)]
    pub high_contrast: bool,
    /// Reduced-motion preference.
    #[serde(default)]
    pub reduced_motion: bool,
    /// When true, next load applies safe-start chrome (essentials layout, no custom keymap restore).
    #[serde(default)]
    pub safe_start_next: bool,
    /// Max undo timeline entries retained (handbook B9 / P7).
    #[serde(default = "default_history_retention")]
    pub history_retention_limit: u32,
    /// Action id → shortcut chord overrides (empty map = defaults only).
    pub keymap: BTreeMap<String, String>,
    pub schema_version: u32,
}

/// Inclusive bounds for [`Preferences::history_retention_limit`].
pub const HISTORY_RETENTION_MIN: u32 = 8;
pub const HISTORY_RETENTION_MAX: u32 = 512;

fn default_history_retention() -> u32 {
    128
}

/// Clamp retention to product bounds.
pub fn clamp_history_retention(value: u32) -> u32 {
    value.clamp(HISTORY_RETENTION_MIN, HISTORY_RETENTION_MAX)
}

fn default_ui_density() -> String {
    "dense".into()
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
            last_saved_workspace_json: String::new(),
            brush_presets_json: phototux_engine::BrushPresetLibrary::with_defaults()
                .to_json()
                .unwrap_or_default(),
            ui_density: default_ui_density(),
            high_contrast: false,
            reduced_motion: false,
            safe_start_next: false,
            history_retention_limit: default_history_retention(),
            keymap: BTreeMap::new(),
            schema_version: 5,
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
            let mut prefs = Self::default();
            if env_requests_safe_start() {
                prefs.apply_safe_start_chrome();
            }
            return prefs;
        };
        let mut prefs: Self = serde_json::from_slice(&bytes).unwrap_or_default();
        prefs.migrate_panel_visibility();
        let force = env_requests_safe_start() || prefs.safe_start_next;
        if force {
            prefs.apply_safe_start_chrome();
            prefs.safe_start_next = false;
            let _ = prefs.save();
        }
        prefs
    }

    /// Essentials workspace + ignore custom keymap / last-tool restore (handbook safe-start).
    pub fn apply_safe_start_chrome(&mut self) {
        self.reset_workspace_essentials();
        self.keymap.clear();
        self.restore_last_tool = false;
        self.last_tool = phototux_engine::tool_id::BRUSH.to_owned();
        self.dock_topology_json.clear();
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
        if self.brush_presets_json.is_empty() {
            self.brush_presets_json = phototux_engine::BrushPresetLibrary::with_defaults()
                .to_json()
                .unwrap_or_default();
        }
        if self.ui_density.is_empty() {
            self.ui_density = default_ui_density();
        }
        self.history_retention_limit = clamp_history_retention(self.history_retention_limit);
        self.schema_version = self.schema_version.max(5);
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
        self.capture_last_saved_workspace(workspace);
    }

    fn capture_last_saved_workspace(&mut self, workspace: &WorkspaceState) {
        let snapshot = serde_json::json!({
            "panel_visibility": workspace.panel_visibility,
            "dock": workspace.dock,
            "active_preset_id": workspace.active_preset_id,
        });
        self.last_saved_workspace_json = snapshot.to_string();
    }

    /// Restore layout from the last captured snapshot, if any.
    pub fn load_last_saved_workspace(&self) -> Option<WorkspaceState> {
        if self.last_saved_workspace_json.is_empty() {
            return None;
        }
        #[derive(Deserialize)]
        struct Snap {
            panel_visibility: BTreeMap<String, bool>,
            dock: DockTopology,
            #[serde(default)]
            active_preset_id: String,
        }
        let snap: Snap = serde_json::from_str(&self.last_saved_workspace_json).ok()?;
        let mut ws = WorkspaceState::from_visibility_map(snap.panel_visibility);
        let _ = ws.set_dock(snap.dock);
        ws.active_preset_id = snap.active_preset_id;
        ws.revision = 0;
        Some(ws)
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

fn env_requests_safe_start() -> bool {
    matches!(
        std::env::var("PHOTOTUX_SAFE_START").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
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
        assert_eq!(p.schema_version, 5);
    }

    #[test]
    fn history_retention_clamped() {
        assert_eq!(clamp_history_retention(1), HISTORY_RETENTION_MIN);
        assert_eq!(clamp_history_retention(9999), HISTORY_RETENTION_MAX);
        assert_eq!(clamp_history_retention(128), 128);
    }

    #[test]
    fn safe_start_clears_keymap_and_restores_essentials() {
        let mut p = Preferences::default();
        p.keymap.insert("action.edit.undo".into(), "Ctrl+Y".into());
        p.restore_last_tool = true;
        p.apply_safe_start_chrome();
        assert!(p.keymap.is_empty());
        assert!(!p.restore_last_tool);
        assert!(p.panel_visibility.contains_key("panel.layers"));
    }
}
