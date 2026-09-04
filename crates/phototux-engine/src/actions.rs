//! Toolkit-neutral action descriptors (handbook parity P1.1).
//!
//! Presentations resolve `ActionDescriptor::id` → `command_id` and/or `host_op`.
//! Document mutations still enter [`crate::SessionState::invoke`].

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

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

/// Start an action descriptor; optional fields are added by the builder.
///
/// This used to be a nine-argument function, so every entry carried a run of
/// `None`s for the fields it did not use — `None, None, Some("gaussian"),
/// None, None` at the call site says nothing about which field is which, and
/// the runs made most of this file duplicate text. Naming only what an action
/// actually has is both readable and unique.
/// A display name, escaped so a menu can carry it as a label.
///
/// Menu labels are Qt-style: `&` marks the next character as the accelerator
/// and `&&` is a literal ampersand. Every hand-written label in this file
/// spells its own accelerator, but the generated families take their labels
/// from display vocabularies — `AdjustmentParams::label`, `ShapePreset::label`
/// and their siblings — which know nothing about accelerators. `Black & White`
/// went through the shell's mnemonic stripper and came out as `Black  White`,
/// because a lone `&` is a marker and the space after it is what it marked.
///
/// Escaping here rather than in the vocabularies keeps the ampersand out of
/// the *layer* name, which is the same string seen without a menu around it.
fn as_menu_label(display: &str) -> String {
    display.replace('&', "&&")
}

fn act(id: &str, label: &str, menu: &str, enablement: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: id.into(),
        label: label.into(),
        menu: menu.into(),
        command_id: None,
        host_op: None,
        arg: None,
        shortcut: None,
        icon_key: None,
        enablement: enablement.into(),
        contexts: Vec::new(),
    }
}

impl ActionDescriptor {
    /// The document command this action invokes.
    fn command(mut self, id: &str) -> Self {
        self.command_id = Some(id.to_owned());
        self
    }

    /// A host-only operation, for actions with no document command.
    fn host(mut self, op: &str) -> Self {
        self.host_op = Some(op.to_owned());
        self
    }

    /// The opaque argument carried to the command or host op.
    fn arg(mut self, arg: &str) -> Self {
        self.arg = Some(arg.to_owned());
        self
    }

    /// Default accelerator.
    fn key(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_owned());
        self
    }

    /// Icon key from `assets/icons/ICON_MAP.md`.
    fn icon(mut self, icon_key: &str) -> Self {
        self.icon_key = Some(icon_key.to_owned());
        self
    }
}

/// Menu path of the adjustment-layer submenu.
///
/// A `parent.child` menu name is a submenu of `parent`. The Layer menu already
/// carried thirty entries and overflowed the window on a 1080p display, so the
/// adjustment kinds go one level down rather than off the bottom edge — which
/// is also where every editor of this kind files them.
pub const ADJUSTMENT_SUBMENU: &str = "layer.adjustment";

/// Action id for the "add adjustment layer" entry of one adjustment kind.
fn adjustment_action_id(kind: &str) -> String {
    format!("action.layer.adj-{kind}")
}

/// Action id for the "add filter effect" entry of one filter kind.
fn filter_action_id(kind: &str) -> String {
    format!("action.filter.{kind}")
}

/// Action id for one panel's Window-menu toggle: `panel.layers` →
/// `action.window.panel-layers`.
fn panel_action_id(panel_id: &str) -> String {
    format!("action.window.{}", panel_id.replace('.', "-"))
}

/// Action id for one align-or-distribute entry.
fn align_action_id(key: &str) -> String {
    format!("action.layer.align-{key}")
}

/// Action id for the "add layer style" entry of one style kind.
fn style_action_id(kind: &str) -> String {
    format!("action.layer.{kind}")
}

/// Action id for activating one tool: `tool.select.rect` → `action.tool.select-rect`.
///
/// Only the dots *within* a tool family become dashes, which is the shape the
/// eighteen hand-written ids already had — a renamed action id drops a user's
/// custom shortcut for it without saying so.
fn tool_action_id(tool_id: &str) -> String {
    let rest = tool_id.strip_prefix("tool.").unwrap_or(tool_id);
    format!("action.tool.{}", rest.replace('.', "-"))
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
        act("action.file.new", "&New…", "file", "io_idle")
            .host("document.new")
            .key("Ctrl+N")
            .icon("file-plus"),
        act("action.file.open", "&Open…", "file", "io_idle")
            .host("document.open")
            .key("Ctrl+O")
            .icon("folder-open"),
        act("action.file.save", "&Save", "file", "has_document_io_idle")
            .host("document.save")
            .key("Ctrl+S")
            .icon("floppy-disk"),
        act(
            "action.file.save-as",
            "Save &As…",
            "file",
            "has_document_io_idle",
        )
        .host("document.save_as")
        .key("Ctrl+Shift+S"),
        act(
            "action.file.export",
            "&Export…",
            "file",
            "has_document_io_idle",
        )
        .host("document.export")
        // Photoshop's Export As, and the move off `Ctrl+Shift+E` is
        // deliberate: that chord is Merge Visible in Photoshop, and a user
        // pressing it here expecting a merge should not get a file dialog.
        .key("Ctrl+Alt+Shift+W")
        .icon("export"),
        act(
            "action.file.close",
            "&Close",
            "file",
            "has_document_io_idle",
        )
        .host("document.close")
        .key("Ctrl+W")
        .icon("x"),
        act("action.file.quit", "&Quit", "file", "always")
            .host("app.quit")
            .key("Ctrl+Q"),
        // Edit
        act("action.edit.undo", "&Undo", "edit", "can_undo")
            .command(command_id::HISTORY_UNDO)
            .key("Ctrl+Z")
            .icon("arrow-counter-clockwise"),
        act("action.edit.redo", "&Redo", "edit", "can_redo")
            .command(command_id::HISTORY_REDO)
            .key("Ctrl+Shift+Z")
            .icon("arrow-clockwise"),
        act("action.edit.preferences", "&Preferences…", "edit", "always")
            .command(command_id::APP_SHOW_PREFERENCES)
            .key("Ctrl+,"),
        // Select (presented under Edit or Select menu)
        act("action.select.all", "Select &All", "select", "has_document")
            .host("selection.select_all")
            .key("Ctrl+A"),
        act(
            "action.select.deselect",
            "Deselect",
            "select",
            "selection_active",
        )
        .host("selection.deselect")
        .key("Ctrl+D"),
        act(
            "action.select.invert",
            "&Invert Selection",
            "select",
            "has_document",
        )
        .host("selection.invert")
        .key("Ctrl+Shift+I"),
        act(
            "action.select.selection-to-mask",
            "Selection to &Mask",
            "select",
            "selection_active",
        )
        .command(command_id::SELECTION_TO_MASK),
        act(
            "action.select.mask-to-selection",
            "Mask to Se&lection",
            "select",
            "has_mask",
        )
        .command(command_id::MASK_TO_SELECTION),
        act("action.edit.copy", "&Copy", "edit", "selection_active")
            .host("clipboard.copy")
            .key("Ctrl+C"),
        act(
            "action.edit.copy-selection-mask",
            "Copy &Selection Mask",
            "edit",
            "selection_active",
        )
        .host("clipboard.copy_selection_mask")
        .icon("selection-background"),
        act(
            "action.edit.copy-layer-mask",
            "Copy Layer &Mask",
            "edit",
            "has_mask",
        )
        .host("clipboard.copy_layer_mask")
        .icon("circle-half"),
        act(
            "action.edit.paste-layer",
            "Paste as New Layer",
            "edit",
            "has_document",
        )
        .host("clipboard.paste_layer")
        .key("Ctrl+V"),
        act(
            "action.edit.paste-selection",
            "Paste as &Selection",
            "edit",
            "has_document",
        )
        .host("clipboard.paste_selection")
        .icon("selection-foreground"),
        act(
            "action.edit.paste-mask",
            "Paste as Layer M&ask",
            "edit",
            "has_document",
        )
        .host("clipboard.paste_mask")
        .icon("circle-half-tilt"),
        // Edit > Transform — the *layer* flips. They used to sit in the Image
        // menu, where Photoshop's entries of the same name mirror the whole
        // canvas, so picking one on a five-layer document mirrored one layer
        // and read as a bug. Photoshop keeps a layer flip under
        // Edit > Transform, and so does this.
        act(
            "action.image.flip-h",
            "Flip &Horizontal",
            "edit.transform",
            "has_document_io_idle",
        )
        .host("raster.flip")
        .arg("h")
        .icon("flip-horizontal"),
        act(
            "action.image.flip-v",
            "Flip &Vertical",
            "edit.transform",
            "has_document_io_idle",
        )
        .host("raster.flip")
        .arg("v")
        .icon("flip-vertical"),
        act(
            "action.image.size",
            "Image Si&ze…",
            "image",
            "has_document_io_idle",
        )
        .host("image.size")
        .key("Ctrl+Alt+I")
        .icon("frame-corners"),
        act(
            "action.image.canvas-size",
            "&Canvas Size…",
            "image",
            "has_document_io_idle",
        )
        .host("image.canvas-size")
        .key("Ctrl+Alt+C")
        .icon("arrows-out"),
        // Image > Image Rotation — the canvas. Photoshop's submenu, its order
        // and its names.
        act(
            "action.image.rotate-180",
            "&180°",
            "image.rotation",
            "has_document_io_idle",
        )
        .host("document.rotate")
        .arg("180")
        .icon("arrows-clockwise"),
        act(
            "action.image.rotate-90",
            "90° &Clockwise",
            "image.rotation",
            "has_document_io_idle",
        )
        .host("document.rotate")
        .arg("90")
        .icon("arrow-clockwise"),
        act(
            "action.image.rotate-270",
            "90° Counter Cloc&kwise",
            "image.rotation",
            "has_document_io_idle",
        )
        .host("document.rotate")
        .arg("270")
        .icon("arrow-counter-clockwise"),
        act(
            "action.image.flip-canvas-h",
            "Flip Canvas &Horizontal",
            "image.rotation",
            "has_document_io_idle",
        )
        .host("document.flip")
        .arg("h")
        .icon("flip-horizontal"),
        act(
            "action.image.flip-canvas-v",
            "Flip Canvas &Vertical",
            "image.rotation",
            "has_document_io_idle",
        )
        .host("document.flip")
        .arg("v")
        .icon("flip-vertical"),
        act(
            "action.image.assign-srgb",
            "Assign Profile: sRGB",
            "image.color",
            "has_document",
        )
        .command(command_id::DOCUMENT_ASSIGN_PROFILE)
        .arg("sRGB"),
        act(
            "action.image.assign-p3",
            "Assign Profile: Display-P3",
            "image.color",
            "has_document",
        )
        .command(command_id::DOCUMENT_ASSIGN_PROFILE)
        .arg("Display-P3"),
        act(
            "action.image.convert-srgb",
            "Convert to sRGB",
            "image.color",
            "has_document_io_idle",
        )
        .command(command_id::DOCUMENT_CONVERT_PROFILE)
        .arg("sRGB"),
        act(
            "action.image.convert-p3",
            "Convert to Display-P3",
            "image.color",
            "has_document_io_idle",
        )
        .command(command_id::DOCUMENT_CONVERT_PROFILE)
        .arg("Display-P3"),
        act(
            "action.image.soft-proof-p3",
            "Soft-Proof: Display-P3",
            "image.color",
            "has_document",
        )
        .command(command_id::DOCUMENT_SET_SOFT_PROOF)
        .arg("Display-P3:relative"),
        act(
            "action.image.soft-proof-off",
            "Soft-Proof: Off",
            "image.color",
            "has_document",
        )
        .command(command_id::DOCUMENT_SET_SOFT_PROOF)
        .arg(":relative"),
        act(
            "action.image.embed-icc",
            "Embed &ICC Profile…",
            "image.color",
            "has_document_io_idle",
        )
        .host("document.embed_icc"),
        act(
            "action.image.clear-icc",
            "Clear Embedded ICC",
            "image.color",
            "has_document",
        )
        .command(command_id::DOCUMENT_SET_ICC)
        .arg("clear"),
        // Layer
        act(
            "action.layer.new-raster",
            "New &Layer",
            "layer",
            "has_document",
        )
        .command(command_id::LAYER_CREATE)
        .key("Ctrl+Shift+N"),
        act(
            "action.layer.duplicate",
            "&Duplicate Layer",
            "layer",
            "has_document",
        )
        .host("layer.duplicate")
        .key("Ctrl+J")
        .icon("copy"),
        act(
            "action.layer.new-fill",
            "New &Fill Layer",
            "layer",
            "has_document",
        )
        .command(command_id::LAYER_CREATE_FILL),
        act(
            "action.layer.merge-down",
            "&Merge Down",
            "layer",
            "has_multiple_layers",
        )
        .host("layer.merge-down")
        .key("Ctrl+E")
        .icon("arrows-in-line-vertical"),
        act(
            "action.layer.merge-visible",
            "Merge &Visible",
            "layer",
            "has_multiple_layers",
        )
        .host("layer.merge-visible")
        .key("Ctrl+Shift+E")
        .icon("stack-minus"),
        // Photoshop shares `Ctrl+E` between Merge Down and Merge Group and
        // decides by what is selected. The registry binds one chord to one
        // action, so this one carries no chord and Merge Down's refusal names
        // it instead — a dead end that says where to go is better than a
        // chord that means two things.
        act(
            "action.layer.merge-group",
            "Merge &Group",
            "layer",
            "group_selected",
        )
        .host("layer.merge-group")
        .icon("folder-minus"),
        act(
            "action.layer.flatten",
            "Flatten &Image",
            "layer",
            "has_multiple_layers",
        )
        .host("layer.flatten")
        .icon("stack-simple"),
        act(
            "action.layer.delete",
            "&Delete Layer",
            "layer",
            "has_multiple_layers",
        )
        .command(command_id::LAYER_DELETE),
        act(
            "action.layer.new-group",
            "New &Group",
            "layer",
            "has_document",
        )
        .command(command_id::LAYER_GROUP),
        act(
            "action.layer.ungroup",
            "&Ungroup",
            "layer",
            "group_selected",
        )
        .command(command_id::LAYER_UNGROUP),
        act(
            "action.layer.bake-text",
            "Bake &Text",
            "layer",
            "text_layer",
        )
        .host("text.bake"),
        act(
            "action.layer.rasterize-shape",
            "Rasterize Shape",
            "layer.shape",
            "shape_layer",
        )
        .host("shape.rasterize"),
        // Layer > Smart Objects, where Photoshop keeps them.
        act(
            "action.layer.convert-to-smart",
            "Convert to Smart &Object",
            "layer.smart",
            "has_document_io_idle",
        )
        .host("smart.create"),
        act(
            "action.layer.reset-smart",
            "&Reset Placement",
            "layer.smart",
            "smart_object",
        )
        .host("smart.reset"),
        act(
            "action.layer.rasterize-smart",
            "Rasterize Smart Object",
            "layer.smart",
            "smart_object",
        )
        .host("smart.rasterize"),
        act(
            "action.layer.stroke-path",
            "Stroke Path to Layer",
            "layer.shape",
            "has_document_io_idle",
        )
        .host("path.stroke"),
        act(
            "action.layer.apply-mask",
            "&Apply Mask",
            "layer.mask",
            "has_mask",
        )
        .command(command_id::MASK_APPLY),
        act(
            "action.layer.add-mask",
            "Add &Mask",
            "layer.mask",
            "no_mask",
        )
        .host("mask.create"),
        act(
            "action.layer.delete-mask",
            "Delete Mask",
            "layer.mask",
            "has_mask",
        )
        .host("mask.delete"),
        act(
            "action.layer.toggle-mask",
            "Toggle Mask Enabled",
            "layer.mask",
            "has_mask",
        )
        .host("mask.toggle_enabled"),
        act(
            "action.layer.add-vector-mask",
            "Add Vector Mask",
            "layer.mask",
            "has_document",
        )
        .command(command_id::MASK_CREATE_VECTOR),
        act(
            "action.layer.lock-pixels",
            "Lock Pixels",
            "layer.lock",
            "has_document",
        )
        .command(command_id::LAYER_SET_LOCKS)
        .arg("pixels"),
        act(
            "action.layer.lock-position",
            "Lock Position",
            "layer.lock",
            "has_document",
        )
        .command(command_id::LAYER_SET_LOCKS)
        .arg("position"),
        act(
            "action.layer.lock-all",
            "Lock All",
            "layer.lock",
            "has_document",
        )
        .command(command_id::LAYER_SET_LOCKS)
        .arg("all"),
        act(
            "action.layer.toggle-clip",
            "Create Clipping Mask",
            "layer",
            "has_document",
        )
        .host("layer.toggle_clip"),
        // Filter
        // Opening the gallery does not change a layer, so the command is not
        // in `CHANGES_ACTIVE_LAYER` — but everything the gallery then offers
        // is, and a dialog that opens only to refuse Preview and Apply is
        // worse than an entry that says up front it is unavailable.
        act(
            "action.filter.gallery",
            "Filter &Gallery…",
            "filter",
            "active_layer_unlocked",
        )
        .command(command_id::APP_SHOW_FILTER_GALLERY),
        // View
        act("action.view.zoom-in", "Zoom &In", "view", "has_document")
            .command(command_id::VIEW_ZOOM_IN)
            // `Ctrl+=` rather than Photoshop's printed `Ctrl++`: the plus is
            // a shifted key on most layouts, only one chord can be bound to
            // an action, and this is the one people actually press. Photoshop
            // accepts both.
            .key("Ctrl+=")
            .icon("magnifying-glass-plus"),
        act("action.view.zoom-out", "Zoom &Out", "view", "has_document")
            .command(command_id::VIEW_ZOOM_OUT)
            .key("Ctrl+-")
            .icon("magnifying-glass-minus"),
        act(
            "action.view.zoom-actual",
            "&Actual Pixels",
            "view",
            "has_document",
        )
        .command(command_id::VIEW_ZOOM_ACTUAL)
        .key("Ctrl+1")
        .icon("frame-corners"),
        act(
            "action.view.zoom-fit",
            "&Fit on Screen",
            "view",
            "has_document",
        )
        .command(command_id::VIEW_ZOOM_TO_FIT)
        .key("Ctrl+0")
        .icon("corners-in"),
        // These four icons were packaged into the qrc and referenced by
        // nothing — they are the toggles they were cut for, and the menu draws
        // an icon when an entry has one. A View menu where half the entries
        // carry a glyph and half do not reads as unfinished rather than as a
        // distinction.
        act(
            "action.view.toggle-guides",
            "Show &Guides",
            "view",
            "always",
        )
        .host("view.toggle_guides")
        .icon("rectangle-dashed"),
        act("action.view.toggle-grid", "Show G&rid", "view", "always")
            .host("view.toggle_grid")
            .icon("grid-four"),
        act(
            "action.view.toggle-rulers",
            "Show &Rulers",
            "view",
            "always",
        )
        .host("view.toggle_rulers")
        .icon("ruler"),
        act("action.view.toggle-snap", "Sna&p", "view", "always")
            .host("view.toggle_snap")
            .icon("magnet"),
        act(
            "action.view.guide-v",
            "New Vertical Guide",
            "view",
            "has_document",
        )
        .host("view.guide_v"),
        act(
            "action.view.guide-h",
            "New Horizontal Guide",
            "view",
            "has_document",
        )
        .host("view.guide_h"),
        act(
            "action.view.clear-guides",
            "Clear Guides",
            "view",
            "has_document",
        )
        .host("view.clear_guides"),
        // Panel-local view actions (handbook 05): workspace scope, no document
        // mutation, so they stay available with no document open.
        act(
            "action.view.expand-all-groups",
            "E&xpand All Property Groups",
            "view",
            "always",
        )
        .host("inspector.expand_all")
        .icon("arrows-out-line-vertical"),
        act(
            "action.view.collapse-all-groups",
            "&Collapse All Property Groups",
            "view",
            "always",
        )
        .host("inspector.collapse_all")
        .icon("arrows-in-line-vertical"),
        // Window
        act("action.window.reset", "Reset Workspace", "window", "always")
            .command(command_id::WORKSPACE_RESET),
        act(
            "action.window.preset-essentials",
            "Workspace: Essentials",
            "window",
            "always",
        )
        .command(command_id::WORKSPACE_APPLY_PRESET)
        .arg("workspace.preset.essentials"),
        act(
            "action.window.preset-compact",
            "Workspace: Compact",
            "window",
            "always",
        )
        .command(command_id::WORKSPACE_APPLY_PRESET)
        .arg("workspace.preset.compact"),
        act(
            "action.window.preset-painting",
            "Workspace: Painting",
            "window",
            "always",
        )
        .command(command_id::WORKSPACE_APPLY_PRESET)
        .arg("workspace.preset.painting"),
        act(
            "action.window.preset-factory",
            "Workspace: Factory defaults",
            "window",
            "always",
        )
        .command(command_id::WORKSPACE_APPLY_PRESET)
        .arg("workspace.preset.factory"),
        // Help
        act("action.help.about", "&About PhotoTux", "help", "always")
            .host("help.about")
            .icon("info"),
        // App chrome
        act(
            "action.app.command-palette",
            "Command &Palette…",
            "edit",
            "always",
        )
        .host("palette.open")
        .key("Ctrl+Shift+P"),
        act(
            "action.app.simulate-device-lost",
            "Simulate Device Lost (debug)",
            "view",
            "has_document",
        )
        .host("app.simulate_device_lost"),
        act(
            "action.app.recover-gpu",
            "&Recover graphics…",
            "view",
            "has_document",
        )
        .host("app.recover_gpu")
        .icon("arrows-clockwise"),
    ];
    // One Select > Modify entry per modify op, generated from the vocabulary.
    //
    // A conformance test already refuses an op the user cannot invoke, and it
    // fired the moment Smooth and Border were added — three hand-written
    // entries against what is now a five-op vocabulary.
    //
    // They sit in a submenu because Photoshop's do, and because five entries
    // that each open a radius prompt are the depth of the Select menu rather
    // than its first screen.
    actions.extend(crate::SelectionModifyOp::ALL.into_iter().map(|op| {
        act(
            &format!("action.select.{}", op.action_suffix()),
            &as_menu_label(op.label()),
            "select.modify",
            "selection_active",
        )
        .host("selection.modify")
        .arg(&format!("{}:{}", op.as_str(), op.menu_radius()))
    }));
    // Layer ▸ Arrange, generated from `ArrangeOp` for the same reason the
    // families below are generated from theirs: the ops carry their own
    // labels and Photoshop's chords, and a hand-written list here would be a
    // second copy free to drift from them.
    //
    // A submenu because Photoshop's is one, and because the Layer menu already
    // runs to the bottom of a 1080p window.
    actions.extend(crate::ArrangeOp::ALL.into_iter().map(|op| {
        act(
            &format!("action.layer.arrange-{}", op.as_str()),
            &as_menu_label(op.label()),
            "layer.arrange",
            "has_multiple_layers",
        )
        .command(command_id::LAYER_ARRANGE)
        .arg(op.as_str())
        .key(op.shortcut())
    }));
    // One Shape submenu entry per preset, and one Combine entry per boolean
    // op. Both were hand-written lists restating vocabularies that already
    // exist — `ShapePreset` and `BooleanOp` — with the same silent drift risk:
    // a preset added to the engine with no menu entry can be created by
    // nothing the user can reach.
    actions.extend(crate::ShapePreset::ALL.into_iter().map(|preset| {
        act(
            &format!("action.layer.shape-{}", preset.as_str()),
            &as_menu_label(preset.label()),
            "layer.shape",
            "has_document",
        )
        .host("shape.create")
        .arg(preset.as_str())
    }));
    actions.extend(crate::BooleanOp::ALL.into_iter().map(|op| {
        act(
            &format!("action.layer.shape-{}", op.as_str()),
            &as_menu_label(op.label()),
            "layer.boolean",
            "has_document_io_idle",
        )
        .command(command_id::SHAPE_BOOLEAN)
        .arg(op.as_str())
    }));
    // One action per tool, generated from the tool descriptors.
    //
    // These were eighteen hand-written entries restating each tool's title,
    // icon and accelerator — a second list of the tool vocabulary, and the
    // largest duplicated block in this file. `tools` is a search-only menu:
    // tools belong on the shelf and in action search, not in the menu bar,
    // which is why these labels carry no mnemonic.
    actions.extend(crate::default_tools().into_iter().map(|tool| {
        act(
            &tool_action_id(&tool.id),
            &as_menu_label(&tool.title),
            "tools",
            "has_document",
        )
        .host("tool.activate")
        .arg(&tool.id)
        .key(&tool.shortcut)
        .icon(&tool.icon_key)
    }));
    // One Window-menu toggle per panel, generated from the panel vocabulary.
    //
    // These were seven hand-written entries restating `default_panels()`, and
    // two of them named panels the shell does not draw — so the menu offered
    // toggles that changed the persisted workspace and put nothing on screen.
    // Generating them means a panel cannot be offered unless it is declared,
    // and a companion test means it cannot be declared unless it is drawn.
    actions.extend(crate::default_panels().into_iter().map(|panel| {
        act(
            &panel_action_id(&panel.id),
            &as_menu_label(&panel.title),
            "window",
            "always",
        )
        .command(command_id::WORKSPACE_TOGGLE_PANEL)
        .arg(&panel.id)
    }));
    // Align and Distribute, generated from `AlignOp`. Two submenus rather
    // than one, and directly under Layer, because that is where Photoshop
    // files them — a user arriving from Photoshop should find them without
    // hunting. Distribution is enabled only from three layers up: with two,
    // the ends are already fixed and the command would accept the click and
    // then do nothing.
    actions.extend(crate::AlignOp::ALL.into_iter().map(|op| {
        act(
            &align_action_id(op.as_str()),
            &as_menu_label(op.label()),
            if op.is_distribute() {
                "layer.distribute"
            } else {
                "layer.align"
            },
            if op.is_distribute() {
                "has_three_layers"
            } else {
                "has_document"
            },
        )
        .host("layer.align")
        .arg(op.as_str())
        .icon(op.icon_key())
    }));
    // One Layer Style submenu entry per style kind. Four hand-written entries
    // against a four-variant enum was not yet drift, but each also needed its
    // own command id, router arm and handler — which is why the set stood at
    // four. The kind keys are chosen so these ids match the ones that shipped.
    actions.extend(crate::LayerStyle::ALL_KINDS.iter().map(|style| {
        act(
            &style_action_id(style.kind_key()),
            &as_menu_label(style.label()),
            "layer.style",
            "has_document",
        )
        .command(command_id::STYLE_ADD)
        .arg(style.kind_key())
    }));
    // One Filter-menu entry per filter kind, generated the same way and for
    // the same reason: five hand-written entries against a thirteen-kind
    // vocabulary left Box Blur, Invert and Offset with no way in.
    actions.extend(crate::FilterParams::ALL_KINDS.iter().map(|params| {
        act(
            &filter_action_id(params.kind_key()),
            &as_menu_label(params.label()),
            "filter",
            "has_document",
        )
        .command(command_id::FILTER_ADD_EFFECT)
        .arg(params.kind_key())
    }));
    // One Layer-menu entry per adjustment kind, generated from the vocabulary.
    //
    // These were three hand-written entries against seven kinds, so
    // Hue/Saturation, Invert, Threshold and Posterize had no way into the
    // document from the chrome at all — the same four the composite shader
    // was ignoring. Two independent lists, one silence.
    actions.extend(crate::AdjustmentParams::ALL_KINDS.iter().map(|params| {
        act(
            &adjustment_action_id(params.kind_key()),
            &as_menu_label(params.label()),
            ADJUSTMENT_SUBMENU,
            "has_document",
        )
        .command(command_id::FILTER_ADD_ADJUSTMENT)
        .arg(params.kind_key())
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
    set_contexts(&mut actions, "action.view.zoom-in", &["canvas"]);
    set_contexts(&mut actions, "action.view.zoom-out", &["canvas"]);
    set_contexts(&mut actions, "action.view.zoom-actual", &["canvas"]);
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
    let parts: Vec<&str> = raw.split('+').map(str::trim).collect();
    // `Ctrl++` means Ctrl plus the plus key, and splitting on '+' leaves two
    // empty tails to say so. Without this case the key is filtered away with
    // them and the chord collapses to a bare `Ctrl`, which binds nothing,
    // reports nothing, and is what a user rebinding Zoom In would get.
    let plus_is_the_key =
        parts.len() >= 2 && parts[parts.len() - 1].is_empty() && parts[parts.len() - 2].is_empty();
    let head = if plus_is_the_key {
        &parts[..parts.len() - 2]
    } else {
        &parts[..]
    };
    let mut chord: Vec<String> = head
        .iter()
        .filter(|p| !p.is_empty())
        .map(|part| normalize_chord_part(part))
        .filter(|p| !p.is_empty())
        .collect();
    if plus_is_the_key {
        chord.push("+".to_owned());
    }
    chord.join("+")
}

/// One `+`-separated piece of a chord, in the spelling the map is keyed by.
fn normalize_chord_part(part: &str) -> String {
    match part.to_ascii_lowercase().as_str() {
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
    /// Every tool needs an action, at the id it has always had.
    ///
    /// These were eighteen hand-written entries restating the tool
    /// descriptors' title, icon and accelerator; generating them removed the
    /// second list, and the ids are asserted literally because a renamed
    /// action id drops a user's custom shortcut for it without saying so.
    #[test]
    fn every_tool_has_an_action_at_its_shipped_id() {
        let actions = default_actions();
        for tool in crate::default_tools() {
            let id = tool_action_id(&tool.id);
            let action = actions
                .iter()
                .find(|a| a.id == id)
                .unwrap_or_else(|| panic!("{} has no action at {id}", tool.id));
            assert_eq!(action.host_op.as_deref(), Some("tool.activate"));
            assert_eq!(action.arg.as_deref(), Some(tool.id.as_str()));
            assert_eq!(action.label, tool.title);
            assert_eq!(action.icon_key.as_deref(), Some(tool.icon_key.as_str()));
            assert_eq!(action.shortcut.as_deref(), Some(tool.shortcut.as_str()));
        }
        for id in [
            "action.tool.move",
            "action.tool.select-rect",
            "action.tool.select-ellipse",
            "action.tool.select-lasso",
            "action.tool.select-polygon",
            "action.tool.brush",
            "action.tool.eraser",
            "action.tool.fill",
            "action.tool.gradient",
            "action.tool.eyedropper",
            "action.tool.text",
            "action.tool.shape",
            "action.tool.path-edit",
            "action.tool.crop",
            "action.tool.transform",
            "action.tool.pan",
            "action.tool.zoom",
        ] {
            assert!(
                actions.iter().any(|a| a.id == id),
                "{id} disappeared; a user shortcut bound to it would be dropped"
            );
        }
    }

    /// Every layer style needs a menu entry naming a kind the engine knows.
    /// The ids are asserted literally because a renamed action id drops a
    /// user's custom shortcut for it without saying so.
    #[test]
    fn every_layer_style_has_a_menu_action() {
        let actions = default_actions();
        for style in crate::LayerStyle::ALL_KINDS {
            let id = style_action_id(style.kind_key());
            let action = actions
                .iter()
                .find(|a| a.id == id)
                .unwrap_or_else(|| panic!("{} has no action", style.kind_key()));
            assert_eq!(action.command_id.as_deref(), Some(command_id::STYLE_ADD));
            assert_eq!(action.arg.as_deref(), Some(style.kind_key()));
            assert_eq!(action.menu, "layer.style");
            assert!(crate::LayerStyle::default_for_kind(style.kind_key()).is_some());
        }
        // The four ids that shipped before these entries were generated.
        for id in [
            "action.layer.drop-shadow",
            "action.layer.stroke-style",
            "action.layer.outer-glow",
            "action.layer.color-overlay",
        ] {
            assert!(
                actions.iter().any(|a| a.id == id),
                "{id} disappeared; a user shortcut bound to it would be dropped"
            );
        }
    }

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
    /// A declared panel the shell does not draw is a menu entry that lies.
    ///
    /// `panel.paths` and `panel.character` were exactly that: declared here,
    /// offered as Window-menu toggles, rendered nowhere. Toggling one changed
    /// the persisted workspace and put nothing on screen, with no feedback of
    /// any kind. The shell names the panels it draws in `panelShowsInDock`
    /// calls, which is the only place that fact exists, so this reads them.
    #[test]
    fn every_declared_panel_is_drawn_by_the_qml_shell() {
        let shell =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml/Main.qml"))
                .expect("qml/Main.qml is readable from the engine crate");
        for panel in crate::default_panels() {
            assert!(
                shell.contains(&format!("panelShowsInDock(\"{}\")", panel.id)),
                "{} is declared by the engine and drawn by no dock",
                panel.id
            );
        }
    }

    /// The shell reads `lastAnnounce` and says it out loud.
    ///
    /// Nineteen command handlers call `announce`, and the string reached QML
    /// through a property that nothing was bound to — published, exposed, and
    /// silent. The two halves that make it audible are a live region carrying
    /// the text and an `Accessible.announce` call raising the event; a shell
    /// that keeps one and drops the other is back to writing announcements
    /// nobody hears. Handbook 29 — Events and Announcements.
    #[test]
    fn the_shell_speaks_the_engines_announcements() {
        let shell =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../qml/Main.qml"))
                .expect("qml/Main.qml is readable from the engine crate");
        assert!(
            shell.contains("Accessible.name: AppSession.lastAnnounce"),
            "no live region carries the announcement text"
        );
        assert!(
            shell.contains("Accessible.announce("),
            "the live region never raises an announcement event"
        );
    }

    #[test]
    fn every_panel_has_exactly_one_window_menu_toggle() {
        let toggles: Vec<&ActionDescriptor> = action_table()
            .iter()
            .filter(|a| a.command_id.as_deref() == Some(command_id::WORKSPACE_TOGGLE_PANEL))
            .collect();
        let panels = crate::default_panels();
        assert_eq!(
            toggles.len(),
            panels.len(),
            "the Window menu and the panel vocabulary disagree on how many panels exist"
        );
        for panel in &panels {
            assert!(
                toggles
                    .iter()
                    .any(|a| a.arg.as_deref() == Some(panel.id.as_str())),
                "{} has no Window-menu toggle",
                panel.id
            );
        }
    }

    #[test]
    fn panel_toggle_ids_match_the_ones_that_shipped() {
        // The shell switches on these literally, and a renamed action id drops
        // a user's custom shortcut for it without saying so.
        for (panel, action) in [
            ("panel.layers", "action.window.panel-layers"),
            ("panel.properties", "action.window.panel-properties"),
            ("panel.navigator", "action.window.panel-navigator"),
            ("panel.swatches", "action.window.panel-swatches"),
            ("panel.history", "action.window.panel-history"),
        ] {
            assert_eq!(panel_action_id(panel), action);
            assert!(action_by_id(action).is_some(), "{action} is missing");
        }
    }

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

    /// Every punctuation chord the registry declares must survive
    /// normalisation unchanged.
    ///
    /// The shell binds one `Shortcut` per key of `default_shortcut_map`, and
    /// that map is keyed by `normalize_shortcut(declared)`. A chord the
    /// normaliser *rewrites* therefore reaches the map under a spelling the
    /// menu never shows, and the entry gets a printed accelerator that does
    /// nothing. Letters are safe by construction — a one-character part is
    /// upper-cased, a longer one has its first letter capitalised — so the
    /// hazard is entirely in the punctuation: `]`, `[`, `,`, `=`, `-` and the
    /// `+` that `Ctrl++` splits into nothing at all.
    ///
    /// Deliberately *not* an assertion that the chord is in the map: the map
    /// is built from these same declarations, so that comparison can only
    /// agree with itself. Collisions are `default_shortcut_chords_unique`'s
    /// job.
    #[test]
    fn punctuation_chords_survive_normalisation_unchanged() {
        let mut checked = 0;
        for action in default_actions() {
            let Some(declared) = action.shortcut.as_deref() else {
                continue;
            };
            let key = declared.rsplit('+').next().unwrap_or(declared);
            if key.is_empty() || key.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }
            checked += 1;
            assert_eq!(
                normalize_shortcut(declared),
                declared,
                "{} declares {declared}, which normalisation rewrites — the \
                 menu would print a chord the shell never binds",
                action.id
            );
        }
        assert!(
            checked >= 4,
            "found {checked} punctuation chords — the scan broke rather than \
             the registry"
        );
    }

    #[test]
    fn normalize_shortcut_stable() {
        assert_eq!(normalize_shortcut("ctrl+z"), "Ctrl+Z");
        assert_eq!(normalize_shortcut("Ctrl+Shift+Z"), "Ctrl+Shift+Z");
        assert_eq!(normalize_shortcut("ctrl + shift + z"), "Ctrl+Shift+Z");
    }

    /// `+` is a key, not only a separator.
    ///
    /// Splitting on '+' used to throw the key away with the empty pieces
    /// either side of it, so a user who bound Zoom In to `Ctrl++` — the
    /// binding Photoshop prints — silently got `Ctrl`, which Qt cannot
    /// activate and nothing reported.
    #[test]
    fn a_chord_can_end_on_the_plus_key() {
        assert_eq!(normalize_shortcut("Ctrl++"), "Ctrl++");
        assert_eq!(normalize_shortcut("ctrl+shift++"), "Ctrl+Shift++");
        assert_eq!(normalize_shortcut("+"), "+");
        // A dangling separator is still a dangling separator.
        assert_eq!(normalize_shortcut("Ctrl+"), "Ctrl");
        assert_eq!(normalize_shortcut(""), "");
    }

    /// Every `&` in an action label is a mnemonic marker or an escaped literal.
    ///
    /// Qt reads `&` as "the next character is the accelerator" and `&&` as a
    /// literal ampersand, and the shell strips the markers itself now that it
    /// draws menu rows by hand. A lone `&` before a space is neither: the
    /// stripper eats it *and* leaves the space, which is how the Black & White
    /// adjustment reached the menu as "Black  White". Generated families take
    /// their labels from display vocabularies that know nothing about
    /// accelerators, so each goes through `as_menu_label`; this fails if a new
    /// one forgets.
    #[test]
    fn every_action_label_escapes_its_ampersands() {
        for action in action_table() {
            let chars: Vec<char> = action.label.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] != '&' {
                    i += 1;
                    continue;
                }
                match chars.get(i + 1) {
                    // An escaped literal.
                    Some('&') => i += 2,
                    // A mnemonic marks a character you can actually press.
                    Some(next) if next.is_alphanumeric() => i += 2,
                    other => panic!(
                        "action {} has a stray `&` in {:?} (followed by {:?}). \
                         Menu labels are Qt-style: `&x` marks an accelerator and \
                         `&&` is a literal ampersand. A generated label should go \
                         through `as_menu_label`.",
                        action.id, action.label, other
                    ),
                }
            }
        }
    }

    #[test]
    fn a_display_name_becomes_a_menu_label_with_its_ampersand_intact() {
        assert_eq!(as_menu_label("Black & White"), "Black && White");
        assert_eq!(as_menu_label("Levels"), "Levels");
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
