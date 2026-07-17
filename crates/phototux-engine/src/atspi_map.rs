//! Host AT-SPI role/state mapping from semantic accessibility nodes (handbook §29 / DR-016).
//!
//! PhotoTux owns the semantic tree; the Linux host adapter maps roles to AT-SPI names.
//! This module is the portable mapping contract — not a D-Bus AT-SPI server.

use serde::{Deserialize, Serialize};

/// Semantic role used in `accessibilityTreeJson`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRole {
    Toolbar,
    Image,
    Panel,
    Button,
    Menu,
    Dialog,
    Status,
    Unknown,
}

impl SemanticRole {
    pub fn parse(s: &str) -> Self {
        match s {
            "toolbar" => Self::Toolbar,
            "image" | "canvas" => Self::Image,
            "panel" => Self::Panel,
            "button" => Self::Button,
            "menu" => Self::Menu,
            "dialog" => Self::Dialog,
            "status" | "status_bar" => Self::Status,
            _ => Self::Unknown,
        }
    }

    /// AT-SPI role name (ATSPI_ROLE_* string form used by host adapters).
    pub fn atspi_role(self) -> &'static str {
        match self {
            Self::Toolbar => "tool_bar",
            Self::Image => "image",
            Self::Panel => "panel",
            Self::Button => "push_button",
            Self::Menu => "menu",
            Self::Dialog => "dialog",
            Self::Status => "status_bar",
            Self::Unknown => "unknown",
        }
    }
}

/// One projected node for the host AT-SPI adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtspiProjectionNode {
    pub id: String,
    pub name: String,
    pub semantic_role: String,
    pub atspi_role: String,
    pub states: Vec<String>,
}

/// Map a semantic tree JSON array into AT-SPI projection nodes.
///
/// # Errors
/// Returns an error when JSON is not an array of objects.
pub fn project_semantic_tree(tree_json: &str) -> Result<Vec<AtspiProjectionNode>, String> {
    let value: serde_json::Value =
        serde_json::from_str(tree_json).map_err(|e| format!("a11y tree JSON: {e}"))?;
    let arr = value
        .as_array()
        .ok_or_else(|| "a11y tree must be a JSON array".to_owned())?;
    let mut out = Vec::with_capacity(arr.len());
    for node in arr {
        let id = node
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let name = node
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let role_str = node
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let semantic = SemanticRole::parse(role_str);
        let mut states = Vec::new();
        if let Some(state) = node.get("state").and_then(|v| v.as_object()) {
            if state.get("enabled").and_then(|v| v.as_bool()) == Some(true) {
                states.push("enabled".into());
            }
            if state.get("visible").and_then(|v| v.as_bool()) == Some(true) {
                states.push("visible".into());
            }
            if state.get("busy").and_then(|v| v.as_bool()) == Some(true) {
                states.push("busy".into());
            }
            if state.get("docked").and_then(|v| v.as_bool()) == Some(true) {
                states.push("docked".into());
            }
        }
        out.push(AtspiProjectionNode {
            id,
            name,
            semantic_role: role_str.to_owned(),
            atspi_role: semantic.atspi_role().to_owned(),
            states,
        });
    }
    Ok(out)
}

/// Serialize projection for QML / host bridge.
pub fn project_semantic_tree_json(tree_json: &str) -> String {
    match project_semantic_tree(tree_json) {
        Ok(nodes) => serde_json::to_string(&nodes).unwrap_or_else(|_| "[]".into()),
        Err(_) => "[]".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_toolbar_and_canvas_roles() {
        let tree = r#"[
          {"id":"chrome.toolbar","role":"toolbar","name":"Tools","state":{"enabled":true}},
          {"id":"chrome.canvas","role":"image","name":"Canvas 100×100","state":{"busy":false}}
        ]"#;
        let nodes = project_semantic_tree(tree).expect("project");
        assert_eq!(nodes[0].atspi_role, "tool_bar");
        assert!(nodes[0].states.contains(&"enabled".into()));
        assert_eq!(nodes[1].atspi_role, "image");
    }

    #[test]
    fn panel_maps_to_atspi_panel() {
        assert_eq!(SemanticRole::Panel.atspi_role(), "panel");
    }

    /// Evidence pack fixture: semantic tree → AT-SPI projection JSON (DR-028 A8).
    #[test]
    fn evidence_pack_projects_chrome_fixture() {
        let tree = r#"[
          {"id":"chrome.toolbar","role":"toolbar","name":"Tools","state":{"enabled":true}},
          {"id":"chrome.canvas","role":"image","name":"Canvas 1920×1080","state":{"busy":false,"editTarget":"layer"}},
          {"id":"panel.layers","role":"panel","name":"Layers","state":{"visible":true,"docked":true}}
        ]"#;
        let json = project_semantic_tree_json(tree);
        let nodes: Vec<AtspiProjectionNode> = serde_json::from_str(&json).expect("projection json");
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].atspi_role, "tool_bar");
        assert_eq!(nodes[1].atspi_role, "image");
        assert_eq!(nodes[2].atspi_role, "panel");
        assert!(nodes[2].states.iter().any(|s| s == "docked"));
    }
}
