import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Dense panel-header chrome — Phosphor icons only, uniform 16×16 box.
RowLayout {
    id: root
    spacing: 0

    /// Which panel this header belongs to. Every call site used to restate it
    /// four times over — once per enablement expression and once per dispatch —
    /// and the enablement rule was written out in full for each panel.
    required property string panelId
    /// This panel's position in the right dock stack, and the stack's length.
    /// The three enablement rules below are derived from them here rather than
    /// at each header.
    required property int stackRow
    required property int stackLength

    readonly property bool canMoveUp: stackRow > 0
    readonly property bool canMoveDown: stackRow >= 0 && stackRow < stackLength - 1
    readonly property bool canTearOff: stackLength > 1

    /// Panels whose body is disclosure groups (Properties) offer expand/collapse all.
    property bool showsDisclosureToggle: false
    /// Drives which direction the toggle offers, per handbook 05 panel-local view actions.
    property bool anyGroupExpanded: true

    /// Tear-off stays a signal because its placement differs per panel; the
    /// shell decides where a floated panel lands. Moving and auto-hiding are
    /// the same action everywhere, so they are performed here.
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
            // Every panel header carries four or five of these, all icon-only,
            // and they are in the tab chain. Without this a keyboard user
            // tabbing into the dock sees nothing at all move.
            border.color: btn.visualFocus ? Theme.focusRing : "transparent"
            border.width: 1
        }
    }

    HeaderIconButton {
        visible: root.showsDisclosureToggle
        icon.source: Theme.iconUrl(AppSession.iconRoot,
                                   root.anyGroupExpanded ? "arrows-in-line-vertical"
                                                         : "arrows-out-line-vertical")
        Accessible.name: root.anyGroupExpanded ? qsTr("Collapse all groups")
                                               : qsTr("Expand all groups")
        ThemedToolTip {
            visible: parent.hovered
            text: parent.Accessible.name
        }
        onClicked: root.disclosureToggleRequested()
    }
    HeaderIconButton {
        enabled: root.canMoveUp
        icon.source: Theme.iconUrl(AppSession.iconRoot, "arrow-up")
        Accessible.name: qsTr("Move panel up")
        ThemedToolTip {
            visible: parent.hovered
            text: parent.Accessible.name
        }
        onClicked: AppSession.movePanelInStack(root.panelId, -1)
    }
    HeaderIconButton {
        enabled: root.canMoveDown
        icon.source: Theme.iconUrl(AppSession.iconRoot, "arrow-down")
        Accessible.name: qsTr("Move panel down")
        ThemedToolTip {
            visible: parent.hovered
            text: parent.Accessible.name
        }
        onClicked: AppSession.movePanelInStack(root.panelId, 1)
    }
    HeaderIconButton {
        icon.source: Theme.iconUrl(AppSession.iconRoot, "minus-square")
        Accessible.name: qsTr("Auto-hide panel")
        ThemedToolTip {
            visible: parent.hovered
            text: parent.Accessible.name
        }
        onClicked: AppSession.togglePanelAutoHide(root.panelId)
    }
    HeaderIconButton {
        enabled: root.canTearOff
        icon.source: Theme.iconUrl(AppSession.iconRoot, "arrow-square-out")
        Accessible.name: qsTr("Tear off panel")
        ThemedToolTip {
            visible: parent.hovered
            text: parent.Accessible.name
        }
        onClicked: root.tearOffRequested()
    }
}
