import QtQuick
import QtQuick.Controls
import phototux_ui

/// A tooltip drawn from `Theme.qml` rather than by the Controls style.
///
/// The shell was using the attached form — `ToolTip.visible: hovered` with
/// `ToolTip.text` — at forty call sites. That drives the **shared** tool tip
/// instance, and the shared instance is built by the Controls style, which is
/// Basic, which hardcodes a light palette. A dark editor was popping pale grey
/// tips over its own chrome, and nothing at the call site said so.
///
/// The shared instance cannot be restyled from one place: assigning to
/// `ToolTip.toolTip.background` is accepted and has no effect, from an Item or
/// from the window. So each site declares one of these instead, which is a
/// popup it owns and this file styles.
///
/// Usage — `visible` reads the hover state of whatever it is declared inside:
///
///     ThemedButton {
///         Accessible.name: qsTr("Move panel up")
///         ThemedToolTip { visible: parent.hovered; text: parent.Accessible.name }
///     }
ToolTip {
    id: control

    /// Long enough not to fire while the pointer crosses a toolbar, short
    /// enough to feel like an answer. Qt's own default is 0, which turns a
    /// row of icon buttons into a flicker of popups.
    delay: 450
    timeout: 8000
    padding: Theme.spaceSm
    /// Tool tips are chrome about chrome: the control they describe already
    /// carries the same string as its `Accessible.name`, so reading it twice
    /// is noise.
    Accessible.ignored: true

    background: Rectangle {
        color: Theme.surfaceRaised
        border.color: Theme.borderEffective
        border.width: 1
        radius: Theme.radiusSm
    }

    contentItem: Text {
        text: control.text
        color: Theme.colorOnSurfaceEffective
        font.pixelSize: Theme.fontBodySm
        wrapMode: Text.WordWrap
        // A tip that runs the width of the window is a paragraph, not a tip.
        width: Math.min(implicitWidth, 320)
    }
}
