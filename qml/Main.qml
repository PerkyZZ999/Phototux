import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

ApplicationWindow {
    id: root
    visible: true
    width: 1440
    height: 900
    title: qsTr("PhotoTux")
    color: "#1e1e22"

    // Dense dark chrome — Breeze-inspired neutrals (ADR-002)
    readonly property color panelBg: "#2b2b30"
    readonly property color panelBorder: "#3d3d45"
    readonly property color accent: "#3daee9"
    readonly property color textPrimary: "#eff0f1"
    readonly property color textMuted: "#a0a0a8"
    readonly property color canvasBg: "#121214"

    Component.onCompleted: {
        // Sync defaults into singleton after construction
        if (AppSession.brushSize < 1)
            AppSession.setBrushSize(12)
        if (AppSession.zoom < 0.05)
            AppSession.setZoom(1)
    }

    header: ToolBar {
        id: topBar
        height: 40
        background: Rectangle {
            color: root.panelBg
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: root.panelBorder
            }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            spacing: 6

            Label {
                text: "PhotoTux"
                color: root.accent
                font.bold: true
                font.pixelSize: 13
            }

            ToolSeparator {
                contentItem: Rectangle {
                    implicitWidth: 1
                    color: root.panelBorder
                }
            }

            ToolButton {
                text: qsTr("File")
                flat: true
            }
            ToolButton {
                text: qsTr("Edit")
                flat: true
            }
            ToolButton {
                text: qsTr("View")
                flat: true
            }
            ToolButton {
                text: qsTr("Image")
                flat: true
            }

            Item { Layout.fillWidth: true }

            Label {
                text: qsTr("Phase 1 shell")
                color: root.textMuted
                font.pixelSize: 11
            }
        }
    }

    footer: ToolBar {
        id: statusBar
        height: 28
        background: Rectangle {
            color: root.panelBg
            Rectangle {
                anchors.top: parent.top
                width: parent.width
                height: 1
                color: root.panelBorder
            }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 16

            Label {
                text: AppSession.statusText
                color: root.textPrimary
                font.pixelSize: 11
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            Label {
                text: qsTr("FPS: —")
                color: root.textMuted
                font.pixelSize: 11
            }

            Label {
                text: qsTr("Vulkan / wgpu: pending Phase 2")
                color: root.textMuted
                font.pixelSize: 11
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Left tool strip
        Rectangle {
            Layout.preferredWidth: 48
            Layout.fillHeight: true
            color: root.panelBg

            Rectangle {
                anchors.right: parent.right
                width: 1
                height: parent.height
                color: root.panelBorder
            }

            Column {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: 8
                spacing: 4

                Repeater {
                    model: ["🖌", "☐", "✥", "🔍", "🎨"]
                    delegate: Rectangle {
                        width: 36
                        height: 36
                        radius: 4
                        color: index === 0 ? Qt.rgba(0.24, 0.68, 0.91, 0.25) : "transparent"
                        border.color: index === 0 ? root.accent : "transparent"
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: modelData
                            font.pixelSize: 16
                        }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: AppSession.setBrushSize(AppSession.brushSize)
                        }
                    }
                }
            }
        }

        // Center canvas placeholder (GPU in Phase 2)
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Rectangle {
                anchors.fill: parent
                color: root.canvasBg

                // Checkerboard hint for transparency mindset
                Canvas {
                    id: checker
                    anchors.fill: parent
                    opacity: 0.15
                    onPaint: {
                        const ctx = getContext("2d")
                        const s = 16
                        for (let y = 0; y < height; y += s) {
                            for (let x = 0; x < width; x += s) {
                                ctx.fillStyle = ((x / s + y / s) % 2 === 0) ? "#333" : "#222"
                                ctx.fillRect(x, y, s, s)
                            }
                        }
                    }
                    Component.onCompleted: requestPaint()
                    onWidthChanged: requestPaint()
                    onHeightChanged: requestPaint()
                }

                Rectangle {
                    anchors.centerIn: parent
                    width: Math.min(parent.width * 0.55, 640) * AppSession.zoom
                    height: Math.min(parent.height * 0.55, 360) * AppSession.zoom
                    color: "#2a2a32"
                    border.color: root.panelBorder
                    border.width: 1
                    radius: 2

                    Label {
                        anchors.centerIn: parent
                        text: qsTr("Canvas viewport\n(wgpu · Phase 2)")
                        color: root.textMuted
                        horizontalAlignment: Text.AlignHCenter
                        font.pixelSize: 14
                    }
                }
            }
        }

        // Right properties dock
        Rectangle {
            Layout.preferredWidth: 280
            Layout.fillHeight: true
            color: root.panelBg

            Rectangle {
                anchors.left: parent.left
                width: 1
                height: parent.height
                color: root.panelBorder
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 14

                Label {
                    text: qsTr("Properties")
                    color: root.textPrimary
                    font.bold: true
                    font.pixelSize: 12
                }

                Label {
                    text: qsTr("Brush size")
                    color: root.textMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Slider {
                        id: brushSlider
                        Layout.fillWidth: true
                        from: 1
                        to: 200
                        value: AppSession.brushSize > 0 ? AppSession.brushSize : 12
                        onMoved: AppSession.setBrushSize(value)
                    }

                    Label {
                        text: Math.round(brushSlider.value) + "px"
                        color: root.textPrimary
                        font.pixelSize: 11
                        Layout.preferredWidth: 40
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Label {
                    text: qsTr("Zoom")
                    color: root.textMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Slider {
                        id: zoomSlider
                        Layout.fillWidth: true
                        from: 0.1
                        to: 4.0
                        value: AppSession.zoom > 0 ? AppSession.zoom : 1.0
                        onMoved: AppSession.setZoom(value)
                    }

                    Label {
                        text: Math.round(zoomSlider.value * 100) + "%"
                        color: root.textPrimary
                        font.pixelSize: 11
                        Layout.preferredWidth: 40
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Button {
                    text: qsTr("Reset view")
                    Layout.fillWidth: true
                    onClicked: {
                        AppSession.resetView()
                        zoomSlider.value = 1.0
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: root.panelBorder
                }

                Label {
                    text: qsTr("Layers")
                    color: root.textPrimary
                    font.bold: true
                    font.pixelSize: 12
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 120
                    color: "#232328"
                    border.color: root.panelBorder
                    radius: 4

                    Column {
                        anchors.fill: parent
                        anchors.margins: 8
                        spacing: 4

                        Repeater {
                            model: ["Background", "Layer 1", "Layer 2"]
                            delegate: Rectangle {
                                width: parent.width
                                height: 28
                                color: index === 1 ? Qt.rgba(0.24, 0.68, 0.91, 0.15) : "transparent"
                                radius: 3

                                Label {
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.left: parent.left
                                    anchors.leftMargin: 8
                                    text: modelData
                                    color: root.textPrimary
                                    font.pixelSize: 11
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                Label {
                    text: qsTr("Rust ↔ QML binding live")
                    color: root.accent
                    font.pixelSize: 10
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }
    }
}
