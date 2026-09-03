import QtQuick
import QtQuick.Controls
import phototux_ui

/// Single-line text entry drawn from `Theme` tokens.
///
/// Same reason as its siblings: the Basic style draws a white field with black
/// text, which is unreadable chrome in a dark panel and was reaching the user
/// in eight places — the fill and text colours, the shape hex fields, the
/// swatches hex field.
///
/// Sunken rather than raised, matching `ThemedSpinBox`: a field you type into
/// reads as a well, and that is what distinguishes it from a button at a
/// glance in a dense panel.
TextField {
    id: control

    /// The value this field is a *view* of. Bind this, never `text`.
    ///
    /// Qt drops a `TextField`'s `text` binding the moment the user types into
    /// it, and nothing puts it back. Every field in this shell that displays a
    /// document value was therefore one rejected keystroke from showing
    /// something the document does not have: typing `notacolour` into the
    /// swatches hex and pressing Return left `notacolour` on screen for the
    /// rest of the session while the swatch beside it never moved. Undo is the
    /// other way in — Ctrl+Z inside a focused field is the field's own undo.
    ///
    /// `Qt.binding` looks like the fix and was not reliable here: with a
    /// conditional source the field kept showing the wrong half of a pair.
    /// A plain property cannot lose its binding, and writing `text` from its
    /// change handler has no state to reason about. Leave `source` unset and
    /// the field owns its own contents, which is right for a field nothing
    /// else writes.
    property string source
    onSourceChanged: control.text = control.source
    Component.onCompleted: if (control.source.length > 0) control.text = control.source

    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontBodySm
    color: control.enabled ? Theme.colorOnSurfaceEffective : Theme.colorOnSurfaceDisabled
    placeholderTextColor: Theme.colorOnSurfaceMuted
    selectionColor: Theme.primary
    selectedTextColor: Theme.primaryOn
    selectByMouse: true
    leftPadding: Theme.spaceSm
    rightPadding: Theme.spaceSm

    background: Rectangle {
        implicitWidth: 96
        implicitHeight: Theme.controlHeight
        color: control.enabled ? Theme.surfaceSunken : Theme.surfaceContainer
        radius: Theme.radiusSm
        border.width: control.activeFocus ? 2 : 1
        border.color: control.activeFocus ? Theme.focusRing : Theme.borderEffective
    }
}
