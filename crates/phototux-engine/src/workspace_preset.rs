//! Built-in workspace presets (handbook 03) — layout only, never document state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dock::DockTopology;
use crate::shell::essentials_panel_visibility;
use crate::workspace::WorkspaceState;

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

pub fn workspace_preset_by_id(id: &str) -> Option<WorkspacePreset> {
    builtin_workspace_presets().into_iter().find(|p| p.id == id)
}

pub fn workspace_presets_json() -> String {
    serde_json::to_string(&builtin_workspace_presets()).unwrap_or_else(|_| "[]".into())
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
}
