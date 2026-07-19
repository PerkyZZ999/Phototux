import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Dense panel-header chrome: reorder / auto-hide / tear-off (Phosphor glyphs).
RowLayout {
    id: root
    spacing: 0

    property bool canMoveUp: true
    property bool canMoveDown: true
    property bool canTearOff: true

    signal moveUpRequested()
    signal moveDownRequested()
    signal autoHideRequested()
    signal tearOffRequested()

    ToolButton {
        implicitWidth: 22
        implicitHeight: 22
        padding: 0
        enabled: root.canMoveUp
        icon.source: Theme.iconUrl(AppSession.iconRoot, "caret-up")
        icon.width: 12
        icon.height: 12
        Accessible.name: qsTr("Move panel up")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.moveUpRequested()
    }
    ToolButton {
        implicitWidth: 22
        implicitHeight: 22
        padding: 0
        enabled: root.canMoveDown
        icon.source: Theme.iconUrl(AppSession.iconRoot, "caret-down")
        icon.width: 12
        icon.height: 12
        Accessible.name: qsTr("Move panel down")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.moveDownRequested()
    }
    ToolButton {
        implicitWidth: 22
        implicitHeight: 22
        padding: 0
        icon.source: Theme.iconUrl(AppSession.iconRoot, "minus")
        icon.width: 12
        icon.height: 12
        Accessible.name: qsTr("Auto-hide panel")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.autoHideRequested()
    }
    ToolButton {
        implicitWidth: 22
        implicitHeight: 22
        padding: 0
        enabled: root.canTearOff
        icon.source: Theme.iconUrl(AppSession.iconRoot, "arrow-square-out")
        icon.width: 12
        icon.height: 12
        Accessible.name: qsTr("Tear off panel")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.tearOffRequested()
    }
}
