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

    component HeaderIconButton: ToolButton {
        id: btn
        implicitWidth: 22
        implicitHeight: 22
        padding: 0
        icon.width: 12
        icon.height: 12
        contentItem: ThemedIcon {
            anchors.centerIn: parent
            source: btn.icon.source
            size: 12
            color: btn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
        }
        background: Rectangle {
            radius: Theme.radiusXs
            color: btn.hovered && btn.enabled ? Theme.surfaceContainerHigh : "transparent"
        }
    }

    HeaderIconButton {
        enabled: root.canMoveUp
        icon.source: Theme.iconUrl(AppSession.iconRoot, "caret-up")
        Accessible.name: qsTr("Move panel up")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.moveUpRequested()
    }
    HeaderIconButton {
        enabled: root.canMoveDown
        icon.source: Theme.iconUrl(AppSession.iconRoot, "caret-down")
        Accessible.name: qsTr("Move panel down")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.moveDownRequested()
    }
    HeaderIconButton {
        icon.source: Theme.iconUrl(AppSession.iconRoot, "minus")
        Accessible.name: qsTr("Auto-hide panel")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.autoHideRequested()
    }
    HeaderIconButton {
        enabled: root.canTearOff
        icon.source: Theme.iconUrl(AppSession.iconRoot, "arrow-square-out")
        Accessible.name: qsTr("Tear off panel")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.tearOffRequested()
    }
}
