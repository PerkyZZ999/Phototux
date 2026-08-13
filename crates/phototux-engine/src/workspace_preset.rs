//! Built-in + user workspace presets (handbook 03) — layout only, never document state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dock::DockTopology;
use crate::shell::essentials_panel_visibility;
use crate::workspace::WorkspaceState;

/// Id prefix for user-authored presets persisted in preferences.
pub const USER_WORKSPACE_PRESET_PREFIX: &str = "workspace.preset.user.";

/// Soft cap so prefs stay small (titles + dock JSON).
pub const MAX_USER_WORKSPACE_PRESETS: usize = 24;

/// Named layout snapshot (visibility + dock topology).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePreset {
    pub id: String,
    pub title: String,
    pub panel_visibility: BTreeMap<String, bool>,
    pub dock: DockTopology,
}

impl WorkspacePreset {
    fn from_parts(
        id: &str,
        title: &str,
        visibility: BTreeMap<String, bool>,
        dock: DockTopology,
    ) -> Self {
        Self {
            id: id.to_owned(),
            title: title.to_owned(),
            panel_visibility: visibility,
            dock,
        }
    }

    /// Snapshot the current workspace layout under a user preset id + title.
    pub fn from_workspace(
        id: impl Into<String>,
        title: impl Into<String>,
        ws: &WorkspaceState,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            panel_visibility: ws.panel_visibility.clone(),
            dock: ws.dock.clone(),
        }
    }

    pub fn is_user_preset(&self) -> bool {
        self.id.starts_with(USER_WORKSPACE_PRESET_PREFIX)
    }
}

/// Built-in presets shipped with the desktop shell.
pub fn builtin_workspace_presets() -> Vec<WorkspacePreset> {
    let essentials = WorkspaceState::essentials();
    let mut compact_vis = essentials.panel_visibility.clone();
    compact_vis.insert("panel.navigator".into(), false);
    compact_vis.insert("panel.history".into(), false);
    compact_vis.insert("panel.swatches".into(), false);
    let mut compact_dock = DockTopology::essentials();
    compact_dock.right_stack = vec!["panel.properties".into(), "panel.layers".into()];
    compact_dock.floating.clear();
    compact_dock.auto_hidden.clear();
    // Compact means one group: both panels share the dock's full height and
    // the user tabs between them.
    compact_dock.tabbed_with_previous = vec!["panel.layers".into()];
    compact_dock.normalize_groups();

    let mut painting_vis = essentials.panel_visibility.clone();
    painting_vis.insert("panel.navigator".into(), false);
    painting_vis.insert("panel.history".into(), false);
    let mut painting_dock = DockTopology::essentials();
    painting_dock.right_stack = vec![
        "panel.properties".into(),
        "panel.swatches".into(),
        "panel.layers".into(),
    ];
    painting_dock.floating.clear();
    painting_dock.auto_hidden.clear();
    // Three panels fit stacked without starving each other, so painting keeps
    // them separate and visible at once.
    painting_dock.tabbed_with_previous.clear();
    painting_dock.normalize_groups();

    vec![
        WorkspacePreset::from_parts(
            "workspace.preset.essentials",
            "Essentials",
            essentials.panel_visibility,
            essentials.dock,
        ),
        WorkspacePreset::from_parts(
            "workspace.preset.compact",
            "Compact",
            compact_vis,
            compact_dock,
        ),
        WorkspacePreset::from_parts(
            "workspace.preset.painting",
            "Painting",
            painting_vis,
            painting_dock,
        ),
        WorkspacePreset::from_parts(
            "workspace.preset.factory",
            "Factory defaults",
            essentials_panel_visibility().into_iter().collect(),
            DockTopology::essentials(),
        ),
    ]
}

pub fn is_user_workspace_preset_id(id: &str) -> bool {
    id.starts_with(USER_WORKSPACE_PRESET_PREFIX)
}

/// Normalize a display title into a stable slug fragment for user preset ids.
pub fn slugify_workspace_preset_title(title: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "layout".into()
    } else {
        // Cap slug length so ids stay readable in prefs JSON.
        out.chars().take(48).collect()
    }
}

pub fn parse_user_workspace_presets(json: &str) -> Vec<WorkspacePreset> {
    if json.trim().is_empty() {
        return Vec::new();
    }
    let Ok(mut list) = serde_json::from_str::<Vec<WorkspacePreset>>(json) else {
        return Vec::new();
    };
    list.retain(|p| p.is_user_preset() && !p.title.trim().is_empty() && p.dock.validate().is_ok());
    list.truncate(MAX_USER_WORKSPACE_PRESETS);
    list
}

pub fn user_workspace_presets_json(presets: &[WorkspacePreset]) -> String {
    serde_json::to_string(presets).unwrap_or_else(|_| "[]".into())
}

/// Built-ins first, then user presets (dedupe by id — builtins win).
pub fn merged_workspace_presets(user_json: &str) -> Vec<WorkspacePreset> {
    let mut out = builtin_workspace_presets();
    let builtin_ids: BTreeMap<String, ()> = out.iter().map(|p| (p.id.clone(), ())).collect();
    for preset in parse_user_workspace_presets(user_json) {
        if builtin_ids.contains_key(&preset.id) {
            continue;
        }
        out.push(preset);
    }
    out
}

pub fn workspace_preset_by_id(id: &str) -> Option<WorkspacePreset> {
    builtin_workspace_presets().into_iter().find(|p| p.id == id)
}

/// Resolve a preset from builtins or a user-presets JSON blob.
pub fn resolve_workspace_preset(id: &str, user_json: &str) -> Option<WorkspacePreset> {
    if let Some(builtin) = workspace_preset_by_id(id) {
        return Some(builtin);
    }
    parse_user_workspace_presets(user_json)
        .into_iter()
        .find(|p| p.id == id)
}

pub fn workspace_presets_json() -> String {
    serde_json::to_string(&builtin_workspace_presets()).unwrap_or_else(|_| "[]".into())
}

pub fn merged_workspace_presets_json(user_json: &str) -> String {
    serde_json::to_string(&merged_workspace_presets(user_json)).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_validate() {
        for preset in builtin_workspace_presets() {
            assert!(preset.dock.validate().is_ok(), "{}", preset.id);
            assert!(!preset.panel_visibility.is_empty());
        }
    }

    #[test]
    fn apply_preset_leaves_document_clean() {
        use crate::{SessionState, SizePreset};
        let mut session = SessionState::default();
        session.apply_preset(SizePreset::P720);
        let doc_gen = session.document_generation();
        let mut ws = WorkspaceState::essentials();
        let painting = workspace_preset_by_id("workspace.preset.painting").expect("preset");
        ws.apply_preset(&painting);
        assert!(!ws.is_visible("panel.history"));
        assert_eq!(session.document_generation(), doc_gen);
    }

    #[test]
    fn user_preset_roundtrip_and_merge() {
        let mut ws = WorkspaceState::essentials();
        assert!(ws.set_visible("panel.navigator", false));
        let preset = WorkspacePreset::from_workspace(
            format!("{USER_WORKSPACE_PRESET_PREFIX}my-desk"),
            "My Desk",
            &ws,
        );
        let json = user_workspace_presets_json(std::slice::from_ref(&preset));
        let merged = merged_workspace_presets(&json);
        assert!(
            merged
                .iter()
                .any(|p| p.id == preset.id && p.title == "My Desk")
        );
        assert!(
            !merged
                .iter()
                .find(|p| p.id == preset.id)
                .expect("user")
                .panel_visibility
                .get("panel.navigator")
                .copied()
                .unwrap_or(true)
        );
    }

    #[test]
    fn slugify_title() {
        assert_eq!(slugify_workspace_preset_title("My Desk!"), "my-desk");
        assert_eq!(slugify_workspace_preset_title("   "), "layout");
    }
}
