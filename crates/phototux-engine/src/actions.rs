//! Toolkit-neutral action descriptors (handbook parity P1.1).
//!
//! Presentations resolve `ActionDescriptor::id` → `command_id` and/or `host_op`.
//! Document mutations still enter [`crate::SessionState::invoke`].

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::{command_id, tool_id};

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

/// Menu path of the adjustment-layer submenu.
///
/// A `parent.child` menu name is a submenu of `parent`. The Layer menu already
/// carried thirty entries and overflowed the window on a 1080p display, so the
/// seven adjustment kinds go one level down rather than off the bottom edge —
/// which is also where every editor of this kind files them.
pub const ADJUSTMENT_SUBMENU: &str = "layer.adjustment";

/// Action id for the "add adjustment layer" entry of one adjustment kind.
fn adjustment_action_id(kind: &str) -> String {
    format!("action.layer.adj-{kind}")
}

/// Action id for the "add filter effect" entry of one filter kind.
fn filter_action_id(kind: &str) -> String {
    format!("action.filter.{kind}")
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
            "action.edit.copy-selection-mask",
            "Copy &Selection Mask",
            "edit",
            "selection_active",
            None,
            Some("clipboard.copy_selection_mask"),
            None,
            None,
            Some("selection-background"),
        ),
        act(
            "action.edit.copy-layer-mask",
            "Copy Layer &Mask",
            "edit",
            "has_document",
            None,
            Some("clipboard.copy_layer_mask"),
            None,
            None,
            Some("circle-half"),
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
        act(
            "action.edit.paste-selection",
            "Paste as &Selection",
            "edit",
            "has_document",
            None,
            Some("clipboard.paste_selection"),
            None,
            None,
            Some("selection-foreground"),
        ),
        act(
            "action.edit.paste-mask",
            "Paste as Layer M&ask",
            "edit",
            "has_document",
            None,
            Some("clipboard.paste_mask"),
            None,
            None,
            Some("circle-half-tilt"),
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
        act(
            "action.image.embed-icc",
            "Embed &ICC Profile…",
            "image",
            "has_document_io_idle",
            None,
            Some("document.embed_icc"),
            None,
            None,
            None,
        ),
        act(
            "action.image.clear-icc",
            "Clear Embedded ICC",
            "image",
            "has_document",
            Some(command_id::DOCUMENT_SET_ICC),
            None,
            Some("clear"),
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
            "layer.shape",
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
            "layer.shape",
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
            "layer.shape",
            "has_document",
            None,
            Some("shape.create"),
            Some("line"),
            None,
            None,
        ),
        act(
            "action.layer.shape-polygon",
            "Polygon",
            "layer.shape",
            "has_document",
            None,
            Some("shape.create"),
            Some("polygon"),
            None,
            None,
        ),
        act(
            "action.layer.shape-gradient",
            "Gradient Fill",
            "layer.shape",
            "has_document",
            None,
            Some("shape.create"),
            Some("gradient"),
            None,
            None,
        ),
        act(
            "action.layer.shape-live",
            "Live Vector Shape",
            "layer.shape",
            "has_document",
            None,
            Some("shape.create"),
            Some("live"),
            None,
            None,
        ),
        act(
            "action.layer.rasterize-shape",
            "Rasterize Shape",
            "layer.shape",
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
            "layer.boolean",
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
            "layer.boolean",
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
            "layer.boolean",
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
            "layer.boolean",
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
            "layer.style",
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
            "layer.style",
            "has_document",
            Some(command_id::STYLE_ADD_STROKE),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.outer-glow",
            "Outer &Glow",
            "layer.style",
            "has_document",
            Some(command_id::STYLE_ADD_OUTER_GLOW),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.color-overlay",
            "Color &Overlay",
            "layer.style",
            "has_document",
            Some(command_id::STYLE_ADD_COLOR_OVERLAY),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.stroke-path",
            "Stroke Path to Layer",
            "layer.shape",
            "has_document_io_idle",
            None,
            Some("path.stroke"),
            None,
            None,
            None,
        ),
        act(
            "action.layer.apply-mask",
            "&Apply Mask",
            "layer.mask",
            "has_document",
            Some(command_id::MASK_APPLY),
            None,
            None,
            None,
            None,
        ),
        act(
            "action.layer.add-mask",
            "Add &Mask",
            "layer.mask",
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
            "layer.mask",
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
            "layer.mask",
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
            "layer.mask",
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
            "layer.lock",
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
            "layer.lock",
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
            "layer.lock",
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
        // Filter
        act(
            "action.filter.gallery",
            "Filter &Gallery…",
            "filter",
            "has_document",
            Some(command_id::APP_SHOW_FILTER_GALLERY),
            None,
            None,
            None,
            None,
        ),
        // Tools — letter keys follow the conventional raster-editor
        // assignments so muscle memory transfers. `tools` is a search-only
        // menu: these belong on the tool shelf and in action search, not in
        // the menu bar, which is where every editor of this kind puts them.
        act(
            "action.tool.move",
            "&Move",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::MOVE),
            Some("V"),
            Some("arrows-out-cardinal"),
        ),
        act(
            "action.tool.select-rect",
            "&Rectangular Marquee",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::SELECT_RECT),
            Some("M"),
            Some("selection"),
        ),
        act(
            "action.tool.select-ellipse",
            "&Elliptical Marquee",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::SELECT_ELLIPSE),
            Some("Shift+M"),
            Some("circle-dashed"),
        ),
        act(
            "action.tool.select-lasso",
            "&Lasso",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::SELECT_LASSO),
            Some("L"),
            Some("lasso"),
        ),
        act(
            "action.tool.select-polygon",
            "&Polygonal Lasso",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::SELECT_POLYGON),
            Some("Shift+L"),
            Some("polygon"),
        ),
        act(
            "action.tool.crop",
            "&Crop",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::CROP),
            Some("C"),
            Some("crop"),
        ),
        act(
            "action.tool.eyedropper",
            "Eye&dropper",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::EYEDROPPER),
            Some("I"),
            Some("eyedropper"),
        ),
        act(
            "action.tool.brush",
            "&Brush",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::BRUSH),
            Some("B"),
            Some("paint-brush"),
        ),
        act(
            "action.tool.eraser",
            "&Eraser",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::ERASER),
            Some("E"),
            Some("eraser"),
        ),
        act(
            "action.tool.gradient",
            "&Gradient",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::GRADIENT),
            Some("G"),
            Some("gradient"),
        ),
        act(
            "action.tool.fill",
            "Paint Buc&ket",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::FILL),
            Some("Shift+G"),
            Some("paint-bucket"),
        ),
        act(
            "action.tool.text",
            "&Text",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::TEXT),
            Some("T"),
            Some("text-t"),
        ),
        act(
            "action.tool.path-edit",
            "&Path Edit",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::PATH_EDIT),
            Some("P"),
            Some("pen-nib"),
        ),
        act(
            "action.tool.shape",
            "Shape",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::SHAPE),
            Some("U"),
            Some("shapes"),
        ),
        act(
            "action.tool.pan",
            "&Hand",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::PAN),
            Some("H"),
            Some("hand"),
        ),
        act(
            "action.tool.zoom",
            "&Zoom",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::ZOOM),
            Some("Z"),
            Some("magnifying-glass"),
        ),
        act(
            "action.tool.transform",
            "&Free Transform",
            "tools",
            "has_document",
            None,
            Some("tool.activate"),
            Some(tool_id::TRANSFORM),
            Some("Ctrl+T"),
            Some("arrows-out"),
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
        // Panel-local view actions (handbook 05): workspace scope, no document
        // mutation, so they stay available with no document open.
        act(
            "action.view.expand-all-groups",
            "E&xpand All Property Groups",
            "view",
            "always",
            None,
            Some("inspector.expand_all"),
            None,
            None,
            Some("arrows-out-line-vertical"),
        ),
        act(
            "action.view.collapse-all-groups",
            "&Collapse All Property Groups",
            "view",
            "always",
            None,
            Some("inspector.collapse_all"),
            None,
            None,
            Some("arrows-in-line-vertical"),
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
        act(
            "action.app.simulate-device-lost",
            "Simulate Device Lost (debug)",
            "view",
            "has_document",
            None,
            Some("app.simulate_device_lost"),
            None,
            None,
            None,
        ),
        act(
            "action.app.recover-gpu",
            "&Recover graphics…",
            "view",
            "document",
            None,
            Some("app.recover_gpu"),
            None,
            None,
            Some("arrows-clockwise"),
        ),
    ];
    // One Layer-menu entry per adjustment kind, generated from the vocabulary.
    //
    // These were three hand-written entries against seven kinds, so
    // Hue/Saturation, Invert, Threshold and Posterize had no way into the
    // document from the chrome at all — the same four the composite shader
    // was ignoring. Two independent lists, one silence.
    // One Filter-menu entry per filter kind, generated the same way and for
    // the same reason: five hand-written entries against a thirteen-kind
    // vocabulary left Box Blur, Invert and Offset with no way in.
    actions.extend(crate::FilterParams::ALL_KINDS.iter().map(|params| {
        act(
            &filter_action_id(params.kind_key()),
            params.label(),
            "filter",
            "has_document",
            Some(command_id::FILTER_ADD_EFFECT),
            None,
            Some(params.kind_key()),
            None,
            None,
        )
    }));
    actions.extend(crate::AdjustmentParams::ALL_KINDS.iter().map(|params| {
        act(
            &adjustment_action_id(params.kind_key()),
            params.label(),
            ADJUSTMENT_SUBMENU,
            "has_document",
            Some(command_id::FILTER_ADD_ADJUSTMENT),
            None,
            Some(params.kind_key()),
            None,
            None,
        )
    }));
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
    set_contexts(
        &mut actions,
        "action.edit.copy-selection-mask",
        &["selection"],
    );
    set_contexts(
        &mut actions,
        "action.edit.copy-layer-mask",
        &["layer", "mask"],
    );
    set_contexts(
        &mut actions,
        "action.edit.paste-selection",
        &["canvas", "selection"],
    );
    set_contexts(&mut actions, "action.edit.paste-mask", &["layer", "mask"]);
    actions
}

/// The built-in action table, constructed once.
///
/// [`default_actions`] allocates roughly 800 strings across 101 descriptors.
/// Presentations resolve enablement per action per binding evaluation, so a
/// lookup that rebuilt the table turned a single "can I undo?" question into a
/// full table construction — and a shell with ~100 bound menu items paid for it
/// ~100 times whenever an enablement input changed.
struct ActionTable {
    actions: Vec<ActionDescriptor>,
    by_id: HashMap<String, usize>,
}

static ACTION_TABLE: LazyLock<ActionTable> = LazyLock::new(|| {
    let actions = default_actions();
    let by_id = actions
        .iter()
        .enumerate()
        .map(|(index, action)| (action.id.clone(), index))
        .collect();
    ActionTable { actions, by_id }
});

/// Built-in actions, borrowed from the shared table.
pub fn action_table() -> &'static [ActionDescriptor] {
    &ACTION_TABLE.actions
}

/// Look up a built-in action by id.
pub fn action_by_id(id: &str) -> Option<&'static ActionDescriptor> {
    ACTION_TABLE
        .by_id
        .get(id)
        .and_then(|&index| ACTION_TABLE.actions.get(index))
}

/// Actions that contribute to a context-menu surface.
pub fn actions_for_context(ctx: &str) -> Vec<ActionDescriptor> {
    action_table()
        .iter()
        .filter(|a| a.contexts.iter().any(|c| c == ctx))
        .cloned()
        .collect()
}

/// JSON for QML consumption.
pub fn actions_json() -> String {
    serde_json::to_string(action_table()).unwrap_or_else(|_| "[]".into())
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
    for action in action_table() {
        let Some(shortcut) = action.shortcut.as_deref() else {
            continue;
        };
        let chord = normalize_shortcut(shortcut);
        if chord.is_empty() {
            continue;
        }
        map.entry(chord).or_insert_with(|| action.id.clone());
    }
    map
}

/// Default action id → chord map (for menu / palette display).
pub fn default_action_shortcuts() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for action in action_table() {
        let Some(shortcut) = action.shortcut.as_deref() else {
            continue;
        };
        let chord = normalize_shortcut(shortcut);
        if !chord.is_empty() {
            map.entry(action.id.clone()).or_insert(chord);
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
    /// Every filter kind needs a Filter-menu entry, or nothing the user can
    /// reach creates it. Three kinds shipped that way.
    #[test]
    fn every_filter_kind_has_a_filter_menu_action() {
        let actions = default_actions();
        for params in crate::FilterParams::ALL_KINDS {
            let id = filter_action_id(params.kind_key());
            let action = actions
                .iter()
                .find(|a| a.id == id)
                .unwrap_or_else(|| panic!("{} has no action", params.kind_key()));
            assert_eq!(
                action.command_id.as_deref(),
                Some(command_id::FILTER_ADD_EFFECT)
            );
            assert_eq!(action.arg.as_deref(), Some(params.kind_key()));
        }
    }

    /// The other direction: an effect action naming a kind the engine cannot
    /// parse would report success and create nothing.
    #[test]
    fn every_filter_action_names_a_known_kind() {
        for action in default_actions() {
            if action.command_id.as_deref() != Some(command_id::FILTER_ADD_EFFECT) {
                continue;
            }
            let arg = action.arg.as_deref().unwrap_or_default();
            assert!(
                crate::FilterParams::default_for_kind(arg).is_some(),
                "{} names unknown filter kind {arg:?}",
                action.id
            );
        }
    }

    /// Every adjustment kind needs a Layer-menu entry, or it can be created
    /// by nothing the user can reach. Four kinds shipped that way.
    #[test]
    fn every_adjustment_kind_has_a_layer_menu_action() {
        let actions = default_actions();
        for params in crate::AdjustmentParams::ALL_KINDS {
            let id = adjustment_action_id(params.kind_key());
            let action = actions
                .iter()
                .find(|a| a.id == id)
                .unwrap_or_else(|| panic!("{} has no action", params.kind_key()));
            assert_eq!(
                action.command_id.as_deref(),
                Some(command_id::FILTER_ADD_ADJUSTMENT)
            );
            assert_eq!(action.arg.as_deref(), Some(params.kind_key()));
            assert_eq!(action.menu, ADJUSTMENT_SUBMENU);
        }
    }

    /// The other direction: an action carrying an adjustment argument the
    /// engine cannot parse would create nothing and report success.
    #[test]
    fn every_adjustment_action_names_a_known_kind() {
        for action in default_actions() {
            if action.command_id.as_deref() != Some(command_id::FILTER_ADD_ADJUSTMENT) {
                continue;
            }
            let arg = action.arg.as_deref().unwrap_or_default();
            assert!(
                crate::AdjustmentParams::default_for_kind(arg).is_some(),
                "{} names unknown adjustment kind {arg:?}",
                action.id
            );
        }
    }

    use super::*;
    use crate::command_id;
    use crate::selection::{SelectionModifyOp, parse_selection_modify_arg};
    use crate::shape_preset::ShapePreset;
    use std::collections::HashSet;

    /// Enablement is resolved per action per binding evaluation, so the lookup
    /// must not rebuild the table. Identity of the returned reference is the
    /// observable difference between borrowing the shared table and
    /// constructing a fresh one each call.
    #[test]
    fn action_lookup_borrows_one_shared_table() {
        let first = action_by_id("action.edit.undo").expect("undo registered");
        let second = action_by_id("action.edit.undo").expect("undo registered");
        assert!(
            std::ptr::eq(first, second),
            "action_by_id rebuilt the table instead of borrowing it"
        );
        assert!(std::ptr::eq(
            first,
            &action_table()[ACTION_TABLE.by_id["action.edit.undo"]]
        ));
        assert_eq!(action_by_id("action.does.not.exist"), None);
    }

    #[test]
    fn action_table_matches_the_builder() {
        assert_eq!(action_table(), default_actions().as_slice());
    }

    /// Every tool on the shelf must be reachable by key and by search, and
    /// must name a tool the host will actually accept — a typo or a missing
    /// entry in the host's known-tool list silently selects the brush instead.
    #[test]
    fn every_tool_has_an_action_with_a_shortcut() {
        let actions = action_table();
        for tool in crate::default_tools() {
            let action = actions
                .iter()
                .find(|a| a.arg.as_deref() == Some(tool.id.as_str()) && a.menu == "tools")
                .unwrap_or_else(|| panic!("{} has no action, so no key and no search", tool.id));
            assert_eq!(action.host_op.as_deref(), Some("tool.activate"));
            assert!(
                action.shortcut.as_deref().is_some_and(|s| !s.is_empty()),
                "{} has no shortcut",
                tool.id
            );
        }
    }

    /// Tools belong on the shelf and in search, not in the menu bar — the
    /// menu-building code filters on these names, so a stray tool menu would
    /// appear as a bar entry no editor of this kind has.
    #[test]
    fn tools_are_search_only_and_not_a_menu_bar_entry() {
        for action in action_table() {
            if action.id.starts_with("action.tool.") {
                assert_eq!(
                    action.menu, "tools",
                    "{} is not in the tools group",
                    action.id
                );
            }
            assert!(
                menu_has_a_home(&action.menu),
                "{} uses unknown menu {}",
                action.id,
                action.menu
            );
        }
    }

    /// Menu-bar entries, plus the search-only tools group.
    const BAR: [&str; 9] = [
        "file", "edit", "select", "image", "layer", "filter", "view", "window", "help",
    ];

    /// Whether the shell has somewhere to draw this menu name.
    ///
    /// A `parent.child` name is a submenu, so it needs its parent to be a bar
    /// entry — otherwise the actions are addressed to a menu that does not
    /// exist and vanish from the shell without any test noticing.
    fn menu_has_a_home(menu: &str) -> bool {
        if BAR.contains(&menu) || menu == "tools" {
            return true;
        }
        match menu.split_once('.') {
            Some((parent, child)) => BAR.contains(&parent) && !child.is_empty(),
            None => false,
        }
    }

    /// A submenu the engine declares must be declared by the shell too. QML
    /// builds each menu from an explicit `actionsForMenu` call, so a submenu
    /// nobody instantiates is a set of actions with no way in.
    #[test]
    fn every_submenu_is_declared_by_the_qml_shell() {
        let shell =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml/Main.qml"))
                .expect("qml/Main.qml is readable from the engine crate");
        let mut submenus: Vec<&str> = action_table()
            .iter()
            .map(|a| a.menu.as_str())
            .filter(|m| m.contains('.'))
            .collect();
        submenus.sort_unstable();
        submenus.dedup();
        assert!(
            !submenus.is_empty(),
            "no submenus found — the parse broke rather than the shell"
        );
        for menu in submenus {
            assert!(
                shell.contains(&format!("actionsForMenu(\"{menu}\")")),
                "{menu} is declared by the engine and instantiated by no menu"
            );
        }
    }

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

    /// Handbook 28: expert affordances must not remove menu/action-search
    /// discovery, so the panel-local disclosure actions live in the registry
    /// rather than only on the Properties header.
    #[test]
    fn disclosure_actions_are_menu_discoverable() {
        let actions = default_actions();
        for (id, host_op) in [
            ("action.view.expand-all-groups", "inspector.expand_all"),
            ("action.view.collapse-all-groups", "inspector.collapse_all"),
        ] {
            let action = actions
                .iter()
                .find(|a| a.id == id)
                .unwrap_or_else(|| panic!("{id} is not registered"));
            assert_eq!(action.menu, "view", "{id} must appear in the View menu");
            assert_eq!(action.host_op.as_deref(), Some(host_op));
            assert_eq!(
                action.enablement, "always",
                "{id} is workspace scope and needs no document"
            );
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

    /// The registry stores `selection.modify` arguments as opaque strings, so
    /// nothing in the type system connects `"contract:2"` to the op that runs.
    /// This is that connection: every shipped argument must parse, and every
    /// op the enum defines must be reachable from a menu.
    #[test]
    fn selection_modify_actions_carry_a_parsable_argument() {
        let modify: Vec<_> = default_actions()
            .into_iter()
            .filter(|a| a.host_op.as_deref() == Some("selection.modify"))
            .collect();
        assert!(
            !modify.is_empty(),
            "no action routes to selection.modify — the host op id moved and this test went blind"
        );

        let mut reached = Vec::new();
        for action in &modify {
            let arg = action.arg.as_deref().unwrap_or_else(|| {
                panic!("{} routes to selection.modify with no argument", action.id)
            });
            let (op, _radius) = parse_selection_modify_arg(arg).unwrap_or_else(|| {
                panic!("{} carries {arg:?}, which names no selection op", action.id)
            });
            reached.push(op);
        }

        for op in SelectionModifyOp::ALL {
            assert!(
                reached.contains(&op),
                "no action reaches SelectionModifyOp::{op:?} — it exists but the user cannot invoke it"
            );
        }
    }

    /// Same pairing as the selection ops, for the Layer menu's shape entries.
    /// Since an unknown kind now creates nothing at all, a registry arg that
    /// stopped parsing would be a menu entry that quietly does nothing.
    #[test]
    fn shape_create_actions_name_a_known_preset() {
        let creates: Vec<_> = default_actions()
            .into_iter()
            .filter(|a| a.host_op.as_deref() == Some("shape.create"))
            .collect();
        assert!(
            !creates.is_empty(),
            "no action routes to shape.create — the host op id moved and this test went blind"
        );

        let mut reached = Vec::new();
        for action in &creates {
            let arg = action
                .arg
                .as_deref()
                .unwrap_or_else(|| panic!("{} routes to shape.create with no kind", action.id));
            let preset = ShapePreset::parse(arg)
                .unwrap_or_else(|| panic!("{} carries {arg:?}, which names no shape", action.id));
            reached.push(preset);
        }

        for preset in ShapePreset::ALL {
            assert!(
                reached.contains(&preset),
                "no action reaches ShapePreset::{preset:?} — it exists but the user cannot create it"
            );
        }
    }
}
