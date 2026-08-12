import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Dense panel-header chrome — Phosphor icons only, uniform 16×16 box.
RowLayout {
    id: root
    spacing: 0

    property bool canMoveUp: true
    property bool canMoveDown: true
    property bool canTearOff: true
    /// Panels whose body is disclosure groups (Properties) offer expand/collapse all.
    property bool showsDisclosureToggle: false
    /// Drives which direction the toggle offers, per handbook 05 panel-local view actions.
    property bool anyGroupExpanded: true

    signal moveUpRequested()
    signal moveDownRequested()
    signal autoHideRequested()
    signal tearOffRequested()
    signal disclosureToggleRequested()

    component HeaderIconButton: ToolButton {
        id: btn

        implicitWidth: Theme.panelHeaderBtn
        implicitHeight: Theme.panelHeaderBtn
        padding: 0
        leftPadding: 0
        rightPadding: 0
        topPadding: 0
        bottomPadding: 0
        display: AbstractButton.IconOnly
        icon.width: Theme.iconMd
        icon.height: Theme.iconMd

        contentItem: Item {
            implicitWidth: Theme.iconMd
            implicitHeight: Theme.iconMd
            ThemedIcon {
                anchors.centerIn: parent
                source: btn.icon.source
                size: Theme.iconMd
                color: btn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
            }
        }
        background: Rectangle {
            radius: Theme.radiusXs
            color: btn.hovered && btn.enabled ? Theme.surfaceContainerHigh : "transparent"
        }
    }

    HeaderIconButton {
        visible: root.showsDisclosureToggle
        icon.source: Theme.iconUrl(AppSession.iconRoot,
                                   root.anyGroupExpanded ? "arrows-in-line-vertical"
                                                         : "arrows-out-line-vertical")
        Accessible.name: root.anyGroupExpanded ? qsTr("Collapse all groups")
                                               : qsTr("Expand all groups")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.disclosureToggleRequested()
    }
    HeaderIconButton {
        enabled: root.canMoveUp
        icon.source: Theme.iconUrl(AppSession.iconRoot, "arrow-up")
        Accessible.name: qsTr("Move panel up")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.moveUpRequested()
    }
    HeaderIconButton {
        enabled: root.canMoveDown
        icon.source: Theme.iconUrl(AppSession.iconRoot, "arrow-down")
        Accessible.name: qsTr("Move panel down")
        ToolTip.visible: hovered
        ToolTip.text: Accessible.name
        onClicked: root.moveDownRequested()
    }
    HeaderIconButton {
        icon.source: Theme.iconUrl(AppSession.iconRoot, "minus-square")
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
