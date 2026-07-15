import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui
import PhototuxCanvas 1.0

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
    // Avoid names starting with "on" — QML reserves on* for signal handlers.
    readonly property color colorOnSurface: "#EFF0F1"
    readonly property color colorOnSurfaceMuted: "#A0A0A8"
    readonly property color canvasLetterbox: "#0C0C0E"
    readonly property color toolActiveBg: "#3DAEE940"
    readonly property int toolStripWidth: 48
    readonly property int dockWidth: 280
    readonly property int statusHeight: 28

    function iconUrl(stem) {
        var root = AppSession.iconRoot
        if (!root || root.length === 0)
            return ""
        if (root.charAt(0) === "/")
            return "file://" + root + "/" + stem + ".svg"
        return "file:///" + root + "/" + stem + ".svg"
    }

    Component.onCompleted: {
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

            ToolSeparator {
                contentItem: Rectangle { implicitWidth: 1; color: root.border }
            }

            ToolButton {
                text: qsTr("Undo")
                enabled: AppSession.canUndo
                onClicked: AppSession.undo()
            }
            ToolButton {
                text: qsTr("Redo")
                enabled: AppSession.canRedo
                onClicked: AppSession.redo()
            }

            Item { Layout.fillWidth: true }

            Label {
                text: qsTr("Phase 4 · brush")
                color: root.colorOnSurfaceMuted
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
                color: root.colorOnSurface
                font.pixelSize: 11
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            Label {
                text: AppSession.hasDocument
                      ? (Math.round(AppSession.zoom * 100) + "% · "
                         + AppSession.docWidth + "×" + AppSession.docHeight)
                      : ""
                color: root.colorOnSurfaceMuted
                font.pixelSize: 11
            }

            Label {
                text: AppSession.compositeMs > 0
                      ? (qsTr("comp ") + AppSession.compositeMs.toFixed(2) + " ms")
                      : ""
                color: AppSession.compositeMs > 0 && AppSession.compositeMs < 2.0
                       ? root.primary : root.colorOnSurfaceMuted
                font.pixelSize: 11
            }

            Label {
                text: AppSession.strokeLatencyMs > 0
                      ? (qsTr("lat ") + Math.round(AppSession.strokeLatencyMs) + " ms")
                      : ""
                color: AppSession.strokeLatencyMs > 0 && AppSession.strokeLatencyMs < 8
                       ? root.primary : root.colorOnSurfaceMuted
                font.pixelSize: 11
            }

            Label {
                id: fpsLabel
                text: AppSession.fps > 0
                      ? (qsTr("FPS: ") + Math.round(AppSession.fps))
                      : qsTr("FPS: —")
                color: AppSession.fps >= 60 ? root.primary : root.colorOnSurfaceMuted
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
                        { id: "tool.eraser", stem: "eraser", tip: qsTr("Eraser") },
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

        // GPU canvas viewport
        Item {
            id: canvasHost
            Layout.fillWidth: true
            Layout.fillHeight: true

            onWidthChanged: AppSession.setViewportSize(width, height)
            onHeightChanged: AppSession.setViewportSize(width, height)
            Component.onCompleted: AppSession.setViewportSize(width, height)

            PhototuxCanvas {
                id: gpuCanvas
                anchors.fill: parent
                zoom: AppSession.zoom
                panX: AppSession.panX
                panY: AppSession.panY
                docWidth: AppSession.docWidth
                docHeight: AppSession.docHeight
                hasDocument: AppSession.hasDocument
                phase: frameClock.phase + AppSession.graphRevision * 0.01
            }

            // Brush size cursor (visual guide)
            Rectangle {
                id: brushCursor
                visible: AppSession.hasDocument
                         && (AppSession.activeTool === "tool.brush"
                             || AppSession.activeTool === "tool.eraser")
                         && canvasInput.containsMouse
                width: Math.max(4, AppSession.brushSize * AppSession.zoom)
                height: width
                radius: width / 2
                color: "transparent"
                border.color: AppSession.activeTool === "tool.eraser"
                              ? "#E06060" : root.primary
                border.width: 1
                x: canvasInput.mouseX - width / 2
                y: canvasInput.mouseY - height / 2
                z: 3
            }

            Label {
                visible: !AppSession.hasDocument
                anchors.centerIn: parent
                z: 2
                text: qsTr("No document — File → New")
                color: root.colorOnSurfaceMuted
                font.pixelSize: 14
            }

            // Continuous phase + FPS + worker poll
            FrameAnimation {
                id: frameClock
                running: root.visible
                property real phase: 0
                property real fpsEma: 0
                onTriggered: {
                    phase = (phase + frameTime * 2.0) % 1000.0
                    AppSession.pollEngine()
                    if (frameTime > 0) {
                        var inst = 1.0 / frameTime
                        fpsEma = fpsEma > 0 ? (fpsEma * 0.9 + inst * 0.1) : inst
                        AppSession.reportFps(fpsEma)
                    }
                }
            }

            // Pointer: brush / eraser / pan / zoom / wheel
            MouseArea {
                id: canvasInput
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                hoverEnabled: true
                cursorShape: {
                    if (AppSession.activeTool === "tool.pan")
                        return Qt.OpenHandCursor
                    if (AppSession.activeTool === "tool.zoom")
                        return Qt.CrossCursor
                    if (AppSession.activeTool === "tool.brush"
                            || AppSession.activeTool === "tool.eraser")
                        return Qt.BlankCursor
                    return Qt.ArrowCursor
                }
                property real lastX: 0
                property real lastY: 0
                property bool dragging: false
                property bool painting: false

                onPressed: function (mouse) {
                    lastX = mouse.x
                    lastY = mouse.y
                    dragging = true
                    if (mouse.button === Qt.MiddleButton
                            || AppSession.activeTool === "tool.pan") {
                        cursorShape = Qt.ClosedHandCursor
                        painting = false
                        return
                    }
                    if (AppSession.activeTool === "tool.brush"
                            || AppSession.activeTool === "tool.eraser") {
                        painting = true
                        // pressure: tablet via mouse.pressure if available, else 1
                        var p = (typeof mouse.pressure === "number" && mouse.pressure > 0)
                                ? mouse.pressure : 1.0
                        AppSession.strokeBegin(mouse.x, mouse.y, p)
                    }
                }
                onReleased: function (mouse) {
                    if (painting) {
                        AppSession.strokeEnd()
                        painting = false
                    }
                    dragging = false
                    if (AppSession.activeTool === "tool.pan")
                        cursorShape = Qt.OpenHandCursor
                }
                onPositionChanged: function (mouse) {
                    if (!dragging || !AppSession.hasDocument)
                        return
                    var dx = mouse.x - lastX
                    var dy = mouse.y - lastY
                    lastX = mouse.x
                    lastY = mouse.y
                    var panMode = AppSession.activeTool === "tool.pan"
                                  || (mouse.buttons & Qt.MiddleButton)
                    if (panMode) {
                        AppSession.panBy(dx, dy)
                    } else if (AppSession.activeTool === "tool.zoom") {
                        var factor = Math.exp(-dy * 0.01)
                        AppSession.zoomAt(factor, mouse.x, mouse.y)
                        zoomSlider.value = AppSession.zoom
                    } else if (painting) {
                        var p = (typeof mouse.pressure === "number" && mouse.pressure > 0)
                                ? mouse.pressure : 1.0
                        AppSession.strokeMove(mouse.x, mouse.y, p)
                    }
                }
                onWheel: function (wheel) {
                    if (!AppSession.hasDocument)
                        return
                    var steps = wheel.angleDelta.y / 120.0
                    var factor = Math.pow(1.12, steps)
                    AppSession.zoomAt(factor, wheel.x, wheel.y)
                    zoomSlider.value = AppSession.zoom
                    wheel.accepted = true
                }
                onDoubleClicked: function (mouse) {
                    if (AppSession.hasDocument) {
                        AppSession.zoomToFit()
                        zoomSlider.value = AppSession.zoom
                    }
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
                    color: root.colorOnSurface
                    font.bold: true
                    font.pixelSize: 12
                }

                Label {
                    text: qsTr("Brush size")
                    color: root.colorOnSurfaceMuted
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
                        color: root.colorOnSurface
                        font.pixelSize: 11
                        Layout.preferredWidth: 40
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Label {
                    text: qsTr("Hardness")
                    color: root.colorOnSurfaceMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    Slider {
                        id: hardnessSlider
                        Layout.fillWidth: true
                        from: 0
                        to: 1
                        value: AppSession.brushHardness
                        enabled: AppSession.hasDocument
                        onMoved: AppSession.setBrushHardness(value)
                    }
                    Label {
                        text: Math.round(hardnessSlider.value * 100) + "%"
                        color: root.colorOnSurface
                        font.pixelSize: 11
                        Layout.preferredWidth: 40
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Label {
                    text: qsTr("Color")
                    color: root.colorOnSurfaceMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 4
                    Rectangle {
                        width: 28
                        height: 28
                        radius: 4
                        color: Qt.rgba(AppSession.brushR, AppSession.brushG, AppSession.brushB, 1)
                        border.color: root.border
                    }
                    Slider {
                        id: colorR
                        Layout.fillWidth: true
                        from: 0; to: 1
                        value: AppSession.brushR
                        enabled: AppSession.hasDocument
                        onMoved: AppSession.setBrushColor(value, colorG.value, colorB.value)
                    }
                }
                Slider {
                    id: colorG
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: AppSession.brushG
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setBrushColor(colorR.value, value, colorB.value)
                }
                Slider {
                    id: colorB
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: AppSession.brushB
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setBrushColor(colorR.value, colorG.value, value)
                }

                Label {
                    text: qsTr("Zoom")
                    color: root.colorOnSurfaceMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    Slider {
                        id: zoomSlider
                        Layout.fillWidth: true
                        from: 0.05
                        to: 8.0
                        value: AppSession.zoom
                        enabled: AppSession.hasDocument
                        onMoved: AppSession.setZoom(value)
                    }
                    Label {
                        text: Math.round(zoomSlider.value * 100) + "%"
                        color: root.colorOnSurface
                        font.pixelSize: 11
                        Layout.preferredWidth: 48
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Button {
                    text: qsTr("Fit to view")
                    Layout.fillWidth: true
                    enabled: AppSession.hasDocument
                    onClicked: {
                        AppSession.zoomToFit()
                        zoomSlider.value = AppSession.zoom
                    }
                }

                Button {
                    text: qsTr("Reset zoom 100%")
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

                Label {
                    text: qsTr("Layer opacity")
                    color: root.colorOnSurfaceMuted
                    font.pixelSize: 11
                }

                RowLayout {
                    Layout.fillWidth: true
                    Slider {
                        id: layerOpacitySlider
                        Layout.fillWidth: true
                        from: 0
                        to: 1
                        value: AppSession.activeOpacity
                        enabled: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
                        onMoved: AppSession.setActiveOpacity(value)
                    }
                    Label {
                        text: Math.round(layerOpacitySlider.value * 100) + "%"
                        color: root.colorOnSurface
                        font.pixelSize: 11
                        Layout.preferredWidth: 40
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Label {
                    text: qsTr("GPU")
                    color: root.colorOnSurface
                    font.bold: true
                    font.pixelSize: 12
                }

                Label {
                    text: gpuCanvas.gpuStatus
                    color: root.colorOnSurfaceMuted
                    font.pixelSize: 10
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Label {
                    text: AppSession.compositeMs > 0
                          ? (qsTr("Composite: ") + AppSession.compositeMs.toFixed(2) + " ms")
                          : qsTr("Composite: —")
                    color: root.colorOnSurfaceMuted
                    font.pixelSize: 10
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
                        color: root.colorOnSurface
                        font.bold: true
                        font.pixelSize: 12
                        Layout.fillWidth: true
                    }
                    Label {
                        text: AppSession.layerCount
                        color: root.colorOnSurfaceMuted
                        font.pixelSize: 10
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Button {
                        text: qsTr("Add")
                        Layout.fillWidth: true
                        enabled: AppSession.hasDocument
                        onClicked: AppSession.addLayer()
                    }
                    Button {
                        text: qsTr("Del")
                        Layout.fillWidth: true
                        enabled: AppSession.hasDocument && AppSession.layerCount > 1
                        onClicked: AppSession.deleteActiveLayer()
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 160
                    color: root.surfaceOverlay
                    border.color: root.border
                    radius: 4

                    // Top of stack drawn at top of list (reverse index).
                    ListView {
                        id: layerList
                        anchors.fill: parent
                        anchors.margins: 6
                        clip: true
                        spacing: 2
                        model: AppSession.layerCount
                        delegate: Rectangle {
                            width: layerList.width
                            height: 28
                            radius: 3
                            // stack bottom = index 0; UI shows top first
                            readonly property int stackIndex: AppSession.layerCount - 1 - index
                            readonly property var nameParts: AppSession.layerNames.split("|")
                            readonly property var visParts: AppSession.layerVisibility.split("|")
                            readonly property string layerName: stackIndex >= 0 && stackIndex < nameParts.length
                                ? nameParts[stackIndex] : ""
                            readonly property bool layerVis: stackIndex >= 0 && stackIndex < visParts.length
                                ? visParts[stackIndex] === "1" : true
                            color: AppSession.activeLayerIndex === stackIndex
                                   ? root.toolActiveBg : "transparent"
                            border.color: AppSession.activeLayerIndex === stackIndex
                                          ? root.primary : "transparent"
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 6
                                anchors.rightMargin: 6
                                spacing: 6

                                Image {
                                    source: root.iconUrl(layerVis ? "eye" : "eye-slash")
                                    width: 16
                                    height: 16
                                    sourceSize: Qt.size(16, 16)
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: AppSession.toggleLayerVisible(stackIndex)
                                    }
                                }

                                Label {
                                    Layout.fillWidth: true
                                    text: layerName
                                    color: root.colorOnSurface
                                    font.pixelSize: 11
                                    elide: Text.ElideRight
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                anchors.leftMargin: 28
                                onClicked: AppSession.setActiveLayer(stackIndex)
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                Label {
                    text: qsTr("composite · undo")
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
            AppSession.setViewportSize(canvasHost.width, canvasHost.height)
            if (presetLabel && presetLabel.length > 0)
                AppSession.applySizePreset(presetLabel)
            else
                AppSession.applyDocumentSize(w, h)
            zoomSlider.value = AppSession.zoom
            brushSlider.value = AppSession.brushSize
            layerOpacitySlider.value = AppSession.activeOpacity
        }
    }

    Connections {
        target: AppSession
        function onActiveOpacityChanged() {
            if (Math.abs(layerOpacitySlider.value - AppSession.activeOpacity) > 0.001)
                layerOpacitySlider.value = AppSession.activeOpacity
        }
    }

}
