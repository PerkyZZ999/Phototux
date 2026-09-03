//! The command vocabulary: identifiers, arguments, effects and errors.
//!
//! Split out of `commands.rs` because it is the half with no private surface —
//! every item here is `pub`, so it moves without widening anything. The router
//! and its eighty-seven command bodies stay together in the parent: those
//! methods are private by design, and separating them by family would mean
//! making all of them `pub(crate)` to keep the dispatch able to call them,
//! trading real encapsulation for shorter files.

use thiserror::Error;

use crate::error::DocumentError;
use crate::history::HistoryKind;
use crate::layer::{LayerId, ShapeContent, TextContent};
use crate::selection::{SelectionCombine, SelectionModifyOp, SelectionRect, SelectionShape};

pub mod command_id {
    pub const HISTORY_UNDO: &str = "history.undo";
    pub const HISTORY_REDO: &str = "history.redo";

    pub const LAYER_CREATE: &str = "layer.create";
    pub const LAYER_CREATE_FILL: &str = "layer.create-fill";
    pub const LAYER_SET_FILL_COLOR: &str = "layer.set-fill-color";
    /// Copy the active layer, record and pixels, directly above itself.
    pub const LAYER_DUPLICATE: &str = "layer.duplicate";
    /// Composite the active layer onto the one below it. Destructive.
    pub const LAYER_MERGE_DOWN: &str = "layer.merge-down";
    /// Composite every visible layer into one, keeping the hidden ones.
    pub const LAYER_MERGE_VISIBLE: &str = "layer.merge-visible";
    /// Composite a group's contents into one layer and drop the group.
    pub const LAYER_MERGE_GROUP: &str = "layer.merge-group";
    /// Composite every visible layer into one. Destructive by design.
    pub const LAYER_FLATTEN: &str = "layer.flatten";
    pub const LAYER_DELETE: &str = "layer.delete";
    pub const LAYER_SET_ACTIVE: &str = "layer.set-active";
    pub const LAYER_SET_VISIBILITY: &str = "layer.set-visibility";
    pub const LAYER_SET_OPACITY: &str = "layer.set-opacity";
    pub const LAYER_SET_BLEND: &str = "layer.set-blend";
    pub const LAYER_REORDER: &str = "layer.reorder";
    pub const LAYER_ARRANGE: &str = "layer.arrange";
    pub const LAYER_GROUP: &str = "layer.group";
    pub const LAYER_UNGROUP: &str = "layer.ungroup";
    pub const LAYER_SET_CLIP: &str = "layer.set-clip";
    pub const LAYER_SET_LOCKS: &str = "layer.set-locks";
    /// Align or distribute the selected layers by their measured content.
    ///
    /// One command for all eight operations rather than eight commands: they
    /// differ only in which edge they read, and a per-operation command would
    /// need a constant, a router arm, a handler and a taxonomy row each.
    pub const LAYER_ALIGN: &str = "layer.align";
    /// Replace the active layer's blend ranges ("Blend If").
    pub const LAYER_SET_BLEND_IF: &str = "layer.set-blend-if";

    pub const VIEW_ZOOM_TO: &str = "view.zoom-to";
    pub const VIEW_ZOOM_TO_FIT: &str = "view.zoom-to-fit";
    /// Step one zoom stop in / out about the viewport centre.
    pub const VIEW_ZOOM_IN: &str = "view.zoom-in";
    pub const VIEW_ZOOM_OUT: &str = "view.zoom-out";
    /// One image pixel per screen pixel.
    pub const VIEW_ZOOM_ACTUAL: &str = "view.zoom-actual";
    pub const VIEW_PAN_TO: &str = "view.pan-to";
    pub const VIEW_PAN_BY: &str = "view.pan-by";
    pub const VIEW_ZOOM_AT: &str = "view.zoom-at";
    pub const VIEW_SET_TOOL: &str = "view.set-tool";

    pub const DOCUMENT_NEW_PRESET: &str = "document.new-preset";
    pub const DOCUMENT_NEW_SIZE: &str = "document.new-size";
    pub const DOCUMENT_ASSIGN_PROFILE: &str = "document.assign-profile";
    pub const DOCUMENT_CONVERT_PROFILE: &str = "document.convert-profile";
    pub const DOCUMENT_SET_SOFT_PROOF: &str = "document.set-soft-proof";
    /// Embed or clear validated ICC profile bytes on the document.
    pub const DOCUMENT_SET_ICC: &str = "document.set-icc";
    /// Resample every layer to a new pixel size (Image Size).
    pub const DOCUMENT_RESIZE: &str = "document.resize";
    /// Change the canvas extent without resampling (Canvas Size).
    pub const DOCUMENT_CANVAS_SIZE: &str = "document.canvas-size";
    pub const DOCUMENT_CROP: &str = "document.crop";
    /// Rotate the canvas by a quarter-turn count (see `CommandArgs::Rotate`).
    pub const DOCUMENT_ROTATE: &str = "document.rotate";
    /// Mirror the whole canvas, every layer at once.
    pub const DOCUMENT_FLIP: &str = "document.flip";
    pub const HISTORY_JUMP: &str = "history.jump";

    pub const SELECTION_REPLACE: &str = "selection.replace";
    pub const SELECTION_DESELECT: &str = "selection.deselect";
    pub const SELECTION_INVERT: &str = "selection.invert";
    pub const SELECTION_SELECT_ALL: &str = "selection.select-all";
    pub const SELECTION_MODIFY: &str = "selection.modify";
    /// Select by colour from a seed pixel (magic wand / colour range).
    pub const SELECTION_COLOR_SELECT: &str = "selection.color-select";
    /// Copy pixel selection R8 into the active layer mask (host GPU).
    pub const SELECTION_TO_MASK: &str = "selection.to-mask";
    /// Load active layer mask R8 into the pixel selection channel (host GPU).
    pub const MASK_TO_SELECTION: &str = "mask.to-selection";

    pub const MASK_CREATE: &str = "mask.create";
    pub const MASK_DELETE: &str = "mask.delete";
    pub const MASK_SET_ENABLED: &str = "mask.set-enabled";
    pub const MASK_SET_ATTRIBUTES: &str = "mask.set-attributes";
    pub const MASK_CREATE_VECTOR: &str = "mask.create-vector";
    pub const MASK_APPLY: &str = "mask.apply";

    pub const TEXT_CREATE: &str = "text.create";
    pub const TEXT_SET_CONTENT: &str = "text.set-content";
    pub const TEXT_BAKE: &str = "text.bake";

    pub const SHAPE_CREATE: &str = "shape.create";
    pub const SHAPE_RASTERIZE: &str = "shape.rasterize";
    pub const SHAPE_BOOLEAN: &str = "shape.boolean";
    /// Recolour a shape layer without touching its geometry.
    pub const SHAPE_SET_APPEARANCE: &str = "shape.set-appearance";

    /// Wrap the active layer's pixels so a transform can be re-applied to
    /// them rather than accumulated on them (DR-032).
    pub const SMART_CREATE: &str = "smartobject.create";
    /// Replace a smart object's placement. Non-destructive: the source is
    /// restored and the whole placement re-applied, never composed.
    pub const SMART_SET_PLACEMENT: &str = "smartobject.set-placement";
    /// Bake a smart object down to ordinary pixels and drop its source.
    pub const SMART_RASTERIZE: &str = "smartobject.rasterize";

    pub const FILTER_ADD_ADJUSTMENT: &str = "filter.add-adjustment";
    pub const FILTER_SET_PARAMETERS: &str = "filter.set-parameters";
    pub const FILTER_ADD_EFFECT: &str = "filter.add-effect";
    pub const FILTER_SET_GAUSSIAN_RADIUS: &str = "filter.set-gaussian-radius";
    /// Start / refresh ephemeral filter gallery preview (no document dirty).
    pub const FILTER_PREVIEW: &str = "filter.preview";
    /// Update preview parameters while gallery is open.
    pub const FILTER_SET_PREVIEW_PARAMS: &str = "filter.set-preview-params";
    /// Commit preview into layer effects + filter plan (undoable).
    pub const FILTER_COMMIT: &str = "filter.commit";
    /// Abort preview session without mutating authority.
    pub const FILTER_CANCEL_PREVIEW: &str = "filter.cancel-preview";
    pub const EFFECT_REORDER: &str = "effect.reorder";
    pub const EFFECT_SET_ENABLED: &str = "effect.set-enabled";

    pub const PATH_SET_CLOSED: &str = "path.set-closed";
    pub const PATH_MOVE_ANCHOR: &str = "path.move-anchor";
    pub const PATH_ADD_ANCHOR: &str = "path.add-anchor";
    pub const PATH_DELETE_ANCHOR: &str = "path.delete-anchor";

    /// Add a layer style, named by kind.
    ///
    /// Replaces `style.add-drop-shadow` and its three siblings: one command
    /// per style meant a new style needed a constant, a router arm, a handler
    /// and a taxonomy row before it could exist at all, which is why the set
    /// stood at four.
    pub const STYLE_ADD: &str = "style.add";
    /// Edit one layer style's scalar parameters.
    pub const STYLE_SET_PARAMS: &str = "style.set-params";
    /// Replace one colour on one layer style.
    pub const STYLE_SET_COLOR: &str = "style.set-color";
    /// Enable or disable one layer style.
    pub const STYLE_SET_ENABLED: &str = "style.set-enabled";
    /// Remove one layer style.
    pub const STYLE_REMOVE: &str = "style.remove";

    pub const CLIPBOARD_PASTE_LAYER: &str = "clipboard.paste-layer";
    pub const PATH_STROKE_TO_LAYER: &str = "path.stroke-to-layer";

    pub const RASTER_TRANSFORM_COMMIT: &str = "raster.transform-commit";
    pub const RASTER_FLIP: &str = "raster.flip";
    pub const RASTER_FILL: &str = "raster.fill";
    pub const RASTER_GRADIENT: &str = "raster.gradient";
    pub const RASTER_PAINT_STROKE: &str = "raster.paint-stroke";

    /// Application chrome — host opens preferences dialog.
    pub const APP_SHOW_PREFERENCES: &str = "app.show-preferences";
    /// Application chrome — host opens filter gallery dialog.
    pub const APP_SHOW_FILTER_GALLERY: &str = "app.show-filter-gallery";
    /// Workspace chrome — reset panel visibility to Essentials.
    pub const WORKSPACE_RESET: &str = "workspace.reset";
    /// Workspace chrome — toggle a panel by id (`panel.layers`, …).
    pub const WORKSPACE_TOGGLE_PANEL: &str = "workspace.toggle-panel";
    /// Workspace chrome — apply a built-in layout preset by id.
    pub const WORKSPACE_APPLY_PRESET: &str = "workspace.apply-preset";

    /// Built-in commands registered for discovery / headless tests.
    pub const ALL: &[&str] = &[
        HISTORY_UNDO,
        HISTORY_REDO,
        HISTORY_JUMP,
        LAYER_CREATE,
        LAYER_CREATE_FILL,
        LAYER_SET_FILL_COLOR,
        LAYER_DUPLICATE,
        LAYER_MERGE_DOWN,
        LAYER_MERGE_VISIBLE,
        LAYER_MERGE_GROUP,
        LAYER_FLATTEN,
        LAYER_DELETE,
        LAYER_SET_ACTIVE,
        LAYER_SET_VISIBILITY,
        LAYER_SET_OPACITY,
        LAYER_SET_BLEND,
        LAYER_REORDER,
        LAYER_ARRANGE,
        LAYER_GROUP,
        LAYER_UNGROUP,
        LAYER_SET_CLIP,
        LAYER_SET_LOCKS,
        LAYER_ALIGN,
        LAYER_SET_BLEND_IF,
        VIEW_ZOOM_TO,
        VIEW_ZOOM_TO_FIT,
        VIEW_ZOOM_IN,
        VIEW_ZOOM_OUT,
        VIEW_ZOOM_ACTUAL,
        VIEW_PAN_TO,
        VIEW_PAN_BY,
        VIEW_ZOOM_AT,
        VIEW_SET_TOOL,
        DOCUMENT_NEW_PRESET,
        DOCUMENT_NEW_SIZE,
        DOCUMENT_ASSIGN_PROFILE,
        DOCUMENT_CONVERT_PROFILE,
        DOCUMENT_SET_SOFT_PROOF,
        DOCUMENT_SET_ICC,
        DOCUMENT_RESIZE,
        DOCUMENT_CANVAS_SIZE,
        DOCUMENT_CROP,
        DOCUMENT_ROTATE,
        DOCUMENT_FLIP,
        SELECTION_REPLACE,
        SELECTION_DESELECT,
        SELECTION_INVERT,
        SELECTION_SELECT_ALL,
        SELECTION_MODIFY,
        SELECTION_COLOR_SELECT,
        SELECTION_TO_MASK,
        MASK_TO_SELECTION,
        MASK_CREATE,
        MASK_DELETE,
        MASK_SET_ENABLED,
        MASK_SET_ATTRIBUTES,
        MASK_CREATE_VECTOR,
        MASK_APPLY,
        TEXT_CREATE,
        TEXT_SET_CONTENT,
        TEXT_BAKE,
        SHAPE_CREATE,
        SHAPE_RASTERIZE,
        SHAPE_BOOLEAN,
        SHAPE_SET_APPEARANCE,
        SMART_CREATE,
        SMART_SET_PLACEMENT,
        SMART_RASTERIZE,
        FILTER_ADD_ADJUSTMENT,
        FILTER_SET_PARAMETERS,
        FILTER_ADD_EFFECT,
        FILTER_SET_GAUSSIAN_RADIUS,
        FILTER_PREVIEW,
        FILTER_SET_PREVIEW_PARAMS,
        FILTER_COMMIT,
        FILTER_CANCEL_PREVIEW,
        EFFECT_REORDER,
        EFFECT_SET_ENABLED,
        PATH_SET_CLOSED,
        PATH_MOVE_ANCHOR,
        PATH_ADD_ANCHOR,
        PATH_DELETE_ANCHOR,
        STYLE_ADD,
        STYLE_SET_PARAMS,
        STYLE_SET_COLOR,
        STYLE_SET_ENABLED,
        STYLE_REMOVE,
        CLIPBOARD_PASTE_LAYER,
        PATH_STROKE_TO_LAYER,
        RASTER_TRANSFORM_COMMIT,
        RASTER_FLIP,
        RASTER_FILL,
        RASTER_GRADIENT,
        RASTER_PAINT_STROKE,
        APP_SHOW_PREFERENCES,
        APP_SHOW_FILTER_GALLERY,
        WORKSPACE_RESET,
        WORKSPACE_TOGGLE_PANEL,
        WORKSPACE_APPLY_PRESET,
    ];
}

/// Host follow-up after a successful command (canvas / pixel / chrome work).
#[derive(Debug, Clone, PartialEq)]
pub enum HostFollowUp {
    None,
    /// Rewrite all layer pixels from `from` profile to `to`, then mark converted.
    ConvertPixels {
        from: String,
        to: String,
    },
    /// Open the preferences dialog (application chrome).
    ShowPreferences,
    /// Reset workspace panel visibility to Essentials.
    ResetWorkspace,
    /// Toggle visibility of a dock panel by descriptor id.
    TogglePanel {
        panel_id: String,
    },
    /// Apply a built-in workspace layout preset.
    ApplyWorkspacePreset {
        preset_id: String,
    },
    /// Copy pixel selection into the active layer mask (GPU host).
    SelectionToMask,
    /// Copy active layer mask into the pixel selection channel (GPU host).
    MaskToSelection,
    /// Bake active layer mask into layer pixels, then clear mask (GPU host).
    ApplyMask,
    /// Move `steps` along the history timeline (host applies stroke/selection
    /// stacks). `forward` redoes rather than undoes: the panel lists undone
    /// steps too, so a click can land on either side of the present.
    HistoryJump {
        steps: u32,
        forward: bool,
    },
    /// Rasterize two shape layers, boolean-combine, write into `result` raster layer.
    ShapeBoolean {
        op: crate::BooleanOp,
        a: crate::LayerId,
        b: crate::LayerId,
        result: crate::LayerId,
    },
    /// Re-rasterize a shape layer after path edit (host GPU upload).
    RasterizeShape {
        id: crate::LayerId,
    },
    /// Re-place a smart object: restore its source pixels, then apply the
    /// whole placement to them. The host owns both, so the engine asks.
    PlaceSmartObject {
        id: crate::LayerId,
    },
    /// Open the filter gallery dialog (application chrome).
    ShowFilterGallery,
}

/// Parameters for [`SessionState::invoke`].
#[derive(Debug, Clone)]
pub enum CommandArgs {
    None,
    LayerIndex(i32),
    SetVisibility {
        index: i32,
        visible: bool,
    },
    SetOpacity {
        opacity: f32,
    },
    SetBlend {
        blend: String,
    },
    /// Align or distribute layers by boxes the host measured on the GPU.
    ///
    /// The boxes arrive as arguments because this crate cannot read pixels,
    /// and every layer here is document-sized — so the only meaningful edges
    /// are the ones around its visible content, which only the host can see.
    /// Deciding what to do with them stays in the engine.
    AlignLayers {
        op: crate::AlignOp,
        targets: Vec<crate::AlignTarget>,
    },
    /// The active layer's blend ranges, whole. One argument rather than a
    /// slot index and a value: the eight stops and the channel are read
    /// together by the shader, and a partial update would have to reconstruct
    /// the rest anyway.
    SetBlendIf {
        blend_if: crate::BlendIf,
    },
    Reorder {
        to_index: i32,
    },
    /// Move the selection one place, or all the way, through the stack.
    ///
    /// A relative move rather than a destination index, because that is what
    /// the menu entry means and because resolving it needs the stack: the
    /// registry carries a static argument, and "forward" is static while
    /// "index 4" is not.
    Arrange {
        op: String,
    },
    Zoom {
        zoom: f32,
    },
    Pan {
        world_x: f32,
        world_y: f32,
    },
    PanBy {
        dx: f32,
        dy: f32,
    },
    ZoomAt {
        factor: f32,
        anchor_x: f32,
        anchor_y: f32,
    },
    Tool {
        tool: String,
    },
    NewPreset {
        label: String,
    },
    NewSize {
        width: u32,
        height: u32,
    },
    AssignProfile {
        profile: String,
    },
    ConvertProfile {
        profile: String,
    },
    SelectionReplace {
        shape: SelectionShape,
        combine: SelectionCombine,
        rect: SelectionRect,
        polygon: Vec<(f32, f32)>,
        label: String,
    },
    SelectionModify {
        op: SelectionModifyOp,
        radius: u32,
    },
    MaskSetEnabled {
        enabled: bool,
    },
    LayerSetClip {
        clips: bool,
    },
    TextCreate {
        text: String,
    },
    TextSetContent {
        content: TextContent,
    },
    ShapeCreate {
        content: Box<ShapeContent>,
    },
    ShapeBoolean {
        op: String,
    },
    FillCreate {
        color_rgba: [f32; 4],
    },
    FillColor {
        color_rgba: [f32; 4],
    },
    FilterAdjustment {
        kind: String,
    },
    /// Editor slot values for the active adjustment, index-aligned with
    /// [`crate::AdjustmentParams::editor_slots`].
    FilterParameters {
        slots: [f32; crate::MAX_ADJUSTMENT_SLOTS],
    },
    /// Seed and tolerance for a colour-based selection.
    SelectionColorSelect {
        /// `true` floods from the seed (magic wand); `false` takes every
        /// matching pixel in the layer (colour range).
        contiguous: bool,
        tolerance: f32,
        combine: SelectionCombine,
    },
    /// Layer-style kind key; see [`crate::LayerStyle::kind_key`].
    LayerStyleKind {
        kind: String,
    },
    /// Scalar parameters for the style at `index` on the active layer.
    LayerStyleParams {
        index: usize,
        slots: [f32; crate::MAX_ADJUSTMENT_SLOTS],
    },
    /// One colour of the style at `index` on the active layer.
    LayerStyleColor {
        index: usize,
        color_index: usize,
        rgba: [f32; 4],
    },
    /// Enable flag for the style at `index` on the active layer.
    LayerStyleEnabled {
        index: usize,
        enabled: bool,
    },
    /// The style at `index` on the active layer.
    LayerStyleIndex {
        index: usize,
    },
    FilterEffect {
        kind: String,
    },
    FilterGaussianRadius {
        radius: f32,
    },
    FilterPreview {
        kind: String,
    },
    FilterPreviewParams {
        p0: f32,
        p1: f32,
        p2: f32,
    },
    EffectReorder {
        effect_id: u64,
        to_index: i32,
    },
    EffectSetEnabled {
        effect_id: u64,
        enabled: bool,
    },
    ShapeSetAppearance {
        appearance: crate::ShapeAppearance,
    },
    SmartCreate {
        content: Box<crate::SmartObjectContent>,
    },
    SmartSetPlacement {
        placement: crate::LayerTransform,
    },
    PathSetClosed {
        closed: bool,
    },
    PathMoveAnchor {
        index: usize,
        x: f32,
        y: f32,
    },
    PathAddAnchor {
        x: f32,
        y: f32,
        index: Option<usize>,
    },
    PathDeleteAnchor {
        index: usize,
    },
    PasteLayer {
        name: String,
    },
    PathStroke {
        layer_name: String,
    },
    RasterFlip {
        horizontal: bool,
    },
    RasterPaintStroke {
        label: String,
    },
    DocumentCrop {
        width: u32,
        height: u32,
    },
    TogglePanel {
        panel_id: String,
    },
    ApplyWorkspacePreset {
        preset_id: String,
    },
    SetLocks {
        pixels: bool,
        position: bool,
        all: bool,
        alpha: bool,
    },
    MaskAttributes {
        enabled: bool,
        linked: bool,
        density: f32,
        feather: f32,
        inverted: bool,
        contrast: f32,
        shift: f32,
    },
    SoftProof {
        profile: String,
        intent: String,
    },
    /// `None` clears embedded ICC; `Some` validates and embeds.
    SetIcc {
        bytes: Option<Vec<u8>>,
    },
    HistoryJump {
        entry_id: u64,
    },
    /// Clockwise quarter turns of the canvas: 1 is 90° CW, 2 is 180°, 3 is
    /// 90° CCW. Counted rather than signed so one command covers the three
    /// entries Photoshop's Image Rotation submenu carries.
    Rotate {
        quarter_turns: u32,
    },
    /// New pixel dimensions for Image Size.
    Resize {
        width: u32,
        height: u32,
    },
}

/// Host-side follow-up for undo/redo entries that own GPU or selection stacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostHistoryAction {
    Undo(HistoryKind),
    Redo(HistoryKind),
}

/// Effects the UI host should apply after a successful command.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandEffects {
    pub recomposite: bool,
    pub dirty: bool,
    pub sync_layers: bool,
    pub sync_camera: bool,
    pub sync_doc: bool,
    pub sync_selection: bool,
    pub host_history: Option<HostHistoryAction>,
    pub host_follow_up: HostFollowUp,
    pub created_layer: Option<LayerId>,
    /// Document generation after the command (0 if no document).
    pub generation: u64,
}

impl CommandEffects {
    // `pub(super)` rather than `pub`: the router builds these, nothing outside
    // the command module should. Widening exactly four constructors is the
    // whole cost of moving the vocabulary out — the eighty-four command bodies
    // stayed put precisely to avoid paying it eighty-four times.
    pub(super) fn view_only() -> Self {
        Self {
            recomposite: false,
            dirty: false,
            sync_layers: false,
            sync_camera: true,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation: 0,
        }
    }

    pub(super) fn host_chrome(follow_up: HostFollowUp) -> Self {
        Self {
            recomposite: false,
            dirty: false,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: false,
            host_history: None,
            host_follow_up: follow_up,
            created_layer: None,
            generation: 0,
        }
    }

    pub(super) fn document_edit(generation: u64) -> Self {
        Self {
            recomposite: true,
            dirty: true,
            sync_layers: true,
            sync_camera: false,
            sync_doc: true,
            sync_selection: false,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        }
    }

    pub(super) fn selection_edit(generation: u64) -> Self {
        Self {
            recomposite: false,
            dirty: true,
            sync_layers: false,
            sync_camera: false,
            sync_doc: false,
            sync_selection: true,
            host_history: None,
            host_follow_up: HostFollowUp::None,
            created_layer: None,
            generation,
        }
    }
}

/// Typed command failures.
///
/// The two halves are different kinds of event and belong on different paths.
/// A [`Self::Rejected`] or [`Self::InvalidArgument`] is the command declining
/// and saying why — something the person at the keyboard can act on, and the
/// reason strings are written for them. A [`Self::Unknown`] command, or a
/// document invariant that did not hold, is a wiring fault with nothing useful
/// to tell a user.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CommandError {
    #[error("unknown command `{0}`")]
    Unknown(String),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("command rejected: {0}")]
    Rejected(&'static str),
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
}

impl CommandError {
    /// Whether this is something the user can do something about.
    ///
    /// A *variant* test, not a substring test. The host used to classify these
    /// by searching the rendered `Display` text for the word "rejected" — after
    /// calling `to_string()` on the very value that already knew the answer.
    /// That also mis-routed anything else whose message happened to contain the
    /// word, and driver messages do use it.
    #[must_use]
    pub fn is_user_correctable(&self) -> bool {
        match self {
            Self::Rejected(_) | Self::InvalidArgument(_) => true,
            Self::Document(
                DocumentError::NoDocument
                | DocumentError::LayerLimitReached { .. }
                // The user typed the number; typing a smaller one fixes it.
                | DocumentError::DimensionTooLarge { .. },
            ) => true,
            Self::Unknown(_) | Self::Document(DocumentError::LayerMissingAfterAdd) => false,
        }
    }

    /// The sentence to put in front of a person.
    ///
    /// The reason strings are the message — there is no second table mapping
    /// internal reasons to friendly ones, because a second table is a second
    /// vocabulary and it would drift. What this adds is presentation: a capital
    /// and a full stop, and none of the `command rejected:` scaffolding that
    /// belongs in a log rather than a status bar.
    #[must_use]
    pub fn user_message(&self) -> String {
        let reason = match self {
            Self::Rejected(reason) | Self::InvalidArgument(reason) => (*reason).to_owned(),
            other => other.to_string(),
        };
        let mut chars = reason.chars();
        let mut out = match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => return String::new(),
        };
        if !out.ends_with(['.', '!', '?']) {
            out.push('.');
        }
        out
    }
}
