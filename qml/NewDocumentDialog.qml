import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

Popup {
    id: dialog
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape
    // Scaled with density. A dialog laid out for one type size and pinned
    // to a pixel width crowds its own content at the other: at Comfortable
    // the New Document preset cards ran "Recommended" and "1920 x 1080"
    // right into the card border.
    width: Math.round(720 * Theme.densityScale)
    height: 480
    padding: 0

    signal createRequested(string presetLabel, int width, int height)

    property string selectedPreset: "1080p"
    property int customW: 1920
    property int customH: 1080

    readonly property var presets: [
        { label: "720p",  sub: "1280 × 720",  w: 1280, h: 720,  tip: qsTr("HD ready") },
        { label: "1080p", sub: "1920 × 1080", w: 1920, h: 1080, tip: qsTr("Recommended") },
        { label: "2K",    sub: "2560 × 1440", w: 2560, h: 1440, tip: qsTr("QHD") },
        { label: "4K",    sub: "3840 × 2160", w: 3840, h: 2160, tip: qsTr("UHD") }
    ]

    function selectPreset(label) {
        selectedPreset = label
        for (var i = 0; i < presets.length; ++i) {
            if (presets[i].label === label) {
                customW = presets[i].w
                customH = presets[i].h
                spinW.value = presets[i].w
                spinH.value = presets[i].h
                break
            }
        }
    }

    function syncCustomFromSpins() {
        customW = spinW.value
        customH = spinH.value
        var match = ""
        for (var i = 0; i < presets.length; ++i) {
            if (presets[i].w === spinW.value && presets[i].h === spinH.value) {
                match = presets[i].label
                break
            }
        }
        selectedPreset = match
    }

    function confirmCreate() {
        // Always honor the spin values the user sees. Preset label is only used when
        // width/height still match that preset (avoids stale selection after edits).
        syncCustomFromSpins()
        if (dialog.selectedPreset && dialog.selectedPreset.length > 0)
            dialog.createRequested(dialog.selectedPreset, 0, 0)
        else
            dialog.createRequested("", spinW.value, spinH.value)
        dialog.close()
    }

    onOpened: createButton.forceActiveFocus()

    Keys.onPressed: function (event) {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            dialog.confirmCreate()
            event.accepted = true
        }
    }

    background: Rectangle {
        color: Theme.surface
        border.color: Theme.border
        radius: Theme.radiusLg
    }

    Overlay.modal: Rectangle {
        color: Theme.scrimModal
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.toolbarHeight
            color: Theme.surface

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: Theme.border
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceMd
                anchors.rightMargin: Theme.spaceSm
                spacing: Theme.spaceSm

                Label {
                    text: qsTr("New Document")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontWindow
                    font.weight: Font.DemiBold
                }

                Item { Layout.fillWidth: true }

                ToolButton {
                    id: closeNewDocBtn
                    implicitWidth: 28
                    implicitHeight: 28
                    icon.source: Theme.iconUrl(AppSession.iconRoot, "x")
                    icon.width: 16
                    icon.height: 16
                    contentItem: ThemedIcon {
                        anchors.centerIn: parent
                        source: closeNewDocBtn.icon.source
                        size: 16
                        color: Theme.iconOnSurfaceEffective
                    }
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: closeNewDocBtn.hovered ? Theme.surfaceContainerHigh : "transparent"
                        border.color: closeNewDocBtn.visualFocus ? Theme.focusRing : "transparent"
                        border.width: 1
                    }
                    onClicked: dialog.close()
                    ThemedToolTip {
                        visible: parent.hovered
                        text: qsTr("Close")
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // Preset grid
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceXl
                        spacing: Theme.spaceMd

                        Label {
                            text: qsTr("Blank document presets")
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontHeadlineSm
                            font.weight: Font.DemiBold
                        }

                        Label {
                            text: qsTr("Choose a size. 1080p suits most work; every preset is editable on the right.")
                            color: Theme.colorOnSurfaceMuted
                            font.pixelSize: Theme.fontBodySm
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        GridLayout {
                            Layout.fillWidth: true
                            columns: 4
                            columnSpacing: Theme.spaceMd
                            rowSpacing: Theme.spaceMd

                            Repeater {
                                model: dialog.presets
                                delegate: Rectangle {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 148
                                    radius: Theme.radiusMd
                                    color: dialog.selectedPreset === modelData.label
                                           ? Theme.surfaceRaised : Theme.surface
                                    border.color: dialog.selectedPreset === modelData.label
                                                  ? Theme.primary : Theme.border
                                    border.width: dialog.selectedPreset === modelData.label ? 1 : 1

                                    Rectangle {
                                        visible: dialog.selectedPreset === modelData.label
                                        anchors.top: parent.top
                                        anchors.right: parent.right
                                        anchors.margins: Theme.spaceSm
                                        width: 8
                                        height: 8
                                        radius: 4
                                        color: Theme.primary
                                    }

                                    ColumnLayout {
                                        anchors.centerIn: parent
                                        spacing: Theme.spaceSm
                                        width: parent.width - Theme.spaceLg

                                        Rectangle {
                                            Layout.alignment: Qt.AlignHCenter
                                            implicitWidth: Math.max(36, Math.min(72, modelData.w / modelData.h * 48))
                                            implicitHeight: Math.max(28, Math.min(56, modelData.h / modelData.w * 48))
                                            color: Theme.surfaceSunken
                                            border.color: dialog.selectedPreset === modelData.label
                                                          ? Theme.primary : Theme.border
                                            radius: Theme.radiusXs

                                            ThemedIcon {
                                                anchors.centerIn: parent
                                                source: Theme.iconUrl(AppSession.iconRoot, "monitor")
                                                size: 18
                                                color: dialog.selectedPreset === modelData.label
                                                       ? Theme.iconOnSurfaceEffective
                                                       : Theme.iconDisabledEffective
                                            }
                                        }

                                        Label {
                                            Layout.alignment: Qt.AlignHCenter
                                            text: modelData.label
                                            color: Theme.colorOnSurface
                                            font.pixelSize: Theme.fontLabel
                                            font.weight: Font.DemiBold
                                        }

                                        Label {
                                            Layout.alignment: Qt.AlignHCenter
                                            text: modelData.sub
                                            color: Theme.colorOnSurfaceMuted
                                            font.pixelSize: Theme.fontBodySm
                                        }

                                        Label {
                                            Layout.alignment: Qt.AlignHCenter
                                            text: modelData.tip
                                            color: dialog.selectedPreset === modelData.label
                                                   ? Theme.primary : Theme.colorOnSurfaceDisabled
                                            font.pixelSize: Theme.fontLabelSm
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: dialog.selectPreset(modelData.label)
                                    }
                                }
                            }
                        }

                        Item { Layout.fillHeight: true }
                    }
                }
            }

            // Details dock
            Rectangle {
                Layout.preferredWidth: 260
                Layout.fillHeight: true
                color: Theme.surface

                Rectangle {
                    anchors.left: parent.left
                    width: 1
                    height: parent.height
                    color: Theme.border
                }

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 0

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Theme.toolbarHeight
                        color: Theme.surface

                        Rectangle {
                            anchors.bottom: parent.bottom
                            width: parent.width
                            height: 1
                            color: Theme.border
                        }

                        Label {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.spaceMd
                            text: qsTr("Preset Details")
                            color: Theme.colorOnSurface
                            font.pixelSize: Theme.fontHeadlineSm
                            font.weight: Font.DemiBold
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.margins: Theme.spaceMd
                        spacing: Theme.spaceMd

                        Label {
                            text: qsTr("Width")
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabelSm
                        }

                        ThemedSpinBox {
                            id: spinW
                            Layout.fillWidth: true
                            from: 1
                            to: AppSession.maxDocumentDimension
                            editable: true
                            Accessible.name: qsTr("Document width")
                            // Avoid `value: dialog.customW` binding — it fights editable typing.
                            Component.onCompleted: value = dialog.customW
                            onValueModified: dialog.syncCustomFromSpins()
                        }

                        Label {
                            text: qsTr("Height")
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabelSm
                        }

                        ThemedSpinBox {
                            id: spinH
                            Layout.fillWidth: true
                            from: 1
                            to: AppSession.maxDocumentDimension
                            editable: true
                            Accessible.name: qsTr("Document height")
                            Component.onCompleted: value = dialog.customH
                            onValueModified: dialog.syncCustomFromSpins()
                        }

                        Label {
                            text: qsTr("Pixels · RGB document")
                            color: Theme.colorOnSurfaceMuted
                            font.pixelSize: Theme.fontBodySm
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        Item { Layout.fillHeight: true }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceSm

                            ThemedButton {
                                Layout.fillWidth: true
                                text: qsTr("Cancel")
                                onClicked: dialog.close()
                            }

                            ThemedButton {
                                id: createButton
                                Layout.fillWidth: true
                                text: qsTr("Create")
                                prominence: "primary"
                                focus: true
                                onClicked: dialog.confirmCreate()
                                Keys.onReturnPressed: dialog.confirmCreate()
                                Keys.onEnterPressed: dialog.confirmCreate()
                            }
                        }
                    }
                }
            }
        }
    }

    Component.onCompleted: selectPreset("1080p")
}
