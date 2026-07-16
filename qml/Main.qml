import QtCore
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtQuick.Shapes
import phototux_ui
import PhototuxCanvas 1.0

ApplicationWindow {
    id: root
    visible: true
    width: 1440
    height: 900
    title: AppSession.hasDocument
           ? (AppSession.dirty
              ? qsTr("%1* — PhotoTux").arg(AppSession.documentName)
              : qsTr("%1 — PhotoTux").arg(AppSession.documentName))
           : qsTr("PhotoTux")
    color: Theme.neutral
    property string pendingDestructiveAction: ""
    readonly property var layerMaskFlagParts: AppSession.layerMaskFlags.length > 0
                                             ? AppSession.layerMaskFlags.split("|") : []
    readonly property var layerClipParts: AppSession.layerClips.length > 0
                                         ? AppSession.layerClips.split("|") : []
    readonly property int activeMaskFlag: AppSession.activeLayerIndex >= 0
                                         && AppSession.activeLayerIndex < layerMaskFlagParts.length
                                         ? Number(layerMaskFlagParts[AppSession.activeLayerIndex]) : 0
    readonly property bool activeLayerHasMask: activeMaskFlag !== 0
    readonly property bool activeMaskEnabled: activeMaskFlag === 1
    readonly property bool activeLayerClips: AppSession.activeLayerIndex >= 0
                                            && AppSession.activeLayerIndex < layerClipParts.length
                                            && layerClipParts[AppSession.activeLayerIndex] === "1"

    readonly property int toolStripWidth: Theme.toolStripWidth
    readonly property int dockWidth: Theme.dockWidth
    readonly property int statusHeight: Theme.statusbarHeight
    readonly property color primary: Theme.primary
    readonly property color surface: Theme.surface
    readonly property color surfaceRaised: Theme.surfaceRaised
    readonly property color surfaceSunken: Theme.surfaceSunken
    readonly property color surfaceOverlay: Theme.surfaceOverlay
    readonly property color border: Theme.border

    function iconUrl(stem) {
        return Theme.iconUrl(AppSession.iconRoot, stem)
    }

    function docToScreenX(docX) {
        return canvasHost.width / 2 + (docX - AppSession.panX) * AppSession.zoom
    }
    function docToScreenY(docY) {
        return canvasHost.height / 2 + (docY - AppSession.panY) * AppSession.zoom
    }
    function screenToDocX(screenX) {
        return (screenX - canvasHost.width / 2) / Math.max(0.001, AppSession.zoom) + AppSession.panX
    }
    function screenToDocY(screenY) {
        return (screenY - canvasHost.height / 2) / Math.max(0.001, AppSession.zoom) + AppSession.panY
    }
    function selectionCombineFromModifiers(modifiers) {
        var shift = (modifiers & Qt.ShiftModifier) !== 0
        var alt = (modifiers & Qt.AltModifier) !== 0
        if (shift && alt)
            return "intersect"
        if (shift)
            return "add"
        if (alt)
            return "subtract"
        return AppSession.selectionCombine
    }
    function isSelectTool() {
        return AppSession.activeTool === "tool.select.rect"
                || AppSession.activeTool === "tool.select.ellipse"
    }
    function isCropTool() {
        return AppSession.activeTool === "tool.crop"
    }
    function isTransformTool() {
        return AppSession.activeTool === "tool.transform"
    }
    readonly property color colorOnSurface: Theme.colorOnSurface
    readonly property color colorOnSurfaceMuted: Theme.colorOnSurfaceMuted
    readonly property color warning: Theme.warning
    readonly property color canvasLetterbox: Theme.canvasLetterbox
    readonly property color toolActiveBg: Theme.toolActiveBg

    function executeDestructiveAction(action) {
        pendingDestructiveAction = ""
        if (action === "new") {
            newDocDialog.open()
        } else if (action === "open") {
            openFileDialog.open()
        } else if (action === "close") {
            AppSession.closeDocument()
        } else if (action === "quit") {
            Qt.quit()
        }
    }

    function requestDestructiveAction(action) {
        if (AppSession.dirty) {
            pendingDestructiveAction = action
            unsavedDialog.open()
            return
        }
        executeDestructiveAction(action)
    }

    function discardAndContinue() {
        var action = pendingDestructiveAction
        AppSession.acknowledgeDiscard()
        executeDestructiveAction(action)
    }

    onClosing: function (close) {
        if (AppSession.dirty) {
            close.accepted = false
            pendingDestructiveAction = "quit"
            unsavedDialog.open()
        }
    }

    Action {
        id: newAction
        text: qsTr("&New…")
        icon.source: root.iconUrl("file-plus")
        shortcut: "Ctrl+N"
        enabled: !AppSession.ioBusy
        onTriggered: root.requestDestructiveAction("new")
    }

    Action {
        id: openAction
        text: qsTr("&Open…")
        icon.source: root.iconUrl("folder-open")
        shortcut: "Ctrl+O"
        enabled: !AppSession.ioBusy
        onTriggered: root.requestDestructiveAction("open")
    }

    Action {
        id: saveAction
        text: qsTr("&Save")
        icon.source: root.iconUrl("floppy-disk")
        shortcut: "Ctrl+S"
        enabled: AppSession.hasDocument && !AppSession.ioBusy
        onTriggered: {
            if (AppSession.documentPath && AppSession.documentPath.length > 0)
                AppSession.saveDocument("")
            else
                saveFileDialog.open()
        }
    }

    Action {
        id: saveAsAction
        text: qsTr("Save &As…")
        shortcut: "Ctrl+Shift+S"
        enabled: AppSession.hasDocument && !AppSession.ioBusy
        onTriggered: saveFileDialog.open()
    }

    Action {
        id: exportAction
        text: qsTr("&Export…")
        icon.source: root.iconUrl("export")
        shortcut: "Ctrl+Shift+E"
        enabled: AppSession.hasDocument && !AppSession.ioBusy
        onTriggered: exportFileDialog.open()
    }

    Action {
        id: closeAction
        text: qsTr("&Close")
        icon.source: root.iconUrl("x")
        shortcut: "Ctrl+W"
        enabled: AppSession.hasDocument && !AppSession.ioBusy
        onTriggered: root.requestDestructiveAction("close")
    }

    Action {
        id: quitAction
        text: qsTr("&Quit")
        shortcut: "Ctrl+Q"
        onTriggered: root.requestDestructiveAction("quit")
    }

    Action {
        id: undoAction
        text: qsTr("&Undo")
        icon.source: root.iconUrl("arrow-counter-clockwise")
        shortcut: "Ctrl+Z"
        enabled: AppSession.canUndo && !AppSession.ioBusy
        onTriggered: AppSession.undo()
    }

    Action {
        id: redoAction
        text: qsTr("&Redo")
        icon.source: root.iconUrl("arrow-clockwise")
        shortcut: "Ctrl+Shift+Z"
        enabled: AppSession.canRedo && !AppSession.ioBusy
        onTriggered: AppSession.redo()
    }

    Action {
        id: zoomFitAction
        text: qsTr("Zoom to &Fit")
        icon.source: root.iconUrl("corners-in")
        shortcut: "Ctrl+Shift+J"
        enabled: AppSession.hasDocument
        onTriggered: AppSession.zoomToFit()
    }

    menuBar: MenuBar {
        Menu {
            title: qsTr("&File")
            MenuItem { action: newAction }
            MenuItem { action: openAction }
            MenuSeparator {}
            MenuItem { action: saveAction }
            MenuItem { action: saveAsAction }
            MenuItem { action: exportAction }
            MenuSeparator {}
            MenuItem { action: closeAction }
            MenuItem { action: quitAction }
        }
        Menu {
            title: qsTr("&Edit")
            MenuItem { action: undoAction }
            MenuItem { action: redoAction }
            MenuSeparator {}
            MenuItem {
                text: qsTr("Select &All")
                shortcut: "Ctrl+A"
                enabled: AppSession.hasDocument
                onTriggered: AppSession.selectAll()
            }
            MenuItem {
                text: qsTr("Deselect")
                shortcut: "Ctrl+D"
                enabled: AppSession.hasDocument && AppSession.selectionActive
                onTriggered: AppSession.selectNone()
            }
            MenuItem {
                text: qsTr("&Invert Selection")
                shortcut: "Ctrl+Shift+I"
                enabled: AppSession.hasDocument
                onTriggered: AppSession.invertSelection()
            }
            MenuItem {
                text: qsTr("&Copy")
                shortcut: "Ctrl+C"
                enabled: AppSession.hasDocument && AppSession.selectionActive
                onTriggered: AppSession.copySelection()
            }
            MenuItem {
                text: qsTr("Paste as New Layer")
                shortcut: "Ctrl+V"
                enabled: AppSession.hasDocument
                onTriggered: AppSession.pasteAsNewLayer()
            }
        }
        Menu {
            title: qsTr("&Image")
            MenuItem {
                text: qsTr("Flip &Horizontal")
                enabled: AppSession.hasDocument && !AppSession.ioBusy
                onTriggered: AppSession.flipActiveLayer(true)
            }
            MenuItem {
                text: qsTr("Flip &Vertical")
                enabled: AppSession.hasDocument && !AppSession.ioBusy
                onTriggered: AppSession.flipActiveLayer(false)
            }
            MenuItem {
                text: qsTr("Rotate 90° &Clockwise")
                enabled: AppSession.hasDocument && !AppSession.ioBusy
                onTriggered: AppSession.rotateCanvas90Cw()
            }
        }
        Menu {
            title: qsTr("&Layer")
            MenuItem {
                text: qsTr("New &Group")
                enabled: AppSession.hasDocument
                onTriggered: AppSession.addGroupLayer()
            }
            MenuItem {
                text: qsTr("Add &Mask")
                enabled: AppSession.hasDocument && !root.activeLayerHasMask
                onTriggered: AppSession.addMaskToActive()
            }
            MenuItem {
                text: qsTr("Delete Mask")
                enabled: AppSession.hasDocument && root.activeLayerHasMask
                onTriggered: AppSession.deleteMaskOnActive()
            }
            MenuItem {
                text: root.activeMaskEnabled ? qsTr("Disable Mask") : qsTr("Enable Mask")
                enabled: AppSession.hasDocument && root.activeLayerHasMask
                onTriggered: AppSession.setMaskEnabledOnActive(!root.activeMaskEnabled)
            }
            MenuItem {
                text: qsTr("Create Clipping Mask")
                checkable: true
                checked: root.activeLayerClips
                enabled: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
                onTriggered: AppSession.setClipsToBelowOnActive(checked)
            }
            MenuItem {
                text: qsTr("New &Adjustment…")
                enabled: AppSession.hasDocument
                onTriggered: AppSession.addAdjustmentLayer("brightness")
            }
        }
        Menu {
            title: qsTr("&View")
            MenuItem { action: zoomFitAction }
            MenuItem {
                text: qsTr("Show &Guides")
                checkable: true
                checked: true
                onTriggered: AppSession.setGuidesVisible(checked)
            }
        }
        Menu {
            title: qsTr("&Help")
            MenuItem {
                text: qsTr("&About PhotoTux")
                icon.source: root.iconUrl("info")
                onTriggered: aboutDialog.open()
            }
        }
    }

    Component.onCompleted: {
        if (!AppSession.hasDocument && !AppSession.ioBusy)
            welcomeDialog.open()
    }

    // —— Top chrome ——
    header: ToolBar {
        height: Theme.toolbarHeight
        background: Rectangle {
            color: Theme.surface
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: Theme.border
            }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceMd
            anchors.rightMargin: Theme.spaceMd
            spacing: Theme.spaceSm

            Image {
                source: Theme.logoUrl
                sourceSize: Qt.size(64, 64)
                Layout.preferredWidth: 22
                Layout.preferredHeight: 22
                fillMode: Image.PreserveAspectFit
                smooth: true
                mipmap: true
            }

            Label {
                text: qsTr("PhotoTux")
                color: Theme.colorOnSurface
                font.weight: Font.DemiBold
                font.pixelSize: Theme.fontWindow
            }

            ToolSeparator {
                contentItem: Rectangle { implicitWidth: 1; color: Theme.border }
            }

            ToolButton {
                action: newAction
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                ToolTip.visible: hovered
                ToolTip.text: newAction.text
            }

            ToolButton {
                action: openAction
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                ToolTip.visible: hovered
                ToolTip.text: openAction.text
            }

            ToolButton {
                action: exportAction
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Export flattened PNG or JPEG")
            }

            ToolSeparator {
                contentItem: Rectangle { implicitWidth: 1; color: Theme.border }
            }

            ToolButton {
                action: undoAction
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                ToolTip.visible: hovered
                ToolTip.text: undoAction.text
            }
            ToolButton {
                action: redoAction
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                ToolTip.visible: hovered
                ToolTip.text: redoAction.text
            }

            Item { Layout.fillWidth: true }

            BusyIndicator {
                visible: AppSession.ioBusy
                running: visible
                Layout.preferredWidth: 18
                Layout.preferredHeight: 18
            }

            Label {
                visible: AppSession.ioBusy
                text: qsTr("Working…")
                color: Theme.primary
                font.pixelSize: Theme.fontBodySm
            }

            ToolButton {
                implicitWidth: 28
                implicitHeight: 28
                icon.source: root.iconUrl("question")
                icon.width: 16
                icon.height: 16
                onClicked: aboutDialog.open()
                ToolTip.visible: hovered
                ToolTip.text: qsTr("About PhotoTux")
            }
        }
    }

    footer: ToolBar {
        height: root.statusHeight
        background: Rectangle {
            color: Theme.surfaceContainer
            Rectangle {
                anchors.top: parent.top
                width: parent.width
                height: 1
                color: Theme.border
            }
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceMd
            anchors.rightMargin: Theme.spaceMd
            spacing: Theme.spaceLg

            Label {
                text: AppSession.hasDocument
                      ? (qsTr("%1 × %2 px").arg(AppSession.docWidth).arg(AppSession.docHeight)
                         + "  ·  " + AppSession.statusText)
                      : AppSession.statusText
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            RowLayout {
                visible: AppSession.dirty
                spacing: Theme.spaceXs
                Image {
                    source: root.iconUrl("circle-notch")
                    sourceSize: Qt.size(12, 12)
                    Layout.preferredWidth: 12
                    Layout.preferredHeight: 12
                }
                Label {
                    text: qsTr("Unsaved")
                    color: Theme.warning
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }

            Label {
                text: AppSession.hasDocument
                      ? (qsTr("Zoom: %1%").arg(Math.round(AppSession.zoom * 100)))
                      : ""
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
            }

            Label {
                text: AppSession.compositeMs > 0
                      ? (qsTr("comp %1 ms").arg(AppSession.compositeMs.toFixed(2)))
                      : ""
                color: AppSession.compositeMs > 0 && AppSession.compositeMs < 2.0
                       ? Theme.primary : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
            }

            Label {
                text: AppSession.fps > 0
                      ? (qsTr("FPS: %1").arg(Math.round(AppSession.fps)))
                      : qsTr("FPS: —")
                color: AppSession.fps >= 60 ? Theme.primary : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
            }

            Label {
                text: qsTr("GPU ACCELERATED")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
                opacity: 0.7
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0
        enabled: !AppSession.ioBusy

        // Left tool strip
        Rectangle {
            Layout.preferredWidth: root.toolStripWidth
            Layout.fillHeight: true
            color: Theme.surface

            Rectangle {
                anchors.right: parent.right
                width: 1
                height: parent.height
                color: Theme.border
            }

            Column {
                id: toolColumn
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: Theme.spaceXs
                spacing: Theme.spaceXs
                width: parent.width - Theme.spaceXs * 2

                Repeater {
                    model: [
                        { id: "tool.brush", stem: "paint-brush", tip: qsTr("Brush") },
                        { id: "tool.eraser", stem: "eraser", tip: qsTr("Eraser") },
                        { id: "tool.select.rect", stem: "selection", tip: qsTr("Rectangular Marquee") },
                        { id: "tool.select.ellipse", stem: "circle-dashed", tip: qsTr("Elliptical Marquee") },
                        { id: "tool.move", stem: "arrows-out-cardinal", tip: qsTr("Move") },
                        { id: "tool.transform", stem: "arrows-out", tip: qsTr("Free Transform") },
                        { id: "tool.crop", stem: "crop", tip: qsTr("Crop") },
                        { id: "tool.fill", stem: "paint-bucket", tip: qsTr("Fill") },
                        { id: "tool.gradient", stem: "gradient", tip: qsTr("Gradient") },
                        { id: "tool.eyedropper", stem: "eyedropper", tip: qsTr("Eyedropper") },
                        { id: "tool.text", stem: "text-t", tip: qsTr("Text") },
                        { id: "tool.pan", stem: "hand", tip: qsTr("Pan") },
                        { id: "tool.zoom", stem: "magnifying-glass", tip: qsTr("Zoom") }
                    ]
                    delegate: Item {
                        width: toolColumn.width
                        height: Theme.toolHit

                        Rectangle {
                            anchors.fill: parent
                            anchors.leftMargin: 2
                            anchors.rightMargin: 2
                            radius: Theme.radiusSm
                            color: AppSession.activeTool === modelData.id
                                   ? Theme.toolActiveBg : (toolHover.hovered ? Theme.surfaceContainerHigh : "transparent")

                            Rectangle {
                                visible: AppSession.activeTool === modelData.id
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.bottom: parent.bottom
                                width: 2
                                color: Theme.primary
                            }

                            Image {
                                anchors.centerIn: parent
                                source: root.iconUrl(modelData.stem)
                                width: 20
                                height: 20
                                sourceSize: Qt.size(20, 20)
                            }

                            HoverHandler { id: toolHover }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    if (AppSession.transformActive
                                            && modelData.id !== "tool.transform")
                                        AppSession.cancelTransform()
                                    if (AppSession.cropPreviewActive
                                            && modelData.id !== "tool.crop")
                                        AppSession.cancelCrop()
                                    AppSession.setActiveTool(modelData.id)
                                    if (modelData.id === "tool.transform")
                                        AppSession.beginTransform()
                                }
                                ToolTip.visible: containsMouse
                                ToolTip.text: modelData.tip
                                ToolTip.delay: 400
                                hoverEnabled: true
                            }
                        }
                    }
                }
            }

            ToolButton {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                anchors.bottomMargin: Theme.spaceSm
                implicitWidth: 36
                implicitHeight: 36
                icon.source: root.iconUrl("gear")
                icon.width: 18
                icon.height: 18
                enabled: false
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Settings (coming later)")
            }
        }

        // GPU canvas viewport
        Item {
            id: canvasHost
            Layout.fillWidth: true
            Layout.fillHeight: true

            Rectangle {
                anchors.fill: parent
                color: Theme.canvasLetterbox
                z: -1
            }

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

            // Live marquee drag preview
            Item {
                id: selectionPreview
                visible: AppSession.selectionPreviewActive && AppSession.hasDocument
                z: 4
                x: root.docToScreenX(AppSession.selectionPreviewX)
                y: root.docToScreenY(AppSession.selectionPreviewY)
                width: Math.max(1, AppSession.selectionPreviewW * AppSession.zoom)
                height: Math.max(1, AppSession.selectionPreviewH * AppSession.zoom)

                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer
                    ShapePath {
                        strokeWidth: 1
                        strokeColor: root.primary
                        fillColor: "transparent"
                        strokeStyle: ShapePath.DashLine
                        dashPattern: [4, 4]
                        PathSvg {
                            path: AppSession.activeTool === "tool.select.ellipse"
                                  ? ("M " + (selectionPreview.width / 2) + " 0 "
                                     + "A " + (selectionPreview.width / 2) + " "
                                     + (selectionPreview.height / 2) + " 0 1 1 "
                                     + (selectionPreview.width / 2) + " " + selectionPreview.height + " "
                                     + "A " + (selectionPreview.width / 2) + " "
                                     + (selectionPreview.height / 2) + " 0 1 1 "
                                     + (selectionPreview.width / 2) + " 0")
                                  : ("M 0 0 H " + selectionPreview.width + " V "
                                     + selectionPreview.height + " H 0 Z")
                        }
                    }
                }
            }

            // Marching ants for committed selection
            Item {
                id: selectionAnts
                visible: AppSession.selectionActive && AppSession.hasDocument
                         && AppSession.selectionW > 0 && AppSession.selectionH > 0
                z: 5
                x: root.docToScreenX(AppSession.selectionX)
                y: root.docToScreenY(AppSession.selectionY)
                width: Math.max(1, AppSession.selectionW * AppSession.zoom)
                height: Math.max(1, AppSession.selectionH * AppSession.zoom)

                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer
                    ShapePath {
                        strokeWidth: 1
                        strokeColor: "#000000"
                        fillColor: "transparent"
                        strokeStyle: ShapePath.DashLine
                        dashPattern: [4, 4]
                        dashOffset: frameClock.phase * 12
                        PathSvg {
                            path: AppSession.selectionShape === "ellipse"
                                  ? ("M " + (selectionAnts.width / 2) + " 0 "
                                     + "A " + (selectionAnts.width / 2) + " "
                                     + (selectionAnts.height / 2) + " 0 1 1 "
                                     + (selectionAnts.width / 2) + " " + selectionAnts.height + " "
                                     + "A " + (selectionAnts.width / 2) + " "
                                     + (selectionAnts.height / 2) + " 0 1 1 "
                                     + (selectionAnts.width / 2) + " 0")
                                  : ("M 0 0 H " + selectionAnts.width + " V "
                                     + selectionAnts.height + " H 0 Z")
                        }
                    }
                    ShapePath {
                        strokeWidth: 1
                        strokeColor: root.primary
                        fillColor: "transparent"
                        strokeStyle: ShapePath.DashLine
                        dashPattern: [4, 4]
                        dashOffset: frameClock.phase * 12 + 4
                        PathSvg {
                            path: AppSession.selectionShape === "ellipse"
                                  ? ("M " + (selectionAnts.width / 2) + " 0 "
                                     + "A " + (selectionAnts.width / 2) + " "
                                     + (selectionAnts.height / 2) + " 0 1 1 "
                                     + (selectionAnts.width / 2) + " " + selectionAnts.height + " "
                                     + "A " + (selectionAnts.width / 2) + " "
                                     + (selectionAnts.height / 2) + " 0 1 1 "
                                     + (selectionAnts.width / 2) + " 0")
                                  : ("M 0 0 H " + selectionAnts.width + " V "
                                     + selectionAnts.height + " H 0 Z")
                        }
                    }
                }
            }

            // Crop drag preview
            Rectangle {
                id: cropPreview
                visible: AppSession.cropPreviewActive && AppSession.hasDocument
                z: 6
                x: root.docToScreenX(AppSession.cropPreviewX)
                y: root.docToScreenY(AppSession.cropPreviewY)
                width: Math.max(1, AppSession.cropPreviewW * AppSession.zoom)
                height: Math.max(1, AppSession.cropPreviewH * AppSession.zoom)
                color: "#3DAEE920"
                border.color: root.primary
                border.width: 1
            }

            // Free-transform handles over document bounds
            Item {
                id: transformChrome
                visible: AppSession.transformActive && AppSession.hasDocument
                z: 7
                x: root.docToScreenX(0)
                y: root.docToScreenY(0)
                width: Math.max(1, AppSession.docWidth * AppSession.zoom)
                height: Math.max(1, AppSession.docHeight * AppSession.zoom)

                Rectangle {
                    anchors.fill: parent
                    color: "transparent"
                    border.color: root.primary
                    border.width: 1
                    transform: [
                        Translate {
                            x: AppSession.transformTx * AppSession.zoom
                            y: AppSession.transformTy * AppSession.zoom
                        },
                        Scale {
                            origin.x: transformChrome.width / 2
                            origin.y: transformChrome.height / 2
                            xScale: AppSession.transformSx
                            yScale: AppSession.transformSy
                        },
                        Rotation {
                            origin.x: transformChrome.width / 2
                            origin.y: transformChrome.height / 2
                            angle: AppSession.transformRot
                        }
                    ]
                }

                Repeater {
                    model: [
                        { nx: 0, ny: 0 }, { nx: 0.5, ny: 0 }, { nx: 1, ny: 0 },
                        { nx: 0, ny: 0.5 }, { nx: 1, ny: 0.5 },
                        { nx: 0, ny: 1 }, { nx: 0.5, ny: 1 }, { nx: 1, ny: 1 }
                    ]
                    delegate: Rectangle {
                        width: 8
                        height: 8
                        radius: 1
                        color: Theme.surface
                        border.color: root.primary
                        border.width: 1
                        z: 8
                        property real hx: modelData.nx * transformChrome.width
                        property real hy: modelData.ny * transformChrome.height
                        x: hx * AppSession.transformSx
                           + (1 - AppSession.transformSx) * transformChrome.width / 2
                           + AppSession.transformTx * AppSession.zoom - width / 2
                        y: hy * AppSession.transformSy
                           + (1 - AppSession.transformSy) * transformChrome.height / 2
                           + AppSession.transformTy * AppSession.zoom - height / 2
                        MouseArea {
                            anchors.fill: parent
                            anchors.margins: -4
                            cursorShape: Qt.SizeFDiagCursor
                            property real startDist: 1
                            onPressed: function (mouse) {
                                var cx = transformChrome.width / 2
                                var cy = transformChrome.height / 2
                                var dx = parent.x + width / 2 - cx
                                var dy = parent.y + height / 2 - cy
                                startDist = Math.max(8, Math.sqrt(dx * dx + dy * dy))
                            }
                            onPositionChanged: function (mouse) {
                                if (!pressed)
                                    return
                                var cx = transformChrome.width / 2
                                var cy = transformChrome.height / 2
                                var gx = mapToItem(transformChrome, mouse.x, mouse.y).x
                                var gy = mapToItem(transformChrome, mouse.x, mouse.y).y
                                var dx = gx - cx
                                var dy = gy - cy
                                var dist = Math.max(8, Math.sqrt(dx * dx + dy * dy))
                                var factor = dist / startDist
                                var sx = Math.max(0.05, Math.abs(AppSession.transformSx) * factor)
                                var sy = Math.max(0.05, Math.abs(AppSession.transformSy) * factor)
                                var constrain = (mouse.modifiers & Qt.ShiftModifier) !== 0
                                        || AppSession.transformConstrain
                                AppSession.updateTransformDraft(
                                            AppSession.transformTx,
                                            AppSession.transformTy,
                                            sx, sy,
                                            AppSession.transformRot,
                                            constrain)
                                startDist = dist
                            }
                        }
                    }
                }
            }

            Label {
                visible: !AppSession.hasDocument
                anchors.centerIn: parent
                z: 2
                text: qsTr("No document open")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBody
            }

            // Continuous phase + FPS + worker poll
            FrameAnimation {
                id: frameClock
                running: root.visible
                property real phase: 0
                property real fpsEma: 0
                property bool startupReported: false
                onTriggered: {
                    if (!startupReported) {
                        startupReported = true
                        AppSession.reportInteractive()
                    }
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
                    if (AppSession.activeTool === "tool.zoom"
                            || root.isSelectTool() || root.isCropTool())
                        return Qt.CrossCursor
                    if (root.isTransformTool())
                        return Qt.SizeAllCursor
                    if (AppSession.activeTool === "tool.brush"
                            || AppSession.activeTool === "tool.eraser")
                        return Qt.BlankCursor
                    return Qt.ArrowCursor
                }
                property real lastX: 0
                property real lastY: 0
                property bool dragging: false
                property bool painting: false
                property bool selecting: false
                property bool cropping: false
                property bool transforming: false
                property real selStartX: 0
                property real selStartY: 0

                Keys.onPressed: function (event) {
                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        if (AppSession.transformActive) {
                            AppSession.commitTransform()
                            event.accepted = true
                        } else if (AppSession.cropPreviewActive) {
                            AppSession.commitCrop(
                                        AppSession.cropPreviewX,
                                        AppSession.cropPreviewY,
                                        AppSession.cropPreviewW,
                                        AppSession.cropPreviewH)
                            event.accepted = true
                        }
                    } else if (event.key === Qt.Key_Escape) {
                        if (AppSession.transformActive) {
                            AppSession.cancelTransform()
                            event.accepted = true
                        } else if (AppSession.cropPreviewActive) {
                            AppSession.cancelCrop()
                            event.accepted = true
                        }
                    }
                }
                focus: true

                onPressed: function (mouse) {
                    forceActiveFocus()
                    lastX = mouse.x
                    lastY = mouse.y
                    dragging = true
                    if (mouse.button === Qt.MiddleButton
                            || AppSession.activeTool === "tool.pan") {
                        cursorShape = Qt.ClosedHandCursor
                        painting = false
                        selecting = false
                        cropping = false
                        transforming = false
                        return
                    }
                    if (root.isSelectTool()) {
                        selecting = true
                        painting = false
                        cropping = false
                        transforming = false
                        selStartX = mouse.x
                        selStartY = mouse.y
                        return
                    }
                    if (root.isCropTool()) {
                        cropping = true
                        selecting = false
                        painting = false
                        transforming = false
                        selStartX = mouse.x
                        selStartY = mouse.y
                        return
                    }
                    if (root.isTransformTool()) {
                        if (!AppSession.transformActive)
                            AppSession.beginTransform()
                        transforming = true
                        selecting = false
                        cropping = false
                        painting = false
                        return
                    }
                    if (AppSession.activeTool === "tool.text") {
                        AppSession.addTextLayer(qsTr("Text"))
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
                    if (selecting) {
                        var x0 = Math.min(selStartX, mouse.x)
                        var y0 = Math.min(selStartY, mouse.y)
                        var w = Math.abs(mouse.x - selStartX)
                        var h = Math.abs(mouse.y - selStartY)
                        var dx = root.screenToDocX(x0)
                        var dy = root.screenToDocY(y0)
                        var dw = w / Math.max(0.001, AppSession.zoom)
                        var dh = h / Math.max(0.001, AppSession.zoom)
                        var combine = root.selectionCombineFromModifiers(mouse.modifiers)
                        if (AppSession.activeTool === "tool.select.ellipse") {
                            AppSession.selectEllipse(
                                        Math.round(dx), Math.round(dy),
                                        Math.round(dw), Math.round(dh), combine)
                        } else {
                            AppSession.selectRect(
                                        Math.round(dx), Math.round(dy),
                                        Math.round(dw), Math.round(dh), combine)
                        }
                        AppSession.setSelectionPreview(false, 0, 0, 0, 0)
                        selecting = false
                    }
                    if (cropping) {
                        cropping = false
                    }
                    transforming = false
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
                    if (selecting) {
                        var x0 = Math.min(selStartX, mouse.x)
                        var y0 = Math.min(selStartY, mouse.y)
                        var w = Math.abs(mouse.x - selStartX)
                        var h = Math.abs(mouse.y - selStartY)
                        AppSession.setSelectionPreview(
                                    true,
                                    Math.round(root.screenToDocX(x0)),
                                    Math.round(root.screenToDocY(y0)),
                                    Math.round(w / Math.max(0.001, AppSession.zoom)),
                                    Math.round(h / Math.max(0.001, AppSession.zoom)))
                        return
                    }
                    if (cropping) {
                        var cx0 = Math.min(selStartX, mouse.x)
                        var cy0 = Math.min(selStartY, mouse.y)
                        var cw = Math.abs(mouse.x - selStartX)
                        var ch = Math.abs(mouse.y - selStartY)
                        AppSession.setCropPreview(
                                    true,
                                    Math.round(root.screenToDocX(cx0)),
                                    Math.round(root.screenToDocY(cy0)),
                                    Math.round(cw / Math.max(0.001, AppSession.zoom)),
                                    Math.round(ch / Math.max(0.001, AppSession.zoom)))
                        return
                    }
                    if (transforming && AppSession.transformActive) {
                        var tdx = (mouse.x - lastX) / Math.max(0.001, AppSession.zoom)
                        var tdy = (mouse.y - lastY) / Math.max(0.001, AppSession.zoom)
                        lastX = mouse.x
                        lastY = mouse.y
                        AppSession.updateTransformDraft(
                                    AppSession.transformTx + tdx,
                                    AppSession.transformTy + tdy,
                                    AppSession.transformSx,
                                    AppSession.transformSy,
                                    AppSession.transformRot,
                                    AppSession.transformConstrain
                                    || ((mouse.modifiers & Qt.ShiftModifier) !== 0))
                        return
                    }
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

                // Properties panel header
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.panelHeaderHeight
                    color: Theme.surfaceContainer
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: Theme.border
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.spaceSm
                        text: qsTr("Properties")
                        color: Theme.colorOnSurfaceVariant
                        font.pixelSize: Theme.fontLabel
                        font.weight: Font.Medium
                    }
                }

                Flickable {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredHeight: parent.height * 0.52
                    contentHeight: propsCol.implicitHeight
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    ColumnLayout {
                        id: propsCol
                        width: parent.width
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.margins: Theme.spaceMd
                        spacing: Theme.spaceMd

                        // Selection combine modes
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: root.isSelectTool()
                            Label {
                                text: qsTr("Selection")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceXs
                                Repeater {
                                    model: [
                                        { id: "replace", stem: "selection", tip: qsTr("Replace") },
                                        { id: "add", stem: "selection-plus", tip: qsTr("Add") },
                                        { id: "subtract", stem: "minus-circle", tip: qsTr("Subtract") },
                                        { id: "intersect", stem: "intersect", tip: qsTr("Intersect") }
                                    ]
                                    delegate: ToolButton {
                                        implicitWidth: 32
                                        implicitHeight: 28
                                        checkable: true
                                        checked: AppSession.selectionCombine === modelData.id
                                        icon.source: root.iconUrl(modelData.stem)
                                        icon.width: 16
                                        icon.height: 16
                                        enabled: AppSession.hasDocument
                                        onClicked: AppSession.setSelectionCombine(modelData.id)
                                        ToolTip.visible: hovered
                                        ToolTip.text: modelData.tip
                                        background: Rectangle {
                                            radius: Theme.radiusSm
                                            color: parent.checked
                                                   ? Theme.toolActiveBg
                                                   : (parent.hovered ? Theme.surfaceContainerHigh : "transparent")
                                            border.color: parent.checked ? Theme.primary : "transparent"
                                            border.width: 1
                                        }
                                    }
                                }
                            }
                            Label {
                                text: qsTr("Shift add · Alt subtract · Shift+Alt intersect")
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        // Crop / Transform commit chrome
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: root.isCropTool() || root.isTransformTool()
                                     || AppSession.transformActive
                                     || AppSession.cropPreviewActive
                            Label {
                                text: root.isCropTool() || AppSession.cropPreviewActive
                                      ? qsTr("Crop") : qsTr("Free Transform")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceXs
                                Button {
                                    text: qsTr("Apply")
                                    enabled: AppSession.hasDocument
                                             && (AppSession.transformActive
                                                 || AppSession.cropPreviewActive)
                                    onClicked: {
                                        if (AppSession.transformActive)
                                            AppSession.commitTransform()
                                        else if (AppSession.cropPreviewActive)
                                            AppSession.commitCrop(
                                                        AppSession.cropPreviewX,
                                                        AppSession.cropPreviewY,
                                                        AppSession.cropPreviewW,
                                                        AppSession.cropPreviewH)
                                    }
                                }
                                Button {
                                    text: qsTr("Cancel")
                                    enabled: AppSession.transformActive
                                             || AppSession.cropPreviewActive
                                    onClicked: {
                                        if (AppSession.transformActive)
                                            AppSession.cancelTransform()
                                        else
                                            AppSession.cancelCrop()
                                    }
                                }
                            }
                            CheckBox {
                                visible: AppSession.transformActive || root.isTransformTool()
                                text: qsTr("Constrain proportions")
                                checked: AppSession.transformConstrain
                                onToggled: AppSession.updateTransformDraft(
                                               AppSession.transformTx,
                                               AppSession.transformTy,
                                               AppSession.transformSx,
                                               AppSession.transformSy,
                                               AppSession.transformRot,
                                               checked)
                            }
                            Label {
                                visible: AppSession.transformActive
                                text: qsTr("Drag to move · handles scale · Enter apply · Esc cancel")
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                            RowLayout {
                                visible: AppSession.transformActive
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Rotate")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                }
                                Slider {
                                    Layout.fillWidth: true
                                    from: -180
                                    to: 180
                                    value: AppSession.transformRot
                                    onMoved: AppSession.updateTransformDraft(
                                                 AppSession.transformTx,
                                                 AppSession.transformTy,
                                                 AppSession.transformSx,
                                                 AppSession.transformSy,
                                                 value,
                                                 AppSession.transformConstrain)
                                }
                            }
                        }

                        // Brush size
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: AppSession.activeTool === "tool.brush"
                                     || AppSession.activeTool === "tool.eraser"
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Brush Size")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(brushSlider.value) + " px"
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                id: brushSlider
                                Layout.fillWidth: true
                                from: 1
                                to: 200
                                value: AppSession.brushSize
                                enabled: AppSession.hasDocument
                                onMoved: AppSession.setBrushSize(value)
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: AppSession.activeTool === "tool.brush"
                                     || AppSession.activeTool === "tool.eraser"
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Hardness")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(hardnessSlider.value * 100) + " %"
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                id: hardnessSlider
                                Layout.fillWidth: true
                                from: 0
                                to: 1
                                value: AppSession.brushHardness
                                enabled: AppSession.hasDocument
                                onMoved: AppSession.setBrushHardness(value)
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: root.activeLayerHasMask
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: AppSession.maskEditActive
                                          ? qsTr("Mask · Editing") : qsTr("Mask")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                ToolButton {
                                    implicitWidth: 28
                                    implicitHeight: 28
                                    icon.source: root.iconUrl("rectangle-dashed")
                                    icon.width: 16
                                    icon.height: 16
                                    checkable: true
                                    checked: AppSession.maskEditActive
                                    onClicked: AppSession.setMaskEditTarget(checked)
                                    ToolTip.visible: hovered
                                    ToolTip.text: qsTr("Edit layer mask")
                                }
                            }
                            CheckBox {
                                text: qsTr("Enable mask")
                                checked: root.activeMaskEnabled
                                onToggled: AppSession.setMaskEnabledOnActive(checked)
                            }
                            Button {
                                text: qsTr("Delete Mask")
                                onClicked: AppSession.deleteMaskOnActive()
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            Label {
                                text: qsTr("Color")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceSm
                                Rectangle {
                                    width: 28
                                    height: 28
                                    radius: Theme.radiusSm
                                    color: Qt.rgba(AppSession.brushR, AppSession.brushG, AppSession.brushB, 1)
                                    border.color: Theme.border
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Slider {
                                        id: colorR
                                        Layout.fillWidth: true
                                        from: 0; to: 1
                                        value: AppSession.brushR
                                        enabled: AppSession.hasDocument
                                        onMoved: AppSession.setBrushColor(value, colorG.value, colorB.value)
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
                                }
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Zoom")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(zoomSlider.value * 100) + " %"
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                id: zoomSlider
                                Layout.fillWidth: true
                                from: 0.05
                                to: 8.0
                                value: AppSession.zoom
                                enabled: AppSession.hasDocument
                                onMoved: AppSession.setZoom(value)
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceSm
                                Button {
                                    text: qsTr("Fit")
                                    Layout.fillWidth: true
                                    enabled: AppSession.hasDocument
                                    onClicked: {
                                        AppSession.zoomToFit()
                                        zoomSlider.value = AppSession.zoom
                                    }
                                }
                                Button {
                                    text: qsTr("100%")
                                    Layout.fillWidth: true
                                    enabled: AppSession.hasDocument
                                    onClicked: {
                                        AppSession.setZoom(1.0)
                                        zoomSlider.value = 1.0
                                    }
                                }
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Layer Opacity")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(layerOpacitySlider.value * 100) + " %"
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                id: layerOpacitySlider
                                Layout.fillWidth: true
                                from: 0
                                to: 1
                                value: AppSession.activeOpacity
                                enabled: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
                                onMoved: AppSession.setActiveOpacity(value)
                            }
                        }

                        Label {
                            text: gpuCanvas.gpuStatus
                            color: Theme.colorOnSurfaceMuted
                            font.pixelSize: Theme.fontLabelSm
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }
                }

                // Layers panel
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.panelHeaderHeight
                    color: Theme.surfaceContainer
                    Rectangle {
                        anchors.top: parent.top
                        width: parent.width
                        height: 1
                        color: Theme.border
                    }
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: Theme.border
                    }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceSm
                        anchors.rightMargin: Theme.spaceXs
                        spacing: Theme.spaceXs
                        Label {
                            text: qsTr("Layers")
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.Medium
                            Layout.fillWidth: true
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            icon.source: root.iconUrl("plus")
                            icon.width: 14
                            icon.height: 14
                            enabled: AppSession.hasDocument
                            onClicked: AppSession.addLayer()
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Add layer")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            icon.source: root.iconUrl("folder")
                            icon.width: 14
                            icon.height: 14
                            enabled: AppSession.hasDocument
                            onClicked: AppSession.addGroupLayer()
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Add group")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            icon.source: root.iconUrl("trash")
                            icon.width: 14
                            icon.height: 14
                            enabled: AppSession.hasDocument && AppSession.layerCount > 1
                            onClicked: AppSession.deleteActiveLayer()
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Delete layer")
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Theme.surfaceSunken

                    ListView {
                        id: layerList
                        anchors.fill: parent
                        clip: true
                        spacing: 0
                        model: AppSession.layerCount
                        delegate: Rectangle {
                            width: layerList.width
                            height: 36
                            readonly property int stackIndex: AppSession.layerCount - 1 - index
                            readonly property var nameParts: AppSession.layerNames.split("|")
                            readonly property var visParts: AppSession.layerVisibility.split("|")
                            readonly property var kindParts: AppSession.layerKinds.split("|")
                            readonly property var maskParts: root.layerMaskFlagParts
                            readonly property var clipParts: root.layerClipParts
                            readonly property string layerName: stackIndex >= 0 && stackIndex < nameParts.length
                                ? nameParts[stackIndex] : ""
                            readonly property string layerKind: stackIndex >= 0 && stackIndex < kindParts.length
                                ? kindParts[stackIndex] : "raster"
                            readonly property bool layerVis: stackIndex >= 0 && stackIndex < visParts.length
                                ? visParts[stackIndex] === "1" : true
                            readonly property int maskFlag: stackIndex >= 0 && stackIndex < maskParts.length
                                ? Number(maskParts[stackIndex]) : 0
                            readonly property bool hasMask: maskFlag !== 0
                            readonly property bool maskEnabled: maskFlag === 1
                            readonly property bool clipsToBelow: stackIndex >= 0 && stackIndex < clipParts.length
                                ? clipParts[stackIndex] === "1" : false
                            readonly property bool isActive: AppSession.activeLayerIndex === stackIndex
                            color: isActive ? Theme.surfaceRaised
                                   : (layerHover.hovered ? Theme.surfaceContainer : "transparent")
                            border.color: isActive ? Theme.primary : "transparent"
                            border.width: isActive ? 1 : 0

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceSm
                                spacing: Theme.spaceSm

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

                                Rectangle {
                                    width: 24
                                    height: 24
                                    radius: Theme.radiusXs
                                    color: Theme.surface
                                    border.color: Theme.border
                                    Label {
                                        anchors.centerIn: parent
                                        text: layerKind === "group" ? "G"
                                              : (layerKind === "text" ? "T"
                                              : (layerKind === "adjustment" ? "A" : ""))
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
                                    border.color: AppSession.maskEditActive && isActive
                                                  ? Theme.primary : Theme.border
                                    border.width: AppSession.maskEditActive && isActive ? 2 : 1
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
                                            AppSession.setActiveLayer(stackIndex)
                                            AppSession.setMaskEditTarget(true)
                                        }
                                        ToolTip.visible: containsMouse
                                        ToolTip.text: qsTr("Edit layer mask")
                                    }
                                }

                                Image {
                                    visible: hasMask
                                    source: root.iconUrl(maskEnabled ? "eye" : "eye-slash")
                                    width: 14
                                    height: 14
                                    sourceSize: Qt.size(14, 14)
                                    opacity: maskEnabled ? 1.0 : 0.55
                                    z: 2
                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            AppSession.setActiveLayer(stackIndex)
                                            AppSession.setMaskEnabledOnActive(!maskEnabled)
                                        }
                                        ToolTip.visible: containsMouse
                                        ToolTip.text: maskEnabled
                                                      ? qsTr("Disable layer mask")
                                                      : qsTr("Enable layer mask")
                                    }
                                }

                                Label {
                                    visible: clipsToBelow
                                    text: "↳"
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontBody
                                    ToolTip.visible: clipHover.hovered
                                    ToolTip.text: qsTr("Clipping mask")
                                    HoverHandler { id: clipHover }
                                }

                                Label {
                                    Layout.fillWidth: true
                                    text: layerName
                                    color: isActive ? Theme.colorOnSurface : Theme.colorOnSurfaceVariant
                                    font.pixelSize: Theme.fontBodySm
                                    elide: Text.ElideRight
                                }
                            }

                            HoverHandler { id: layerHover }
                            MouseArea {
                                anchors.fill: parent
                                anchors.leftMargin: 28
                                onClicked: AppSession.setActiveLayer(stackIndex)
                            }
                        }
                    }
                }

                // History panel
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.panelHeaderHeight
                    color: Theme.surfaceContainer
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.spaceSm
                        text: qsTr("History")
                        color: Theme.colorOnSurfaceVariant
                        font.pixelSize: Theme.fontLabel
                        font.weight: Font.Medium
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 120
                    color: Theme.surfaceSunken
                    ListView {
                        anchors.fill: parent
                        clip: true
                        model: AppSession.historyLabels.length > 0
                               ? AppSession.historyLabels.split("|") : []
                        delegate: Label {
                            width: parent ? parent.width : 100
                            height: 22
                            leftPadding: Theme.spaceSm
                            text: modelData
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontBodySm
                            elide: Text.ElideRight
                            Accessible.name: modelData
                        }
                    }
                }
            }
        }
    }

    FileDialog {
        id: openFileDialog
        title: qsTr("Open Document")
        currentFolder: StandardPaths.writableLocation(StandardPaths.PicturesLocation)
        fileMode: FileDialog.OpenFile
        nameFilters: [
            qsTr("All supported (*.ptx *.png *.jpg *.jpeg *.webp *.tif *.tiff *.bmp *.gif *.psd)"),
            qsTr("PhotoTux documents (*.ptx)"),
            qsTr("Image files (*.png *.jpg *.jpeg *.webp *.tif *.tiff *.bmp *.gif)"),
            qsTr("Photoshop (*.psd)")
        ]
        onAccepted: AppSession.openRasterFile(selectedFile.toString())
    }

    FileDialog {
        id: saveFileDialog
        title: qsTr("Save PhotoTux Document")
        currentFolder: StandardPaths.writableLocation(StandardPaths.PicturesLocation)
        fileMode: FileDialog.SaveFile
        nameFilters: [ qsTr("PhotoTux documents (*.ptx)") ]
        defaultSuffix: "ptx"
        onAccepted: AppSession.saveDocument(selectedFile.toString())
    }

    FileDialog {
        id: exportFileDialog
        title: qsTr("Export Image")
        currentFolder: StandardPaths.writableLocation(StandardPaths.PicturesLocation)
        fileMode: FileDialog.SaveFile
        nameFilters: [
            qsTr("PNG images (*.png)"),
            qsTr("JPEG images (*.jpg *.jpeg)"),
            qsTr("WebP images (*.webp)"),
            qsTr("TIFF images (*.tif *.tiff)")
        ]
        defaultSuffix: selectedNameFilter.extensions.length > 0
                       ? selectedNameFilter.extensions[0] : "png"
        onAccepted: AppSession.exportRasterFile(selectedFile.toString())
    }

    Dialog {
        id: unsavedDialog
        anchors.centerIn: parent
        modal: true
        title: qsTr("Unsaved changes")
        closePolicy: Popup.CloseOnEscape
        onRejected: root.pendingDestructiveAction = ""
        width: 440

        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
            radius: Theme.radiusMd
        }

        contentItem: Label {
            width: parent ? parent.width - 32 : 400
            text: qsTr("Save the document as .ptx, discard changes, or cancel?")
            wrapMode: Text.WordWrap
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBody
        }

        footer: DialogButtonBox {
            standardButtons: DialogButtonBox.Save | DialogButtonBox.Discard | DialogButtonBox.Cancel
            onAccepted: {
                unsavedDialog.close()
                if (AppSession.documentPath && AppSession.documentPath.length > 0)
                    AppSession.saveDocument("")
                else
                    saveFileDialog.open()
            }
            onDiscarded: {
                unsavedDialog.close()
                root.discardAndContinue()
            }
            onRejected: {
                root.pendingDestructiveAction = ""
                unsavedDialog.close()
            }
        }
    }

    Dialog {
        id: compatibilityDialog
        anchors.centerIn: parent
        modal: true
        title: qsTr("Compatibility report")
        standardButtons: Dialog.Ok
        width: 480
        visible: AppSession.compatibilityReport.length > 0

        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
            radius: Theme.radiusMd
        }

        contentItem: Label {
            width: parent ? parent.width - 32 : 440
            text: AppSession.compatibilityReport
            wrapMode: Text.WordWrap
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
            Accessible.name: qsTr("Import compatibility report")
        }
    }

    Dialog {
        id: ioErrorDialog
        anchors.centerIn: parent
        modal: true
        title: qsTr("File operation failed")
        standardButtons: Dialog.Ok
        width: 440

        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
            radius: Theme.radiusMd
        }

        contentItem: Label {
            width: parent ? parent.width - 32 : 400
            text: AppSession.ioError
            wrapMode: Text.WordWrap
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBody
        }
    }

    Dialog {
        id: aboutDialog
        anchors.centerIn: parent
        modal: true
        title: qsTr("About PhotoTux")
        standardButtons: Dialog.Ok
        width: 400

        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
            radius: Theme.radiusMd
        }

        contentItem: ColumnLayout {
            spacing: Theme.spaceMd
            width: 360

            Image {
                Layout.alignment: Qt.AlignHCenter
                source: Theme.logoUrl
                sourceSize: Qt.size(256, 256)
                Layout.preferredWidth: 96
                Layout.preferredHeight: 96
                fillMode: Image.PreserveAspectFit
                smooth: true
                mipmap: true
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("PhotoTux")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontHeadline
                font.weight: Font.DemiBold
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("Professional Image Environment\nGPU-first editor for Linux and Wayland.")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBodySm
                wrapMode: Text.WordWrap
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("Version 0.1.0  ·  GPU ACCELERATED")
                color: Theme.colorOnSurfaceDisabled
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
            }
        }
    }

    WelcomeDialog {
        id: welcomeDialog
        anchors.centerIn: parent
        onRequestNew: newDocDialog.open()
        onRequestOpen: openFileDialog.open()
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
        function onIoErrorChanged() {
            if (AppSession.ioError.length > 0)
                ioErrorDialog.open()
        }
    }

}
