import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Tab strip for one dock group, drawn in the group's panel header.
///
/// A group of one renders as a plain title, so a single-panel group is
/// indistinguishable from the pre-grouping header — grouping is a layout
/// decision, not a visual style the user has to read.
///
/// Selection is shown by weight, colour **and** an accent underline. Handbook
/// 28 requires state that is not carried by colour alone, and the underline is
/// also what makes the active tab legible in the high-contrast pack.
RowLayout {
    id: root

    /// Panel id whose group this strip represents.
    required property string panelId
    /// Visible tabs of the group, in stack order.
    required property var tabs

    spacing: Theme.spaceXxs

    readonly property bool single: root.tabs.length <= 1

    // Single-tab group: a title, exactly as before grouping existed.
    Label {
        visible: root.single
        text: root.single ? qsTr(root.panelTitle(root.panelId)) : ""
        color: Theme.colorOnSurfaceVariant
        font.pixelSize: Theme.fontLabel
        font.weight: Font.Medium
        elide: Text.ElideRight
        Layout.fillWidth: true
    }

    Repeater {
        model: root.single ? [] : root.tabs
        delegate: AbstractButton {
            id: tab
            required property string modelData

            readonly property bool current: root.activeTab === tab.modelData

            implicitHeight: Theme.panelHeaderHeight
            implicitWidth: tabLabel.implicitWidth + Theme.spaceSm * 2
            focusPolicy: Qt.StrongFocus
            onClicked: AppSession.raisePanelTab(tab.modelData)

            Accessible.role: Accessible.PageTab
            Accessible.name: qsTr(root.panelTitle(tab.modelData))
            // Selection reaches assistive tech without relying on the accent.
            Accessible.description: tab.current ? qsTr("Selected") : qsTr("Not selected")

            background: Rectangle {
                color: tab.current
                       ? Theme.surfaceContainerHigh
                       : (tab.hovered ? Theme.surfaceContainer : "transparent")
                radius: Theme.radiusXs
                border.width: tab.activeFocus ? 1 : 0
                border.color: Theme.focusRing

                // Non-colour selection marker.
                Rectangle {
                    visible: tab.current
                    anchors.bottom: parent.bottom
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.width - Theme.spaceXs
                    height: 2
                    radius: 1
                    color: Theme.primary
                }
            }

            contentItem: Label {
                id: tabLabel
                text: qsTr(root.panelTitle(tab.modelData))
                color: tab.current ? Theme.colorOnSurfaceEffective : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabel
                font.weight: tab.current ? Font.DemiBold : Font.Normal
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
        }
    }

    Item {
        visible: !root.single
        Layout.fillWidth: true
    }

    /// Which tab the group is showing. Read from the host projection so the
    /// strip and the body agree on one answer.
    readonly property string activeTab: {
        var groups = root.dockGroupsSource
        for (var g = 0; g < groups.length; ++g) {
            var list = groups[g].tabs || []
            for (var t = 0; t < list.length; ++t) {
                if (list[t] === root.panelId)
                    return groups[g].active
            }
        }
        return root.panelId
    }

    readonly property var dockGroupsSource: {
        try {
            return JSON.parse(AppSession.dockGroupsJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Descriptor title for a panel id.
    function panelTitle(id) {
        try {
            var list = JSON.parse(AppSession.panelDescriptorsJson || "[]")
            for (var i = 0; i < list.length; ++i) {
                if (list[i].id === id)
                    return list[i].title
            }
        } catch (e) {
            // fall through
        }
        return id
    }
}
