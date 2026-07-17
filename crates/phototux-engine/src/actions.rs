//! Toolkit-neutral action descriptors (handbook parity P1.1).
//!
//! Presentations resolve `ActionDescriptor::id` → `command_id` and/or `host_op`.
//! Document mutations still enter [`crate::SessionState::invoke`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::command_id;

/// Menu / toolbar / shortcut / context-menu contribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: String,
    pub label: String,
    /// Menu grouping: `file`, `edit`, `select`, `image`, `layer`, `filter`, `view`, `window`, `help`.
    pub menu: String,
    /// When set, host invokes [`crate::SessionState::invoke`] with this id.
    pub command_id: Option<String>,
    /// Host-only operation id (I/O, dialogs, toggles, parameterized creates).
    pub host_op: Option<String>,
    /// Optional argument for command or host_op (profile name, shape kind, modify op, …).
    pub arg: Option<String>,
    pub shortcut: Option<String>,
    pub icon_key: Option<String>,
    /// Enablement tag evaluated by the host.
    pub enablement: String,
    /// Context-menu surfaces: `layer`, `canvas`, `selection`, `mask`. Empty = MenuBar-only.
    #[serde(default)]
    pub contexts: Vec<String>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "row constructor for the default action table"
)]
fn act(
    id: &str,
    label: &str,
    menu: &str,
    enablement: &str,
    command_id: Option<&str>,
    host_op: Option<&str>,
    arg: Option<&str>,
    shortcut: Option<&str>,
    icon_key: Option<&str>,
) -> ActionDescriptor {
    ActionDescriptor {
        id: id.into(),
        label: label.into(),
        menu: menu.into(),
        command_id: command_id.map(str::to_owned),
        host_op: host_op.map(str::to_owned),
        arg: arg.map(str::to_owned),
        shortcut: shortcut.map(str::to_owned),
        icon_key: icon_key.map(str::to_owned),
        enablement: enablement.into(),
        contexts: Vec::new(),
    }
}

fn set_contexts(actions: &mut [ActionDescriptor], id: &str, contexts: &[&str]) {
    if let Some(action) = actions.iter_mut().find(|a| a.id == id) {
        action.contexts = contexts.iter().map(|s| (*s).to_owned()).collect();
    }
}

/// Built-in MenuBar actions shipped with the desktop shell.
pub fn default_actions() -> Vec<ActionDescriptor> {
    let mut actions = vec![
        // File
        act(
            "action.file.new",
            "&New…",
            "file",
            "io_idle",
            None,
            Some("document.new"),
            None,
            Some("Ctrl+N"),
            Some("file-plus"),
        ),
        act(
            "action.file.open",
            "&Open…",
            "file",
            "io_idle",
            None,
            Some("document.open"),
            None,
            Some("Ctrl+O"),
            Some("folder-open"),
        ),
        act(
            "action.file.save",
            "&Save",
            "file",
            "has_document_io_idle",
            None,
            Some("document.save"),
            None,
            Some("Ctrl+S"),
            Some("floppy-disk"),
        ),
        act(
            "action.file.save-as",
            "Save &As…",
            "file",
            "has_document_io_idle",
            None,
            Some("document.save_as"),
            None,
            Some("Ctrl+Shift+S"),
            None,
        ),
        act(
            "action.file.export",
            "&Export…",
            "file",
            "has_document_io_idle",
            None,
            Some("document.export"),
            None,
            Some("Ctrl+Shift+E"),
            Some("export"),
        ),
        act(
            "action.file.close",
            "&Close",
            "file",
            "has_document_io_idle",
            None,
            Some("document.close"),
            None,
            Some("Ctrl+W"),
            Some("x"),
        ),
        act(
            "action.file.quit",
            "&Quit",
            "file",
            "always",
            None,
            Some("app.quit"),
            None,
            Some("Ctrl+Q"),
            None,
        ),
        // Edit
        act(
            "action.edit.undo",
            "&Undo",
            "edit",
            "can_undo",
            Some(command_id::HISTORY_UNDO),
            None,
            None,
            Some("Ctrl+Z"),
            Some("arrow-counter-clockwise"),
        ),
        act(
            "action.edit.redo",
            "&Redo",
            "edit",
            "can_redo",
            Some(command_id::HISTORY_REDO),
            None,
            None,
            Some("Ctrl+Shift+Z"),
            Some("arrow-clockwise"),
        ),
        act(
            "action.edit.preferences",
            "&Preferences…",
            "edit",
            "always",
            Some(command_id::APP_SHOW_PREFERENCES),
            None,
            None,
            Some("Ctrl+,"),
            None,
        ),
        // Select (presented under Edit or Select menu)
        act(
            "action.select.all",
            "Select &All",
            "select",
            "has_document",
            None,
            Some("selection.select_all"),
            None,
            Some("Ctrl+A"),
            None,
        ),
        act(
            "action.select.deselect",
            "Deselect",
            "select",
            "selection_active",
            None,
            Some("selection.deselect"),
            None,
            Some("Ctrl+D"),
            None,
        ),
        act(
            "action.select.invert",
            "&Invert Selection",
            "select",
            "has_document",
            None,
            Some("selection.invert"),
            None,
            Some("Ctrl+Shift+I"),
            None,
        ),
        act(
            "action.select.feather",
            "&Feather…",
            "select",
            "selection_active",
            None,
            Some("selection.modify"),
            Some("feather:4"),
            None,
            None,
        ),
        act(
            "action.select.expand",
            "Expand",
            "select",
            "selection_active",
            None,
            Some("selection.modify"),
            Some("expand:2"),
            None,
            None,
        ),
        act(
            "action.select.contract",
            "Contract",
            "select",
            "selection_active",
            None,
            Some("selection.modify"),
            Some("contract:2"),
            None,
            None,
        ),
        act(
            "action.select.selection-to-mask",
            "Selection to &Mask",
            "select",
            "selection_active",
            Some(command_id::SELECTION_TO_MASK),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.select.mask-to-selection",
            "Mask to Se&lection",
            "select",
            "has_document",
            Some(command_id::MASK_TO_SELECTION),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.edit.copy",
            "&Copy",
            "edit",
            "selection_active",
            None,
            Some("clipboard.copy"),
            None,
            Some("Ctrl+C"),
            None,
        ),
        act(
            "action.edit.paste-layer",
            "Paste as New Layer",
            "edit",
            "has_document",
            None,
            Some("clipboard.paste_layer"),
            None,
            Some("Ctrl+V"),
            None,
        ),
        // Image
        act(
            "action.image.flip-h",
            "Flip &Horizontal",
            "image",
            "has_document_io_idle",
            None,
            Some("raster.flip"),
            Some("h"),
            None,
            None,
        ),
        act(
            "action.image.flip-v",
            "Flip &Vertical",
            "image",
            "has_document_io_idle",
            None,
            Some("raster.flip"),
            Some("v"),
            None,
            None,
        ),
        act(
            "action.image.rotate-90",
            "Rotate 90° &Clockwise",
            "image",
            "has_document_io_idle",
            None,
            Some("document.rotate_90"),
            None,
            None,
            None,
        ),
        act(
            "action.image.assign-srgb",
            "Assign Profile: sRGB",
            "image",
            "has_document",
            Some(command_id::DOCUMENT_ASSIGN_PROFILE),
            None,
            Some("sRGB"),
            None,
            None,
        ),
        act(
            "action.image.assign-p3",
            "Assign Profile: Display-P3",
            "image",
            "has_document",
            Some(command_id::DOCUMENT_ASSIGN_PROFILE),
            None,
            Some("Display-P3"),
            None,
            None,
        ),
        act(
            "action.image.convert-srgb",
            "Convert to sRGB",
            "image",
            "has_document_io_idle",
            Some(command_id::DOCUMENT_CONVERT_PROFILE),
            None,
            Some("sRGB"),
            None,
            None,
        ),
        act(
            "action.image.convert-p3",
            "Convert to Display-P3",
            "image",
            "has_document_io_idle",
            Some(command_id::DOCUMENT_CONVERT_PROFILE),
            None,
            Some("Display-P3"),
            None,
            None,
        ),
        act(
            "action.image.soft-proof-p3",
            "Soft-Proof: Display-P3",
            "image",
            "has_document",
            Some(command_id::DOCUMENT_SET_SOFT_PROOF),
            None,
            Some("Display-P3:relative"),
            None,
            None,
        ),
        act(
            "action.image.soft-proof-off",
            "Soft-Proof: Off",
            "image",
            "has_document",
            Some(command_id::DOCUMENT_SET_SOFT_PROOF),
            None,
            Some(":relative"),
            None,
            None,
        ),
        // Layer
        act(
            "action.layer.new-raster",
            "New &Layer",
            "layer",
            "has_document",
            Some(command_id::LAYER_CREATE),
            None,
            None,
            Some("Ctrl+Shift+N"),
            None,
        ),
        act(
            "action.layer.new-fill",
            "New &Fill Layer",
            "layer",
            "has_document",
            Some(command_id::LAYER_CREATE_FILL),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.delete",
            "&Delete Layer",
            "layer",
            "has_multiple_layers",
            Some(command_id::LAYER_DELETE),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.new-group",
            "New &Group",
            "layer",
            "has_document",
            Some(command_id::LAYER_GROUP),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.ungroup",
            "&Ungroup",
            "layer",
            "has_document",
            Some(command_id::LAYER_UNGROUP),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.bake-text",
            "Bake &Text",
            "layer",
            "has_document_io_idle",
            None,
            Some("text.bake"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.shape-rect",
            "Rectangle",
            "layer",
            "has_document",
            None,
            Some("shape.create"),
            Some("rect"),
            None,
            None,
        ),
        act(
            "action.layer.shape-ellipse",
            "Ellipse",
            "layer",
            "has_document",
            None,
            Some("shape.create"),
            Some("ellipse"),
            None,
            None,
        ),
        act(
            "action.layer.shape-line",
            "Line",
            "layer",
            "has_document",
            None,
            Some("shape.create"),
            Some("line"),
            None,
            None,
        ),
        act(
            "action.layer.rasterize-shape",
            "Rasterize Shape",
            "layer",
            "has_document_io_idle",
            None,
            Some("shape.rasterize"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.shape-union",
            "Boolean Union",
            "layer",
            "has_document_io_idle",
            Some(command_id::SHAPE_BOOLEAN),
            None,
            Some("union"),
            None,
            None,
        ),
        act(
            "action.layer.shape-intersect",
            "Boolean Intersect",
            "layer",
            "has_document_io_idle",
            Some(command_id::SHAPE_BOOLEAN),
            None,
            Some("intersect"),
            None,
            None,
        ),
        act(
            "action.layer.shape-difference",
            "Boolean Difference",
            "layer",
            "has_document_io_idle",
            Some(command_id::SHAPE_BOOLEAN),
            None,
            Some("difference"),
            None,
            None,
        ),
        act(
            "action.layer.shape-exclusion",
            "Boolean Exclusion",
            "layer",
            "has_document_io_idle",
            Some(command_id::SHAPE_BOOLEAN),
            None,
            Some("exclusion"),
            None,
            None,
        ),
        act(
            "action.layer.drop-shadow",
            "Drop &Shadow",
            "layer",
            "has_document",
            Some(command_id::STYLE_ADD_DROP_SHADOW),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.stroke-style",
            "Layer Stroke Style",
            "layer",
            "has_document",
            Some(command_id::STYLE_ADD_STROKE),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.stroke-path",
            "Stroke Path to Layer",
            "layer",
            "has_document_io_idle",
            None,
            Some("path.stroke"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.add-mask",
            "Add &Mask",
            "layer",
            "no_mask",
            None,
            Some("mask.create"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.delete-mask",
            "Delete Mask",
            "layer",
            "has_mask",
            None,
            Some("mask.delete"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.toggle-mask",
            "Toggle Mask Enabled",
            "layer",
            "has_mask",
            None,
            Some("mask.toggle_enabled"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.add-vector-mask",
            "Add Vector Mask",
            "layer",
            "has_document",
            Some(command_id::MASK_CREATE_VECTOR),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.lock-pixels",
            "Lock Pixels",
            "layer",
            "has_document",
            Some(command_id::LAYER_SET_LOCKS),
            None,
            Some("pixels"),
            None,
            None,
        ),
        act(
            "action.layer.lock-position",
            "Lock Position",
            "layer",
            "has_document",
            Some(command_id::LAYER_SET_LOCKS),
            None,
            Some("position"),
            None,
            None,
        ),
        act(
            "action.layer.lock-all",
            "Lock All",
            "layer",
            "has_document",
            Some(command_id::LAYER_SET_LOCKS),
            None,
            Some("all"),
            None,
            None,
        ),
        act(
            "action.layer.toggle-clip",
            "Create Clipping Mask",
            "layer",
            "has_document",
            None,
            Some("layer.toggle_clip"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.adj-brightness",
            "Brightness/Contrast",
            "layer",
            "has_document",
            Some(command_id::FILTER_ADD_ADJUSTMENT),
            None,
            Some("brightness"),
            None,
            None,
        ),
        act(
            "action.layer.adj-levels",
            "Levels",
            "layer",
            "has_document",
            Some(command_id::FILTER_ADD_ADJUSTMENT),
            None,
            Some("levels"),
            None,
            None,
        ),
        // Filter
        act(
            "action.filter.gaussian",
            "Gaussian &Blur",
            "filter",
            "has_document",
            Some(command_id::FILTER_ADD_EFFECT),
            None,
            Some("gaussian"),
            None,
            None,
        ),
        act(
            "action.filter.motion",
            "&Motion Blur",
            "filter",
            "has_document",
            Some(command_id::FILTER_ADD_EFFECT),
            None,
            Some("motion"),
            None,
            None,
        ),
        act(
            "action.filter.emboss",
            "&Emboss",
            "filter",
            "has_document",
            Some(command_id::FILTER_ADD_EFFECT),
            None,
            Some("emboss"),
            None,
            None,
        ),
        act(
            "action.filter.sharpen",
            "&Sharpen",
            "filter",
            "has_document",
            Some(command_id::FILTER_ADD_EFFECT),
            None,
            Some("sharpen"),
            None,
            None,
        ),
        // View
        act(
            "action.view.zoom-fit",
            "Zoom to &Fit",
            "view",
            "has_document",
            Some(command_id::VIEW_ZOOM_TO_FIT),
            None,
            None,
            Some("Ctrl+Shift+J"),
            Some("corners-in"),
        ),
        act(
            "action.view.toggle-guides",
            "Show &Guides",
            "view",
            "always",
            None,
            Some("view.toggle_guides"),
            None,
            None,
            None,
        ),
        act(
            "action.view.toggle-grid",
            "Show G&rid",
            "view",
            "always",
            None,
            Some("view.toggle_grid"),
            None,
            None,
            None,
        ),
        act(
            "action.view.toggle-rulers",
            "Show &Rulers",
            "view",
            "always",
            None,
            Some("view.toggle_rulers"),
            None,
            None,
            None,
        ),
        act(
            "action.view.toggle-snap",
            "Sna&p",
            "view",
            "always",
            None,
            Some("view.toggle_snap"),
            None,
            None,
            None,
        ),
        act(
            "action.view.guide-v",
            "New Vertical Guide",
            "view",
            "has_document",
            None,
            Some("view.guide_v"),
            None,
            None,
            None,
        ),
        act(
            "action.view.guide-h",
            "New Horizontal Guide",
            "view",
            "has_document",
            None,
            Some("view.guide_h"),
            None,
            None,
            None,
        ),
        act(
            "action.view.clear-guides",
            "Clear Guides",
            "view",
            "has_document",
            None,
            Some("view.clear_guides"),
            None,
            None,
            None,
        ),
        // Window
        act(
            "action.window.panel-navigator",
            "Navigator",
            "window",
            "always",
            Some(command_id::WORKSPACE_TOGGLE_PANEL),
            None,
            Some("panel.navigator"),
            None,
            None,
        ),
        act(
            "action.window.panel-swatches",
            "Swatches",
            "window",
            "always",
            Some(command_id::WORKSPACE_TOGGLE_PANEL),
            None,
            Some("panel.swatches"),
            None,
            None,
        ),
        act(
            "action.window.panel-layers",
            "Layers",
            "window",
            "always",
            Some(command_id::WORKSPACE_TOGGLE_PANEL),
            None,
            Some("panel.layers"),
            None,
            None,
        ),
        act(
            "action.window.panel-history",
            "History",
            "window",
            "always",
            Some(command_id::WORKSPACE_TOGGLE_PANEL),
            None,
            Some("panel.history"),
            None,
            None,
        ),
        act(
            "action.window.panel-properties",
            "Properties",
            "window",
            "always",
            Some(command_id::WORKSPACE_TOGGLE_PANEL),
            None,
            Some("panel.properties"),
            None,
            None,
        ),
        act(
            "action.window.panel-paths",
            "Paths",
            "window",
            "always",
            Some(command_id::WORKSPACE_TOGGLE_PANEL),
            None,
            Some("panel.paths"),
            None,
            None,
        ),
        act(
            "action.window.panel-character",
            "Character",
            "window",
            "always",
            Some(command_id::WORKSPACE_TOGGLE_PANEL),
            None,
            Some("panel.character"),
            None,
            None,
        ),
        act(
            "action.window.reset",
            "Reset Workspace",
            "window",
            "always",
            Some(command_id::WORKSPACE_RESET),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.window.preset-essentials",
            "Workspace: Essentials",
            "window",
            "always",
            Some(command_id::WORKSPACE_APPLY_PRESET),
            None,
            Some("workspace.preset.essentials"),
            None,
            None,
        ),
        act(
            "action.window.preset-compact",
            "Workspace: Compact",
            "window",
            "always",
            Some(command_id::WORKSPACE_APPLY_PRESET),
            None,
            Some("workspace.preset.compact"),
            None,
            None,
        ),
        act(
            "action.window.preset-painting",
            "Workspace: Painting",
            "window",
            "always",
            Some(command_id::WORKSPACE_APPLY_PRESET),
            None,
            Some("workspace.preset.painting"),
            None,
            None,
        ),
        act(
            "action.window.preset-factory",
            "Workspace: Factory defaults",
            "window",
            "always",
            Some(command_id::WORKSPACE_APPLY_PRESET),
            None,
            Some("workspace.preset.factory"),
            None,
            None,
        ),
        // Help
        act(
            "action.help.about",
            "&About PhotoTux",
            "help",
            "always",
            None,
            Some("help.about"),
            None,
            None,
            Some("info"),
        ),
        // App chrome
        act(
            "action.app.command-palette",
            "Command &Palette…",
            "edit",
            "always",
            None,
            Some("palette.open"),
            None,
            Some("Ctrl+Shift+P"),
            None,
        ),
    ];
    // Context-menu contributions (handbook P1.4).
    set_contexts(&mut actions, "action.layer.new-raster", &["layer"]);
    set_contexts(&mut actions, "action.layer.delete", &["layer"]);
    set_contexts(&mut actions, "action.layer.add-mask", &["layer", "mask"]);
    set_contexts(&mut actions, "action.layer.delete-mask", &["layer", "mask"]);
    set_contexts(&mut actions, "action.layer.toggle-mask", &["layer", "mask"]);
    set_contexts(&mut actions, "action.select.all", &["canvas"]);
    set_contexts(
        &mut actions,
        "action.select.deselect",
        &["canvas", "selection"],
    );
    set_contexts(&mut actions, "action.edit.paste-layer", &["canvas"]);
    set_contexts(&mut actions, "action.view.zoom-fit", &["canvas"]);
    set_contexts(&mut actions, "action.select.feather", &["selection"]);
    set_contexts(&mut actions, "action.select.expand", &["selection"]);
    set_contexts(&mut actions, "action.select.contract", &["selection"]);
    set_contexts(&mut actions, "action.edit.copy", &["selection"]);
    actions
}

/// Look up a built-in action by id.
pub fn action_by_id(id: &str) -> Option<ActionDescriptor> {
    default_actions().into_iter().find(|a| a.id == id)
}

/// Actions that contribute to a context-menu surface.
pub fn actions_for_context(ctx: &str) -> Vec<ActionDescriptor> {
    default_actions()
        .into_iter()
        .filter(|a| a.contexts.iter().any(|c| c == ctx))
        .collect()
}

/// JSON for QML consumption.
pub fn actions_json() -> String {
    serde_json::to_string(&default_actions()).unwrap_or_else(|_| "[]".into())
}

/// JSON for a single context-menu surface.
pub fn context_actions_json(ctx: &str) -> String {
    serde_json::to_string(&actions_for_context(ctx)).unwrap_or_else(|_| "[]".into())
}

/// Normalize a Qt-style shortcut chord (`ctrl+shift+z` → `Ctrl+Shift+Z`).
pub fn normalize_shortcut(raw: &str) -> String {
    raw.split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            match lower.as_str() {
                "ctrl" | "control" | "cmd" => "Ctrl".to_owned(),
                "shift" => "Shift".to_owned(),
                "alt" | "option" => "Alt".to_owned(),
                "meta" | "super" | "win" => "Meta".to_owned(),
                _ if part.len() == 1 => part.to_ascii_uppercase(),
                _ => {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => {
                            let mut s = first.to_ascii_uppercase().to_string();
                            s.extend(chars);
                            s
                        }
                        None => String::new(),
                    }
                }
            }
        })
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

/// Default chord → action id map from built-in descriptors.
pub fn default_shortcut_map() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for action in default_actions() {
        let Some(shortcut) = action.shortcut.as_deref() else {
            continue;
        };
        let chord = normalize_shortcut(shortcut);
        if chord.is_empty() {
            continue;
        }
        map.entry(chord).or_insert(action.id);
    }
    map
}

/// Default action id → chord map (for menu / palette display).
pub fn default_action_shortcuts() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for action in default_actions() {
        let Some(shortcut) = action.shortcut.as_deref() else {
            continue;
        };
        let chord = normalize_shortcut(shortcut);
        if !chord.is_empty() {
            map.entry(action.id).or_insert(chord);
        }
    }
    map
}

/// Resolve a chord against a shortcut map.
pub fn resolve_shortcut<'a>(map: &'a BTreeMap<String, String>, chord: &str) -> Option<&'a str> {
    let key = normalize_shortcut(chord);
    map.get(&key).map(String::as_str)
}

/// Merge default action→chord map with user overrides (action id → chord).
/// Empty override chord removes the binding for that action.
pub fn effective_action_shortcuts(
    overrides: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut map = default_action_shortcuts();
    for (action_id, chord) in overrides {
        let normalized = normalize_shortcut(chord);
        if normalized.is_empty() {
            map.remove(action_id);
        } else {
            map.insert(action_id.clone(), normalized);
        }
    }
    map
}

/// Invert action→chord into chord→action (last writer wins on duplicate chords).
pub fn chord_map_from_action_shortcuts(
    action_to_chord: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (action_id, chord) in action_to_chord {
        map.insert(chord.clone(), action_id.clone());
    }
    map
}

/// If `chord` is already bound to a different action, return that action id.
pub fn shortcut_conflict(
    action_id: &str,
    chord: &str,
    action_to_chord: &BTreeMap<String, String>,
) -> Option<String> {
    let normalized = normalize_shortcut(chord);
    if normalized.is_empty() {
        return None;
    }
    action_to_chord.iter().find_map(|(id, bound)| {
        (id.as_str() != action_id && bound == &normalized).then(|| id.clone())
    })
}

/// JSON object: chord → action id.
pub fn shortcuts_json() -> String {
    serde_json::to_string(&default_shortcut_map()).unwrap_or_else(|_| "{}".into())
}

/// JSON object: action id → chord.
pub fn action_shortcuts_json() -> String {
    serde_json::to_string(&default_action_shortcuts()).unwrap_or_else(|_| "{}".into())
}

/// Effective maps as JSON pair helpers for the host.
pub fn effective_shortcuts_json(overrides: &BTreeMap<String, String>) -> (String, String) {
    let action_map = effective_action_shortcuts(overrides);
    let chord_map = chord_map_from_action_shortcuts(&action_map);
    let chords = serde_json::to_string(&chord_map).unwrap_or_else(|_| "{}".into());
    let actions = serde_json::to_string(&action_map).unwrap_or_else(|_| "{}".into());
    (chords, actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_id;
    use std::collections::HashSet;

    #[test]
    fn action_ids_unique() {
        let mut seen = HashSet::new();
        for a in default_actions() {
            assert!(seen.insert(a.id.clone()), "duplicate action id {}", a.id);
        }
    }

    #[test]
    fn command_ids_are_registered() {
        for a in default_actions() {
            if let Some(cid) = &a.command_id {
                assert!(
                    command_id::ALL.contains(&cid.as_str()),
                    "unknown command_id {cid} on action {}",
                    a.id
                );
            }
        }
    }

    #[test]
    fn actions_serialize() {
        let json = actions_json();
        assert!(json.contains("action.edit.undo"));
        assert!(json.contains(command_id::HISTORY_UNDO));
    }

    #[test]
    fn every_action_has_handler() {
        for a in default_actions() {
            assert!(
                a.command_id.is_some() || a.host_op.is_some(),
                "action {} has neither command nor host_op",
                a.id
            );
        }
    }

    #[test]
    fn context_tags_reference_known_actions() {
        let known: HashSet<_> = default_actions().into_iter().map(|a| a.id).collect();
        for a in default_actions() {
            for ctx in &a.contexts {
                assert!(
                    matches!(ctx.as_str(), "layer" | "canvas" | "selection" | "mask"),
                    "unknown context {ctx} on {}",
                    a.id
                );
            }
            assert!(known.contains(&a.id));
        }
        assert!(
            actions_for_context("layer")
                .iter()
                .any(|a| a.id == "action.layer.delete")
        );
        assert!(!actions_for_context("canvas").is_empty());
        assert!(!actions_for_context("selection").is_empty());
    }

    #[test]
    fn normalize_shortcut_stable() {
        assert_eq!(normalize_shortcut("ctrl+z"), "Ctrl+Z");
        assert_eq!(normalize_shortcut("Ctrl+Shift+Z"), "Ctrl+Shift+Z");
        assert_eq!(normalize_shortcut("ctrl + shift + z"), "Ctrl+Shift+Z");
    }

    #[test]
    fn default_shortcut_chords_unique() {
        let mut seen = HashSet::new();
        for action in default_actions() {
            let Some(shortcut) = action.shortcut.as_deref() else {
                continue;
            };
            let chord = normalize_shortcut(shortcut);
            assert!(
                seen.insert(chord.clone()),
                "duplicate default chord {chord} (action {})",
                action.id
            );
        }
        let map = default_shortcut_map();
        assert_eq!(resolve_shortcut(&map, "ctrl+z"), Some("action.edit.undo"));
    }

    #[test]
    fn overrides_and_conflicts() {
        let mut overrides = BTreeMap::new();
        overrides.insert("action.edit.redo".into(), "Ctrl+Z".into());
        let effective = effective_action_shortcuts(&overrides);
        assert_eq!(
            effective.get("action.edit.redo").map(String::as_str),
            Some("Ctrl+Z")
        );
        let defaults = effective_action_shortcuts(&BTreeMap::new());
        assert_eq!(
            shortcut_conflict("action.edit.redo", "Ctrl+Z", &defaults),
            Some("action.edit.undo".into())
        );
    }
}
