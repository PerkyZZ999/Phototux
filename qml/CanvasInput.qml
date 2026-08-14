// Canvas pointer input: every tool's press, drag and release.
//
// Extracted from the shell as the last large block in `Main.qml`. It is not
// only input routing — it owns the in-progress drag state that the canvas
// overlays draw from (the gradient preview, the brush cursor), which is why
// those properties are declared here and read from the instance rather than
// pushed anywhere.
//
// The seam is ten shell helpers plus two signals. The helpers are passed
// rather than reached for: the tool predicates and the screen-to-document
// conversions are defined once on the shell root, and duplicating either here
// would be a second definition of which tool is a selection tool, or of where
// the canvas thinks the pointer is.
//
// The two context menus stay with the shell. This reports where the user asked
// for one; opening it is the shell's business, the same arrangement
// `LayersPanel` uses.

import QtQuick
import phototux_ui

MouseArea {
    id: root

    /// Screen→document conversion, defined once on the shell.
    required property var screenToDocX
    required property var screenToDocY

    /// Which tool is active, as the shell defines it. Five predicates rather
    /// than one tool string because the shell already answers these, and
    /// re-deriving them here would be a second definition.
    required property var isSelectTool
    required property var isLassoTool
    required property var isPolygonTool
    required property var isCropTool
    required property var isTransformTool

    /// Selection combine mode for the current keyboard modifiers.
    required property var selectionCombineFromModifiers

    /// Append a point to the pen-tool path being drawn.
    required property var appendPathPoint

    /// Defer a call past the end of the current host slot. Required for
    /// anything reacting to a host signal — handbook 32.
    required property var afterHostSlot

    /// Raised where the user asked for a context menu. The menus belong to the
    /// shell, so this reports the position rather than opening one.
    signal selectionContextMenuRequested(real localX, real localY)
    signal canvasContextMenuRequested(real localX, real localY)

acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
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
// Pressure source. QQuickMouseEvent carries no pressure — the
// property simply does not exist — so reading it off the mouse
// event silently produced 1.0 for every dab of every stroke,
// and the brush engine's pressure dynamics never saw a signal.
// A PointHandler tracks the same point and exposes the device's
// real value. It takes only a passive grab, so the MouseArea
// above keeps owning every gesture; this contributes nothing
// but the number. Devices without pressure report 0 while held,
// which falls back to full pressure, so a mouse behaves exactly
// as it did before.
PointHandler {
    id: canvasPoint
    acceptedButtons: Qt.LeftButton
}
function strokePressure() {
    var p = canvasPoint.active ? canvasPoint.point.pressure : 0
    return p > 0 ? p : 1.0
}

property real lastX: 0
property real lastY: 0
property bool dragging: false
property bool painting: false
property bool selecting: false
property bool lassoing: false
property bool polygoning: false
property bool cropping: false
property bool transforming: false
property bool gradienting: false
property bool pathEditing: false
property int pathDragIndex: -1
property real selStartX: 0
property real selStartY: 0
property real gradStartX: 0
property real gradStartY: 0
property real gradEndX: 0
property real gradEndY: 0
property string pathDraft: ""
property real pathCursorX: 0
property real pathCursorY: 0
property real lastClickMs: 0

Connections {
    target: AppSession
    function onActiveToolChanged() {
        if (!root.painting)
            return
        // Clear first so a second tool change cannot queue a
        // second end, then defer: `activeTool` flips inside
        // `setActiveTool`, and ending the stroke from here would
        // re-enter AppSession while it is still borrowed.
        root.painting = false
        root.afterHostSlot(AppSession.strokeEnd)
    }
}

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
        } else if (polygoning && pathDraft.split("|").length >= 3) {
            AppSession.selectPolygon(
                        pathDraft,
                        root.selectionCombineFromModifiers(event.modifiers))
            polygoning = false
            pathDraft = ""
            event.accepted = true
        }
    } else if (event.key === Qt.Key_Escape) {
        if (AppSession.transformActive) {
            AppSession.cancelTransform()
            event.accepted = true
        } else if (AppSession.cropPreviewActive) {
            AppSession.cancelCrop()
            event.accepted = true
        } else if (polygoning || lassoing || AppSession.selectionPathActive) {
            AppSession.cancelSelectionPath()
            polygoning = false
            lassoing = false
            pathDraft = ""
            event.accepted = true
        }
    } else if (event.key === Qt.Key_Delete
               || event.key === Qt.Key_Backspace) {
        if (AppSession.activeTool === "tool.path-edit"
                && AppSession.pathEditSelected >= 0) {
            AppSession.pathDeleteSelectedAnchor()
            event.accepted = true
        }
    }
}
focus: true

onPressed: function (mouse) {
    forceActiveFocus()
    lastX = mouse.x
    lastY = mouse.y
    if (mouse.button === Qt.RightButton) {
        dragging = false
        painting = false
        if (AppSession.selectionActive)
            root.selectionContextMenuRequested(mouse.x, mouse.y)
        else
            root.canvasContextMenuRequested(mouse.x, mouse.y)
        return
    }
    dragging = true
    if (mouse.button === Qt.MiddleButton
            || AppSession.activeTool === "tool.pan") {
        cursorShape = Qt.ClosedHandCursor
        painting = false
        selecting = false
        lassoing = false
        polygoning = false
        cropping = false
        transforming = false
        return
    }
    if (root.isLassoTool()) {
        lassoing = true
        selecting = false
        polygoning = false
        painting = false
        cropping = false
        transforming = false
        pathDraft = ""
        var ldx = root.screenToDocX(mouse.x)
        var ldy = root.screenToDocY(mouse.y)
        pathDraft = root.appendPathPoint(pathDraft, ldx, ldy, 0)
        AppSession.setSelectionPath(pathDraft)
        return
    }
    if (root.isPolygonTool()) {
        selecting = false
        lassoing = false
        painting = false
        cropping = false
        transforming = false
        var now = Date.now()
        var pdx = root.screenToDocX(mouse.x)
        var pdy = root.screenToDocY(mouse.y)
        if (polygoning && (now - lastClickMs) < 350
                && pathDraft.split("|").length >= 3) {
            AppSession.selectPolygon(
                        pathDraft,
                        root.selectionCombineFromModifiers(mouse.modifiers))
            polygoning = false
            pathDraft = ""
            lastClickMs = 0
            return
        }
        polygoning = true
        pathDraft = root.appendPathPoint(pathDraft, pdx, pdy, 0)
        pathCursorX = pdx
        pathCursorY = pdy
        AppSession.setSelectionPath(
                    pathDraft + "|" + Math.round(pathCursorX) + ","
                    + Math.round(pathCursorY))
        lastClickMs = now
        return
    }
    if (root.isSelectTool()) {
        selecting = true
        lassoing = false
        polygoning = false
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
    if (AppSession.activeTool === "tool.shape") {
        AppSession.addShapeLayer("rect")
        return
    }
    if (AppSession.activeTool === "tool.path-edit") {
        pathEditing = true
        var pdx = root.screenToDocX(mouse.x)
        var pdy = root.screenToDocY(mouse.y)
        pathDragIndex = AppSession.pathHitTest(pdx, pdy)
        if (pathDragIndex < 0) {
            AppSession.pathAddAnchor(pdx, pdy)
            pathDragIndex = AppSession.pathEditSelected
        }
        return
    }
    if (AppSession.activeTool === "tool.fill") {
        AppSession.fillActiveLayer()
        return
    }
    if (AppSession.activeTool === "tool.eyedropper") {
        AppSession.sampleColorAt(
                    root.screenToDocX(mouse.x),
                    root.screenToDocY(mouse.y))
        return
    }
    if (AppSession.activeTool === "tool.gradient") {
        gradienting = true
        painting = false
        selecting = false
        gradStartX = root.screenToDocX(mouse.x)
        gradStartY = root.screenToDocY(mouse.y)
        gradEndX = gradStartX
        gradEndY = gradStartY
        return
    }
    if (AppSession.activeTool === "tool.brush"
            || AppSession.activeTool === "tool.eraser") {
        painting = true
        AppSession.strokeBegin(mouse.x, mouse.y,
                               root.strokePressure())
    }
}
onReleased: function (mouse) {
    if (lassoing) {
        var ldx = root.screenToDocX(mouse.x)
        var ldy = root.screenToDocY(mouse.y)
        pathDraft = root.appendPathPoint(pathDraft, ldx, ldy, 0)
        if (pathDraft.split("|").length >= 3) {
            AppSession.selectPolygon(
                        pathDraft,
                        root.selectionCombineFromModifiers(mouse.modifiers))
        } else {
            AppSession.cancelSelectionPath()
        }
        lassoing = false
        pathDraft = ""
    }
    if (selecting) {
        var x0 = Math.min(selStartX, mouse.x)
        var y0 = Math.min(selStartY, mouse.y)
        var w = Math.abs(mouse.x - selStartX)
        var h = Math.abs(mouse.y - selStartY)
        var dx = root.screenToDocX(x0)
        var dy = root.screenToDocY(y0)
        var dw = w / Math.max(0.001, AppSession.zoom)
        var dh = h / Math.max(0.001, AppSession.zoom)
        if (AppSession.prefSnap) {
            var sx0 = AppSession.snapDocumentValue(dx, "v")
            var sy0 = AppSession.snapDocumentValue(dy, "h")
            var sx1 = AppSession.snapDocumentValue(dx + dw, "v")
            var sy1 = AppSession.snapDocumentValue(dy + dh, "h")
            dx = sx0
            dy = sy0
            dw = Math.max(1, sx1 - sx0)
            dh = Math.max(1, sy1 - sy0)
        }
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
    if (gradienting) {
        gradEndX = root.screenToDocX(mouse.x)
        gradEndY = root.screenToDocY(mouse.y)
        AppSession.commitLinearGradient(
                    gradStartX, gradStartY, gradEndX, gradEndY)
        gradienting = false
    }
    if (painting) {
        AppSession.strokeEnd()
        painting = false
    }
    if (pathEditing) {
        if (pathDragIndex >= 0) {
            AppSession.pathMoveAnchor(
                        pathDragIndex,
                        root.screenToDocX(mouse.x),
                        root.screenToDocY(mouse.y))
        }
        pathEditing = false
        pathDragIndex = -1
    }
    dragging = false
    if (AppSession.activeTool === "tool.pan")
        cursorShape = Qt.OpenHandCursor
}
onPositionChanged: function (mouse) {
    if (!AppSession.hasDocument)
        return
    if (pathEditing && pathDragIndex >= 0 && dragging) {
        // Live drag preview via commit on move (undoable per release).
        return
    }
    if (polygoning) {
        pathCursorX = root.screenToDocX(mouse.x)
        pathCursorY = root.screenToDocY(mouse.y)
        AppSession.setSelectionPath(
                    pathDraft + "|" + Math.round(pathCursorX) + ","
                    + Math.round(pathCursorY))
        return
    }
    if (!dragging)
        return
    if (gradienting) {
        gradEndX = root.screenToDocX(mouse.x)
        gradEndY = root.screenToDocY(mouse.y)
        return
    }
    if (lassoing) {
        var ldx = root.screenToDocX(mouse.x)
        var ldy = root.screenToDocY(mouse.y)
        pathDraft = root.appendPathPoint(
                    pathDraft, ldx, ldy,
                    Math.max(2, 3 / Math.max(0.001, AppSession.zoom)))
        AppSession.setSelectionPath(pathDraft)
        return
    }
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
    } else if (painting) {
        AppSession.strokeMove(mouse.x, mouse.y,
                              root.strokePressure())
    }
}
onWheel: function (wheel) {
    if (!AppSession.hasDocument)
        return
    var steps = wheel.angleDelta.y / 120.0
    var factor = Math.pow(1.12, steps)
    AppSession.zoomAt(factor, wheel.x, wheel.y)
    wheel.accepted = true
}
onDoubleClicked: function (mouse) {
    if (AppSession.hasDocument)
        AppSession.zoomToFit()
}
}
