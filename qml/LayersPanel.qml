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
        id: layerRow
        width: root.width
        height: 36

        // Roles from AppSession.layerModel. Names are the
        // model item's field names verbatim — the derive
        // does not camel-case them — so they read as
        // snake_case here and nowhere else in this file.
        required property string name
        required property string kind_badge
        required property string kind_label
        required property bool layer_visible
        required property int mask_flag
        required property bool clips_to_below
        required property bool selected
        required property bool active
        // Index in the engine's bottom→top order, for the
        // commands that take one. Rows arrive already in
        // display order, so nothing here inverts an index.
        required property int stack_index
        // Groups this layer is inside. Rows are a flat list —
        // a group is a parent, not a container — so without
        // this a grouped stack and an ungrouped one draw
        // identically.
        required property int depth
        // Whether a group above this row is hidden. The
        // layer's own flag can still say visible while it
        // contributes nothing, and a row that drew its eye
        // open would describe an image that is not there.
        required property bool hidden_by_group

        // Photoshop indents a group's contents and leaves the
        // visibility toggle in its own fixed column, so the
        // eyes stay in one line down the panel however deep
        // the nesting runs. Only what follows them moves.
        readonly property int indent: depth * Theme.spaceMd
        // What the canvas actually shows for this layer.
        readonly property bool onCanvas: layer_visible && !hidden_by_group
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
                // Dimmed rather than switched to the crossed eye: the layer's
                // own flag is still on, and clicking here still toggles it.
                // What is off is the group above, which the user turns back on
                // there.
                color: layerRow.onCanvas || !layer_visible
                       ? Theme.iconOnSurfaceEffective
                       : Theme.iconDisabledEffective
            }
            background: Rectangle {
                radius: Theme.radiusXs
                color: layerVisButton.hovered ? Theme.surfaceContainerHigh : "transparent"
            }
            onClicked: AppSession.toggleLayerVisible(stack_index)
            Accessible.name: hidden_by_group
                             ? qsTr("%1 — hidden with its group").arg(name)
                             : (layer_visible
                                ? qsTr("Hide %1").arg(name)
                                : qsTr("Show %1").arg(name))
            ThemedToolTip {
                visible: parent.hovered
                text: parent.Accessible.name
            }
        }

        // One hairline per level of nesting, in the gap the indent opens.
        //
        // The indent on its own is twelve pixels of nothing, which at a single
        // level reads as a row that failed to line up rather than as a row
        // inside something. The rails make the same twelve pixels say which
        // group the row belongs to, and they stack when groups nest.
        Repeater {
            model: layerRow.depth
            delegate: Rectangle {
                required property int index
                x: 28 + (index * Theme.spaceMd) + Math.round(Theme.spaceMd / 2)
                width: 1
                height: layerRow.height
                color: Theme.borderEffective
                opacity: 0.35
            }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 28 + layerRow.indent
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceSm

            // Visibility control is layerVisButton (above) so it stays above the row MouseArea.

            // The kind marker, and only when there is a kind worth marking.
            //
            // Every row used to carry this square, empty for raster layers —
            // which is most of them — so the panel showed a bordered blank box
            // beside each ordinary layer that read as a thumbnail that had
            // failed to load. The letters also lived here, a second copy of the
            // layer vocabulary; they come from `LayerKind::badge` now, and the
            // slot keeps its width so the names below it stay aligned.
            Item {
                implicitWidth: 24
                implicitHeight: 24
                Rectangle {
                    anchors.fill: parent
                    visible: kind_badge.length > 0
                    radius: Theme.radiusXs
                    color: Theme.surface
                    border.color: Theme.border
                    Label {
                        anchors.centerIn: parent
                        text: kind_badge
                        color: Theme.colorOnSurfaceVariant
                        font.pixelSize: 9
                    }
                    Accessible.role: Accessible.StaticText
                    Accessible.name: kind_label
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
                    // `containsMouse` is false without this, so the tip
                    // below had never once appeared.
                    hoverEnabled: true
                    onClicked: {
                        AppSession.setActiveLayer(stack_index)
                        AppSession.setMaskEditTarget(true)
                    }
                    ThemedToolTip {
                        visible: parent.containsMouse
                        text: qsTr("Edit layer mask")
                    }
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
                    hoverEnabled: true
                    onClicked: {
                        AppSession.setActiveLayer(stack_index)
                        AppSession.setMaskEnabledOnActive(!maskEnabled)
                    }
                    ThemedToolTip {
                        visible: parent.containsMouse
                        text: maskEnabled ? qsTr("Disable layer mask") : qsTr("Enable layer mask")
                    }
                }
            }

            ThemedIcon {
                visible: clips_to_below
                source: root.iconUrl("arrow-elbow-down-right")
                size: 14
                color: Theme.primary
                Accessible.name: qsTr("Clipped to layer below")
                ThemedToolTip {
                    visible: clipHover.hovered
                    text: qsTr("Clipped to layer below — delete base releases clip")
                }
                HoverHandler { id: clipHover }
            }

            Label {
                Layout.fillWidth: true
                text: name
                color: layerRow.onCanvas
                       ? (active ? Theme.colorOnSurface : Theme.colorOnSurfaceVariant)
                       : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBodySm
                elide: Text.ElideRight
            }
        }

        HoverHandler { id: layerHover }
        MouseArea {
            anchors.fill: parent
            anchors.leftMargin: 48 + layerRow.indent
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
