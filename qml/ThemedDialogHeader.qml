import QtQuick
import QtQuick.Controls

/// Title bar for the shell's inline `Dialog`s.
///
/// The Basic style's default dialog header draws its label in a near-black
/// intended for a light window, which on our dark surfaces left titles barely
/// legible. Dialogs that already build their own chrome — `NewDocumentDialog`,
/// `WelcomeDialog` — do not need this; it exists for the ones declared inline
/// in `Main.qml` that only set `title`.
Rectangle {
    id: root

    /// Title text. Callers pass the dialog's own `title`.
    required property string text

    implicitHeight: Theme.toolbarHeight
    color: Theme.surfaceRaised
    // Only the top corners are rounded, matching the dialog background.
    radius: Theme.radiusMd

    // Square off the bottom edge the radius above would otherwise round.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: parent.radius
        color: parent.color
    }

    Label {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceMd
        anchors.rightMargin: Theme.spaceMd
        text: root.text
        color: Theme.colorOnSurfaceEffective
        font.pixelSize: Theme.fontHeadlineSm
        font.weight: Font.DemiBold
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.borderEffective
    }
}
