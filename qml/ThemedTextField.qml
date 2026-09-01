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
