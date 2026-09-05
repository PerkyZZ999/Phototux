//! Preferences, the workspace, and what persists between sessions.
//!
//! Two stores that look alike and are not. Preferences are the user's settings
//! and are written on change; the workspace is the dock arrangement, which the
//! shell also edits by dragging, so it is persisted from the operations that
//! change it rather than from a single save point.
//!
//! Most of this runs during construction, before the QObject proxy exists —
//! `apply_loaded_preferences` is the first thing
//! `settle_before_the_proxy_attaches` calls. Nothing reachable from here may
//! emit; `nothing_reachable_from_construction_emits` walks the closure and
//! fails on a `publish!` anywhere in it, across module boundaries included.

use phototux_engine::{CommandArgs, NoticeLevel, WorkspaceState, command_id};

use crate::AppSession;
use crate::prefs::Preferences;

impl AppSession {
    // `pub(crate)`, as in the sibling `session_*` modules.

    pub(crate) fn refresh_pref_effective_json(&mut self) {
        // Document soft-proof profile beats user default (none). Guides: session mirrors user prefs.
        let (soft_proof, soft_src) = phototux_engine::resolve_layered(
            String::new(),
            Some(String::new()),
            None,
            self.engine
                .graph
                .as_ref()
                .map(|g| g.color.soft_proof_profile.clone())
                .filter(|s| !s.is_empty()),
        );
        let (density, dens_src) = phototux_engine::resolve_layered(
            "dense".to_owned(),
            Some(self.prefs.ui_density.clone()),
            None,
            None,
        );
        let (guides, guides_src) =
            phototux_engine::resolve_layered(true, Some(self.prefs.show_guides), None, None);
        let map = serde_json::json!({
            "soft_proof_profile": { "value": soft_proof, "source": soft_src.as_str() },
            "ui_density": { "value": density, "source": dens_src.as_str() },
            "show_guides": { "value": guides, "source": guides_src.as_str() },
        });
        self.pref_effective_json = map.to_string();
    }

    pub(crate) fn apply_loaded_preferences(&mut self) {
        self.prefs = Preferences::load();
        self.workspace = WorkspaceState::from_visibility_map(self.prefs.panel_visibility.clone());
        let _ = self.workspace.set_dock(self.prefs.load_dock_topology());
        self.sync_pref_fields_from_store();
        self.refresh_workspace_presets_json();
        self.refresh_shortcut_maps();
        self.engine.guides.show_guides = self.prefs.show_guides;
        self.engine.guides.show_grid = self.prefs.show_grid;
        self.engine.guides.show_rulers = self.prefs.show_rulers;
        self.engine.guides.snap = self.prefs.snap_enabled;
        if self.prefs.restore_last_tool && !self.prefs.last_tool.is_empty() {
            let _ = self.engine.invoke(
                command_id::VIEW_SET_TOOL,
                CommandArgs::Tool {
                    tool: self.prefs.last_tool.clone(),
                },
            );
        }
    }

    pub(crate) fn refresh_workspace_presets_json(&mut self) {
        self.workspace_presets_json =
            phototux_engine::merged_workspace_presets_json(&self.prefs.user_workspace_presets_json);
    }

    pub(crate) fn refresh_shortcut_maps(&mut self) {
        let (chords, actions) = phototux_engine::effective_shortcuts_json(&self.prefs.keymap);
        self.shortcuts_json = chords;
        self.action_shortcuts_json = actions;
    }

    /// Publish descriptor defaults merged with user overrides, so QML reads a
    /// resolved boolean per group instead of re-deriving the default.
    ///
    /// Overrides stay sparse in the store: a group the user never touched keeps
    /// following its descriptor default when that default changes.
    pub(crate) fn refresh_disclosure_open_json(&mut self) {
        let resolved: std::collections::BTreeMap<String, bool> =
            phototux_engine::default_disclosure_groups()
                .into_iter()
                .map(|group| {
                    let open = self
                        .prefs
                        .disclosure_is_open(&group.id, group.open_by_default);
                    (group.id, open)
                })
                .collect();
        self.disclosure_open_json =
            serde_json::to_string(&resolved).unwrap_or_else(|_| "{}".to_owned());
    }

    /// Project the dock's tab groups for QML.
    pub(crate) fn build_dock_groups_json(&self) -> String {
        let groups: Vec<serde_json::Value> = self
            .workspace
            .dock
            .right_groups()
            .into_iter()
            .map(|tabs| {
                let active = tabs
                    .first()
                    .and_then(|id| self.workspace.effective_active_tab(id))
                    .unwrap_or_default();
                serde_json::json!({ "tabs": tabs, "active": active })
            })
            .collect();
        serde_json::to_string(&groups).unwrap_or_else(|_| "[]".into())
    }

    /// Set every registered inspector disclosure group to `open` and persist once.
    pub(crate) fn set_all_disclosure_groups(&mut self, open: bool) {
        for group in phototux_engine::default_disclosure_groups() {
            self.prefs.set_disclosure_open(&group.id, open);
        }
        self.refresh_disclosure_open_json();
        self.persist_prefs();
        self.disclosure_open_json_changed();
    }

    pub(crate) fn persist_workspace_visibility(&mut self) {
        self.prefs.apply_workspace(&self.workspace);
        self.sync_panel_visibility_from_workspace();
        self.persist_prefs();
        self.panel_visibility_json_changed();
        self.dock_topology_json_changed();
        self.dock_groups_json_changed();
        self.workspace_focus_json_changed();
        self.active_workspace_preset_id_changed();
    }

    pub(crate) fn persist_prefs(&mut self) {
        if let Err(error) = self.prefs.save() {
            self.notify(
                NoticeLevel::Error,
                format!("Preferences save failed: {error}"),
            );
        }
    }
}
