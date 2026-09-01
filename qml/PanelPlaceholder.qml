import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// What a dock panel shows when it has nothing to list.
///
/// A panel with an empty model used to render an empty sunken rectangle, which
/// says nothing: a user cannot tell "there is nothing here yet" from "this
/// panel is broken" or "I am looking at the wrong panel". The line names what
/// *will* fill it, so the empty state carries the same information scent as a
/// full one (handbook 01/28).
///
/// It is deliberately quiet — muted text, a dimmed glyph, no border. An empty
/// panel is a normal state, not a warning, and a placeholder loud enough to
/// compete with the canvas would be worse than the blank rectangle it replaces.
Item {
    id: root

    /// Phosphor icon stem for the glyph above the text.
    required property string iconKey
    /// Resolve an icon stem to a URL; passed rather than reached for, matching
    /// the other dock components.
    required property var iconUrl
    /// One line naming what would be here.
    required property string text
    /// Optional second line saying how to get there.
    property string hint: ""

    Accessible.role: Accessible.StaticText
    Accessible.name: root.hint.length > 0 ? root.text + ". " + root.hint : root.text

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - Theme.spaceLg * 2, 220)
        spacing: Theme.spaceXs

        ThemedIcon {
            Layout.alignment: Qt.AlignHCenter
            source: root.iconUrl(root.iconKey)
            size: Theme.iconMd * 2
            // Dimmer than disabled chrome: this is scenery, not a control.
            color: Theme.iconDisabledEffective
            opacity: 0.5
        }
        Label {
            Layout.fillWidth: true
            text: root.text
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontBodySm
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true
            visible: root.hint.length > 0
            text: root.hint
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabelSm
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            opacity: 0.8
        }
    }
}
