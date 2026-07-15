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
    color: "#1E1E22"

    // DESIGN.md tokens
    readonly property color primary: "#3DAEE9"
    readonly property color surface: "#2B2B30"
    readonly property color surfaceRaised: "#323238"
    readonly property color surfaceSunken: "#121214"
    readonly property color surfaceOverlay: "#232328"
    readonly property color border: "#3D3D45"
    readonly property color onSurface: "#EFF0F1"
    readonly property color onSurfaceMuted: "#A0A0A8"
    readonly property color canvasLetterbox: "#0C0C0E"
    readonly property color toolActiveBg: "#3DAEE940"
    readonly property int toolStripWidth: 48
    readonly property int dockWidth: 280
    readonly property int statusHeight: 28

    function iconUrl(stem) {
        var root = AppSession.iconRoot
        if (!root || root.length === 0)
            return ""
        // Qt file URL
        if (root.charAt(0) === "/")
            return "file://" + root + "/" + stem + ".svg"
        return "file:///" + root + "/" + stem + ".svg"
    }

    Component.onCompleted: {
        // First launch: require explicit New Document (ADR-013)
        if (!AppSession.hasDocument)
            newDocDialog.open()
    }

    // —— Top chrome ——
    header: ToolBar {
        height: 40
        background: Rectangle {
            color: root.surface
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: root.border
            }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            spacing: 4

            Label {
                text: "PhotoTux"
                color: root.primary
                font.bold: true
                font.pixelSize: 13
            }

            ToolSeparator {
                contentItem: Rectangle { implicitWidth: 1; color: root.border }
            }

            ToolButton {
                text: qsTr("New")
                onClicked: newDocDialog.open()
            }

            ToolButton {
                text: qsTr("Open")
                enabled: false
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Portals — Phase 5")
            }

            ToolButton {
                text: qsTr("Save")
                enabled: false
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Portals — Phase 5")
            }

            Item { Layout.fillWidth: true }

            Label {
                text: qsTr("Phase 1 shell")
                color: root.onSurfaceMuted
                font.pixelSize: 11
            }
        }
    }

    footer: ToolBar {
        height: root.statusHeight
        background: Rectangle {
            color: root.surface
            Rectangle {
                anchors.top: parent.top
                width: parent.width
                height: 1
                color: root.border
            }
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 16

            Label {
                text: AppSession.statusText
                color: root.onSurface
                font.pixelSize: 11
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            Label {
                text: AppSession.hasDocument
                      ? (Math.round(AppSession.zoom * 100) + "% · "
                         + AppSession.docWidth + "×" + AppSession.docHeight)
                      : ""
                color: root.onSurfaceMuted
                font.pixelSize: 11
            }

            Label {
                text: qsTr("FPS: —")
                color: root.onSurfaceMuted
                font.pixelSize: 11
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Left tool strip
        Rectangle {
            Layout.preferredWidth: root.toolStripWidth
            Layout.fillHeight: true
            color: root.surface

            Rectangle {
                anchors.right: parent.right
                width: 1
                height: parent.height
                color: root.border
            }

            Column {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: 8
                spacing: 4

                Repeater {
                    model: [
                        { id: "tool.brush", stem: "paint-brush", tip: qsTr("Brush") },
                        { id: "tool.pan", stem: "hand", tip: qsTr("Pan") },
                        { id: "tool.zoom", stem: "magnifying-glass", tip: qsTr("Zoom") }
                    ]
                    delegate: Rectangle {
                        width: 36
                        height: 36
                        radius: 4
                        color: AppSession.activeTool === modelData.id ? root.toolActiveBg : "transparent"
                        border.color: AppSession.activeTool === modelData.id ? root.primary : "transparent"
                        border.width: 1

                        Image {
                            anchors.centerIn: parent
                            source: root.iconUrl(modelData.stem)
                            width: 20
                            height: 20
                            sourceSize: Qt.size(20, 20)
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: AppSession.setActiveTool(modelData.id)
                            ToolTip.visible: containsMouse
                            ToolTip.text: modelData.tip
                            ToolTip.delay: 400
                        }
                    }
                }
            }
        }

        // Canvas placeholder
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Rectangle {
                anchors.fill: parent
                color: root.canvasLetterbox

                // Checkerboard
                Canvas {
                    id: checker
                    anchors.fill: parent
                    opacity: 0.12
                    onPaint: {
                        var ctx = getContext("2d")
                        var s = 16
                        for (var y = 0; y < height; y += s) {
                            for (var x = 0; x < width; x += s) {
                                ctx.fillStyle = ((x / s + y / s) % 2 === 0) ? "#444" : "#222"
                                ctx.fillRect(x, y, s, s)
                            }
                        }
                    }
                    Component.onCompleted: requestPaint()
                    onWidthChanged: requestPaint()
                    onHeightChanged: requestPaint()
                }

                // Document frame (aspect from session; zoom scales visual)
                Rectangle {
                    id: docFrame
                    visible: AppSession.hasDocument
                    anchors.centerIn: parent
                    readonly property real aspect: AppSession.docHeight > 0
                        ? AppSession.docWidth / AppSession.docHeight : 16 / 9
                    readonly property real baseW: Math.min(parent.width * 0.7, 720)
                    width: baseW * AppSession.zoom
                    height: (baseW / aspect) * AppSession.zoom
                    color: root.surfaceOverlay
                    border.color: root.border
                    border.width: 1
                    radius: 2

                    Label {
                        anchors.centerIn: parent
                        text: qsTr("Canvas viewport\n(wgpu · Phase 2)")
                        color: root.onSurfaceMuted
                        horizontalAlignment: Text.AlignHCenter
                        font.pixelSize: 13
                    }
                }

                Label {
                    visible: !AppSession.hasDocument
                    anchors.centerIn: parent
                    text: qsTr("No document — File → New")
                    color: root.onSurfaceMuted
                    font.pixelSize: 14
                }
            }
        }

        // Right docks
        Rectangle {
            Layout.preferredWidth: root.dockWidth
            Layout.fillHeight: true
            color: root.surface

            Rectangle {
                anchors.left: parent.left
                width: 1
                height: parent.height
                color: root.border
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 12

                Label {
                    text: qsTr("Properties")
                    color: root.onSurface
                    font.bold: true
                    font.pixelSize: 12
                }

                Label {
                    text: qsTr("Brush size")
                    color: root.onSurfaceMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    Slider {
                        id: brushSlider
                        Layout.fillWidth: true
                        from: 1
                        to: 200
                        value: AppSession.brushSize
                        enabled: AppSession.hasDocument
                        onMoved: AppSession.setBrushSize(value)
                    }
                    Label {
                        text: Math.round(brushSlider.value) + "px"
                        color: root.onSurface
                        font.pixelSize: 11
                        Layout.preferredWidth: 40
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Label {
                    text: qsTr("Zoom")
                    color: root.onSurfaceMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    Slider {
                        id: zoomSlider
                        Layout.fillWidth: true
                        from: 0.1
                        to: 4.0
                        value: AppSession.zoom
                        enabled: AppSession.hasDocument
                        onMoved: AppSession.setZoom(value)
                    }
                    Label {
                        text: Math.round(zoomSlider.value * 100) + "%"
                        color: root.onSurface
                        font.pixelSize: 11
                        Layout.preferredWidth: 40
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Button {
                    text: qsTr("Reset zoom")
                    Layout.fillWidth: true
                    enabled: AppSession.hasDocument
                    onClicked: {
                        AppSession.setZoom(1.0)
                        zoomSlider.value = 1.0
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: root.border
                }

                RowLayout {
                    Layout.fillWidth: true
                    Label {
                        text: qsTr("Layers")
                        color: root.onSurface
                        font.bold: true
                        font.pixelSize: 12
                        Layout.fillWidth: true
                    }
                    Label {
                        text: qsTr("stub")
                        color: root.onSurfaceMuted
                        font.pixelSize: 10
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 120
                    color: root.surfaceOverlay
                    border.color: root.border
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
                                radius: 3
                                color: index === 1 ? root.toolActiveBg : "transparent"
                                Label {
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.left: parent.left
                                    anchors.leftMargin: 8
                                    text: modelData
                                    color: root.onSurface
                                    font.pixelSize: 11
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                Label {
                    text: qsTr("Rust ↔ QML live")
                    color: root.primary
                    font.pixelSize: 10
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }
    }

    NewDocumentDialog {
        id: newDocDialog
        anchors.centerIn: parent
        onAccepted: function (presetLabel, w, h) {
            if (presetLabel && presetLabel.length > 0)
                AppSession.applySizePreset(presetLabel)
            else
                AppSession.applyDocumentSize(w, h)
            zoomSlider.value = AppSession.zoom
            brushSlider.value = AppSession.brushSize
        }
    }
}
