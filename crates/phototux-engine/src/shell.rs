//! Shell descriptors for panels and tools (handbook Phase 3 — Qt presents these).

use serde::{Deserialize, Serialize};

/// Dock / panel contribution descriptor (toolkit-neutral).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelDescriptor {
    pub id: String,
    pub title: String,
    /// Default region: `right`, `left`, `bottom`.
    pub default_region: String,
    pub visible_by_default: bool,
}

/// Tool strip entry descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: String,
    pub title: String,
    pub icon_key: String,
    pub group: String,
}

/// Built-in panel set shipped with the desktop shell.
pub fn default_panels() -> Vec<PanelDescriptor> {
    vec![
        PanelDescriptor {
            id: "panel.properties".into(),
            title: "Properties".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.navigator".into(),
            title: "Navigator".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.swatches".into(),
            title: "Swatches".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.layers".into(),
            title: "Layers".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.history".into(),
            title: "History".into(),
            default_region: "right".into(),
            visible_by_default: true,
        },
        PanelDescriptor {
            id: "panel.paths".into(),
            title: "Paths".into(),
            default_region: "right".into(),
            visible_by_default: false,
        },
        PanelDescriptor {
            id: "panel.character".into(),
            title: "Character".into(),
            default_region: "right".into(),
            visible_by_default: false,
        },
    ]
}

/// Built-in tool strip set.
pub fn default_tools() -> Vec<ToolDescriptor> {
    use crate::tool_id;
    vec![
        ToolDescriptor {
            id: tool_id::BRUSH.into(),
            title: "Brush".into(),
            icon_key: "tool.brush".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::ERASER.into(),
            title: "Eraser".into(),
            icon_key: "tool.eraser".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::FILL.into(),
            title: "Paint Bucket".into(),
            icon_key: "tool.fill".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::GRADIENT.into(),
            title: "Gradient".into(),
            icon_key: "tool.gradient".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::EYEDROPPER.into(),
            title: "Eyedropper".into(),
            icon_key: "tool.eyedropper".into(),
            group: "paint".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_RECT.into(),
            title: "Rectangular Marquee".into(),
            icon_key: "tool.select.rect".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_ELLIPSE.into(),
            title: "Elliptical Marquee".into(),
            icon_key: "tool.select.ellipse".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_LASSO.into(),
            title: "Lasso".into(),
            icon_key: "tool.select.lasso".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::SELECT_POLYGON.into(),
            title: "Polygonal Lasso".into(),
            icon_key: "tool.select.polygon".into(),
            group: "select".into(),
        },
        ToolDescriptor {
            id: tool_id::MOVE.into(),
            title: "Move".into(),
            icon_key: "tool.move".into(),
            group: "transform".into(),
        },
        ToolDescriptor {
            id: tool_id::TRANSFORM.into(),
            title: "Free Transform".into(),
            icon_key: "tool.transform".into(),
            group: "transform".into(),
        },
        ToolDescriptor {
            id: tool_id::CROP.into(),
            title: "Crop".into(),
            icon_key: "tool.crop".into(),
            group: "transform".into(),
        },
        ToolDescriptor {
            id: tool_id::TEXT.into(),
            title: "Text".into(),
            icon_key: "tool.text".into(),
            group: "type".into(),
        },
        ToolDescriptor {
            id: tool_id::SHAPE.into(),
            title: "Shape".into(),
            icon_key: "tool.shape".into(),
            group: "vector".into(),
        },
        ToolDescriptor {
            id: tool_id::PAN.into(),
            title: "Hand".into(),
            icon_key: "tool.pan".into(),
            group: "navigate".into(),
        },
        ToolDescriptor {
            id: tool_id::ZOOM.into(),
            title: "Zoom".into(),
            icon_key: "tool.zoom".into(),
            group: "navigate".into(),
        },
    ]
}

/// Essentials workspace panel visibility map (panel id → visible).
pub fn essentials_panel_visibility() -> Vec<(String, bool)> {
    default_panels()
        .into_iter()
        .map(|p| (p.id, p.visible_by_default))
        .collect()
}

/// JSON for QML consumption.
pub fn panels_json() -> String {
    serde_json::to_string(&default_panels()).unwrap_or_else(|_| "[]".into())
}

pub fn tools_json() -> String {
    serde_json::to_string(&default_tools()).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_serialize() {
        let panels = panels_json();
        assert!(panels.contains("panel.layers"));
        let tools = tools_json();
        assert!(tools.contains("tool.brush"));
    }
}
