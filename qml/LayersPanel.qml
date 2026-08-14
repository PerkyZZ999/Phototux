// The layers panel's list.
//
// Extracted from the shell after the panel gained a real model: with rows
// arriving as model roles rather than six pipe-joined strings re-indexed per
// delegate, what was left had a seam narrow enough to be worth cutting — two
// values in, one signal out, for about a hundred and ninety lines.
//
// The dock placement stays behind. `Layout.row`, visibility and the sunken
// background are the dock's arrangement of its panels, not something this list
// should know about.
//
// Note what is *not* here: no binding reads `AppSession`. A model's row change
// reaches its view synchronously, inside the host slot that made it, so a
// delegate binding that reads the host re-enters a borrowed session and aborts
// the process. Per-row values come from roles; anything else comes through a
// property on the shell. The `AppSession` calls below are all click handlers,
// which the event loop delivers outside any slot. Handbook 32 — Item models
// are the synchronous case.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

ListView {
    id: root

    /// Resolve an icon stem to a URL. Passed rather than reached for, because
    /// the shell's helper reads the asset root from the host and a delegate
    /// binding must not.
    required property var iconUrl

    /// Whether the mask edit target is active, for the mask chip's highlight.
    required property bool maskEditActive

    /// Raised on right-click or the context button. The menu belongs to the
    /// shell, so this reports where and for which layer rather than opening it.
    signal contextMenuRequested(int stackIndex, var origin, real localX, real localY)

    clip: true
    spacing: 0
    reuseItems: true
    cacheBuffer: Theme.toolHit * 4
    model: AppSession.layerModel
    delegate: Rectangle {
        width: root.width
        height: 36

        // Roles from AppSession.layerModel. Names are the
        // model item's field names verbatim — the derive
        // does not camel-case them — so they read as
        // snake_case here and nowhere else in this file.
        required property string name
        required property string kind
        required property bool layer_visible
        required property int mask_flag
        required property bool clips_to_below
        required property bool selected
        required property bool active
        // Index in the engine's bottom→top order, for the
        // commands that take one. Rows arrive already in
        // display order, so nothing here inverts an index.
        required property int stack_index

        readonly property bool hasMask: mask_flag !== 0
        readonly property bool maskEnabled: mask_flag === 1
        color: active || selected ? Theme.surfaceRaised
               : (layerHover.hovered ? Theme.surfaceContainer : "transparent")
        border.color: active ? Theme.primary
                      : (selected ? Theme.primaryHover : "transparent")
        border.width: (active || selected) ? 1 : 0

        ToolButton {
            id: layerVisButton
            z: 3
            anchors.left: parent.left
            anchors.leftMargin: Theme.spaceXs
            anchors.verticalCenter: parent.verticalCenter
            implicitWidth: 22
            implicitHeight: 22
            flat: true
            icon.source: root.iconUrl(layer_visible ? "eye" : "eye-slash")
            icon.width: 16
            icon.height: 16
            contentItem: ThemedIcon {
                anchors.centerIn: parent
                source: layerVisButton.icon.source
                size: 16
                color: Theme.iconOnSurfaceEffective
            }
            background: Rectangle {
                radius: Theme.radiusXs
                color: layerVisButton.hovered ? Theme.surfaceContainerHigh : "transparent"
            }
            onClicked: AppSession.toggleLayerVisible(stack_index)
            Accessible.name: layer_visible
                             ? qsTr("Hide %1").arg(name)
                             : qsTr("Show %1").arg(name)
            ToolTip.visible: hovered
            ToolTip.text: Accessible.name
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 28
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceSm

            // Visibility control is layerVisButton (above) so it stays above the row MouseArea.

            Rectangle {
                width: 24
                height: 24
                radius: Theme.radiusXs
                color: Theme.surface
                border.color: Theme.border
                Label {
                    anchors.centerIn: parent
                    text: kind === "group" ? "G"
                          : (kind === "text" ? "T"
                          : (kind === "adjustment" ? "A"
                          : (kind === "fill" ? "F"
                          : (kind === "shape" ? "S" : ""))))
                    color: Theme.colorOnSurfaceVariant
                    font.pixelSize: 9
                }
            }

            Rectangle {
                visible: hasMask
                Layout.preferredWidth: 24
                Layout.preferredHeight: 24
                radius: Theme.radiusXs
                color: maskEnabled ? Theme.surfaceRaised : Theme.surfaceSunken
                border.color: root.maskEditActive && active
                              ? Theme.primary : Theme.border
                border.width: root.maskEditActive && active ? 2 : 1
                z: 2
                Label {
                    anchors.centerIn: parent
                    text: qsTr("M")
                    color: maskEnabled ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                    font.weight: Font.DemiBold
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        AppSession.setActiveLayer(stack_index)
                        AppSession.setMaskEditTarget(true)
                    }
                    ToolTip.visible: containsMouse
                    ToolTip.text: qsTr("Edit layer mask")
                }
            }

            ThemedIcon {
                visible: hasMask
                source: root.iconUrl(maskEnabled ? "eye" : "eye-slash")
                size: 14
                color: maskEnabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                opacity: maskEnabled ? 1.0 : 0.85
                z: 2
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        AppSession.setActiveLayer(stack_index)
                        AppSession.setMaskEnabledOnActive(!maskEnabled)
                    }
                    ToolTip.visible: containsMouse
                    ToolTip.text: maskEnabled
                                  ? qsTr("Disable layer mask")
                                  : qsTr("Enable layer mask")
                }
            }

            ThemedIcon {
                visible: clips_to_below
                source: root.iconUrl("arrow-elbow-down-right")
                size: 14
                color: Theme.primary
                Accessible.name: qsTr("Clipped to layer below")
                ToolTip.visible: clipHover.hovered
                ToolTip.text: qsTr("Clipped to layer below — delete base releases clip")
                HoverHandler { id: clipHover }
            }

            Label {
                Layout.fillWidth: true
                text: name
                color: active ? Theme.colorOnSurface : Theme.colorOnSurfaceVariant
                font.pixelSize: Theme.fontBodySm
                elide: Text.ElideRight
            }
        }

        HoverHandler { id: layerHover }
        MouseArea {
            anchors.fill: parent
            anchors.leftMargin: 48
            z: -1
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: function (mouse) {
                var ctrl = (mouse.modifiers & Qt.ControlModifier)
                           || (mouse.modifiers & Qt.MetaModifier)
                var shift = !!(mouse.modifiers & Qt.ShiftModifier)
                AppSession.selectLayerClick(stack_index, ctrl, shift)
                if (mouse.button === Qt.RightButton) {
                    root.contextMenuRequested(stack_index, this, mouse.x, mouse.y)
                }
            }
            onPressAndHold: {
                root.contextMenuRequested(stack_index, this, width / 2, height / 2)
            }
        }
    }
}
