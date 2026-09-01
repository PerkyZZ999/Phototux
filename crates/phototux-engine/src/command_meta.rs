//! Slim command taxonomy metadata (handbook parity P1.7).
//!
//! Full ch.08 `CommandDescriptor` (schemas, jobs, capabilities) is deferred.
//! Every [`crate::command_id`] entry MUST have a row here.

use crate::command_id;

/// Where a command is authorized to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    Application,
    Workspace,
    Document,
    View,
    Selection,
}

/// What kind of state the command mutates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationClass {
    None,
    Ephemeral,
    Document,
    HistoryMeta,
    Workspace,
    Preference,
}

/// How the command participates in undo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoPolicy {
    None,
    Transaction,
    Groupable,
    Mergeable,
}

/// How concurrent / stale invocations are resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    ExactVersion,
    LatestWinsView,
    ExclusiveOp,
}

/// Taxonomy axes for a registered command id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMeta {
    pub id: &'static str,
    pub scope: CommandScope,
    pub mutation: MutationClass,
    pub undo: UndoPolicy,
    pub conflict: ConflictPolicy,
}

const fn meta(
    id: &'static str,
    scope: CommandScope,
    mutation: MutationClass,
    undo: UndoPolicy,
    conflict: ConflictPolicy,
) -> CommandMeta {
    CommandMeta {
        id,
        scope,
        mutation,
        undo,
        conflict,
    }
}

/// Document-mutating edit with history transaction (typical layer/pixel commit).
const fn doc_tx(id: &'static str) -> CommandMeta {
    meta(
        id,
        CommandScope::Document,
        MutationClass::Document,
        UndoPolicy::Transaction,
        ConflictPolicy::ExactVersion,
    )
}

/// Document-mutating edit that coalesces (opacity slider, filter params).
const fn doc_merge(id: &'static str) -> CommandMeta {
    meta(
        id,
        CommandScope::Document,
        MutationClass::Document,
        UndoPolicy::Mergeable,
        ConflictPolicy::ExactVersion,
    )
}

/// View / camera / tool chrome — not document dirty.
const fn view_ephemeral(id: &'static str) -> CommandMeta {
    meta(
        id,
        CommandScope::View,
        MutationClass::Ephemeral,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    )
}

/// Selection channel commit (document-scoped selection state + history).
const fn sel_tx(id: &'static str) -> CommandMeta {
    meta(
        id,
        CommandScope::Selection,
        MutationClass::Document,
        UndoPolicy::Transaction,
        ConflictPolicy::ExactVersion,
    )
}

/// Built-in metadata table — must cover [`command_id::ALL`].
pub const ALL: &[CommandMeta] = &[
    meta(
        command_id::HISTORY_JUMP,
        CommandScope::Document,
        MutationClass::HistoryMeta,
        UndoPolicy::None,
        ConflictPolicy::ExactVersion,
    ),
    meta(
        command_id::HISTORY_UNDO,
        CommandScope::Document,
        MutationClass::HistoryMeta,
        UndoPolicy::None,
        ConflictPolicy::ExclusiveOp,
    ),
    meta(
        command_id::HISTORY_REDO,
        CommandScope::Document,
        MutationClass::HistoryMeta,
        UndoPolicy::None,
        ConflictPolicy::ExclusiveOp,
    ),
    doc_tx(command_id::LAYER_CREATE),
    doc_tx(command_id::LAYER_CREATE_FILL),
    doc_tx(command_id::LAYER_SET_FILL_COLOR),
    doc_tx(command_id::LAYER_DELETE),
    meta(
        command_id::LAYER_SET_ACTIVE,
        CommandScope::Document,
        MutationClass::Ephemeral,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    ),
    doc_tx(command_id::LAYER_SET_VISIBILITY),
    doc_merge(command_id::LAYER_SET_OPACITY),
    doc_tx(command_id::LAYER_SET_BLEND),
    doc_tx(command_id::LAYER_REORDER),
    doc_tx(command_id::LAYER_GROUP),
    doc_tx(command_id::LAYER_UNGROUP),
    doc_tx(command_id::LAYER_SET_CLIP),
    doc_tx(command_id::LAYER_SET_LOCKS),
    view_ephemeral(command_id::VIEW_ZOOM_TO),
    view_ephemeral(command_id::VIEW_ZOOM_TO_FIT),
    view_ephemeral(command_id::VIEW_PAN_TO),
    view_ephemeral(command_id::VIEW_PAN_BY),
    view_ephemeral(command_id::VIEW_ZOOM_AT),
    view_ephemeral(command_id::VIEW_SET_TOOL),
    meta(
        command_id::DOCUMENT_NEW_PRESET,
        CommandScope::Document,
        MutationClass::Document,
        UndoPolicy::None,
        ConflictPolicy::ExclusiveOp,
    ),
    meta(
        command_id::DOCUMENT_NEW_SIZE,
        CommandScope::Document,
        MutationClass::Document,
        UndoPolicy::None,
        ConflictPolicy::ExclusiveOp,
    ),
    doc_tx(command_id::DOCUMENT_ASSIGN_PROFILE),
    doc_tx(command_id::DOCUMENT_CONVERT_PROFILE),
    meta(
        command_id::DOCUMENT_SET_SOFT_PROOF,
        CommandScope::Document,
        MutationClass::Ephemeral,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    ),
    doc_tx(command_id::DOCUMENT_SET_ICC),
    doc_tx(command_id::DOCUMENT_CROP),
    doc_tx(command_id::DOCUMENT_ROTATE_90),
    sel_tx(command_id::SELECTION_REPLACE),
    sel_tx(command_id::SELECTION_DESELECT),
    sel_tx(command_id::SELECTION_INVERT),
    sel_tx(command_id::SELECTION_SELECT_ALL),
    sel_tx(command_id::SELECTION_MODIFY),
    sel_tx(command_id::SELECTION_COLOR_SELECT),
    doc_tx(command_id::SELECTION_TO_MASK),
    sel_tx(command_id::MASK_TO_SELECTION),
    doc_tx(command_id::MASK_CREATE),
    doc_tx(command_id::MASK_DELETE),
    doc_tx(command_id::MASK_SET_ENABLED),
    doc_tx(command_id::MASK_SET_ATTRIBUTES),
    doc_tx(command_id::MASK_CREATE_VECTOR),
    doc_tx(command_id::MASK_APPLY),
    doc_tx(command_id::TEXT_CREATE),
    doc_merge(command_id::TEXT_SET_CONTENT),
    doc_tx(command_id::TEXT_BAKE),
    doc_tx(command_id::SHAPE_CREATE),
    doc_tx(command_id::SHAPE_RASTERIZE),
    doc_tx(command_id::SHAPE_BOOLEAN),
    doc_tx(command_id::FILTER_ADD_ADJUSTMENT),
    doc_merge(command_id::FILTER_SET_PARAMETERS),
    doc_tx(command_id::FILTER_ADD_EFFECT),
    doc_merge(command_id::FILTER_SET_GAUSSIAN_RADIUS),
    view_ephemeral(command_id::FILTER_PREVIEW),
    view_ephemeral(command_id::FILTER_SET_PREVIEW_PARAMS),
    doc_tx(command_id::FILTER_COMMIT),
    view_ephemeral(command_id::FILTER_CANCEL_PREVIEW),
    doc_tx(command_id::EFFECT_REORDER),
    doc_tx(command_id::EFFECT_SET_ENABLED),
    doc_tx(command_id::PATH_SET_CLOSED),
    doc_tx(command_id::PATH_MOVE_ANCHOR),
    doc_tx(command_id::PATH_ADD_ANCHOR),
    doc_tx(command_id::PATH_DELETE_ANCHOR),
    doc_tx(command_id::STYLE_ADD),
    doc_merge(command_id::STYLE_SET_PARAMS),
    doc_tx(command_id::STYLE_SET_COLOR),
    doc_tx(command_id::STYLE_SET_ENABLED),
    doc_tx(command_id::STYLE_REMOVE),
    doc_tx(command_id::CLIPBOARD_PASTE_LAYER),
    doc_tx(command_id::PATH_STROKE_TO_LAYER),
    doc_tx(command_id::RASTER_TRANSFORM_COMMIT),
    doc_tx(command_id::RASTER_FLIP),
    doc_tx(command_id::RASTER_FILL),
    doc_tx(command_id::RASTER_GRADIENT),
    meta(
        command_id::RASTER_PAINT_STROKE,
        CommandScope::Document,
        MutationClass::Document,
        UndoPolicy::Groupable,
        ConflictPolicy::ExactVersion,
    ),
    meta(
        command_id::APP_SHOW_PREFERENCES,
        CommandScope::Application,
        MutationClass::Preference,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    ),
    meta(
        command_id::APP_SHOW_FILTER_GALLERY,
        CommandScope::Application,
        MutationClass::Preference,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    ),
    meta(
        command_id::WORKSPACE_RESET,
        CommandScope::Workspace,
        MutationClass::Workspace,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    ),
    meta(
        command_id::WORKSPACE_TOGGLE_PANEL,
        CommandScope::Workspace,
        MutationClass::Workspace,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    ),
    meta(
        command_id::WORKSPACE_APPLY_PRESET,
        CommandScope::Workspace,
        MutationClass::Workspace,
        UndoPolicy::None,
        ConflictPolicy::LatestWinsView,
    ),
];

/// Look up taxonomy metadata for a registered command id.
pub fn meta_for(id: &str) -> Option<&'static CommandMeta> {
    ALL.iter().find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_command_id_has_meta() {
        for id in command_id::ALL {
            assert!(
                meta_for(id).is_some(),
                "missing CommandMeta for command_id {id}"
            );
        }
        assert_eq!(ALL.len(), command_id::ALL.len());
    }

    #[test]
    fn meta_ids_unique_and_match_table() {
        let mut seen = HashSet::new();
        for m in ALL {
            assert!(seen.insert(m.id), "duplicate CommandMeta id {}", m.id);
            assert!(
                command_id::ALL.contains(&m.id),
                "CommandMeta {} not in command_id::ALL",
                m.id
            );
        }
    }
}
