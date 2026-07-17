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
    readonly property var layerKindParts: AppSession.layerKinds.length > 0
                                         ? AppSession.layerKinds.split("|") : []
    readonly property string activeLayerKind: AppSession.activeLayerIndex >= 0
                                             && AppSession.activeLayerIndex < layerKindParts.length
                                             ? layerKindParts[AppSession.activeLayerIndex] : ""
    readonly property var guidesModel: {
        try {
            return JSON.parse(AppSession.guidesJson || "[]")
        } catch (e) {
            return []
        }
    }
    readonly property var actionDescriptors: {
        try {
            return JSON.parse(AppSession.actionsJson || "[]")
        } catch (e) {
            return []
        }
    }
    readonly property var shortcutMap: {
        try {
            return JSON.parse(AppSession.shortcutsJson || "{}")
        } catch (e) {
            return ({})
        }
    }
    readonly property var actionShortcutMap: {
        try {
            return JSON.parse(AppSession.actionShortcutsJson || "{}")
        } catch (e) {
            return ({})
        }
    }
    readonly property var shortcutChordList: Object.keys(root.shortcutMap)
    readonly property var toolDescriptors: {
        try {
            return JSON.parse(AppSession.toolDescriptorsJson || "[]")
        } catch (e) {
            return []
        }
    }
    readonly property var panelDescriptors: {
        try {
            return JSON.parse(AppSession.panelDescriptorsJson || "[]")
        } catch (e) {
            return []
        }
    }
    readonly property var dockTopology: {
        try {
            return JSON.parse(AppSession.dockTopologyJson || "{}")
        } catch (e) {
            return ({ right_stack: [] })
        }
    }
    readonly property var panelVisibilityMap: {
        try {
            return JSON.parse(AppSession.panelVisibilityJson || "{}")
        } catch (e) {
            return ({})
        }
    }
    readonly property var dockRightStack: dockTopology.right_stack || []

    /// Row base for GridLayout dock (header = base*2, body = base*2+1).
    function dockStackRow(panelId) {
        var stack = root.dockRightStack
        for (var i = 0; i < stack.length; ++i) {
            if (stack[i] === panelId)
                return i
        }
        return 1000
    }

    function panelTitle(panelId) {
        var all = root.panelDescriptors
        for (var i = 0; i < all.length; ++i) {
            if (all[i].id === panelId)
                return all[i].title || panelId
        }
        return panelId
    }

    function panelIsVisible(panelId) {
        if (panelId === "panel.navigator")
            return AppSession.panelNavigatorVisible
        if (panelId === "panel.swatches")
            return AppSession.panelSwatchesVisible
        if (panelId === "panel.layers")
            return AppSession.panelLayersVisible
        if (panelId === "panel.history")
            return AppSession.panelHistoryVisible
        if (panelId === "panel.properties")
            return AppSession.panelPropertiesVisible
        var m = root.panelVisibilityMap
        return m[panelId] === true
    }

    function panelHasBody(panelId) {
        return panelId === "panel.properties"
                || panelId === "panel.navigator"
                || panelId === "panel.swatches"
                || panelId === "panel.layers"
                || panelId === "panel.history"
    }

    // Fallback map if a descriptor still uses tool.* icon keys.
    readonly property var toolIconStemMap: ({
        "tool.brush": "paint-brush",
        "tool.eraser": "eraser",
        "tool.select.rect": "selection",
        "tool.select.ellipse": "circle-dashed",
        "tool.select.lasso": "lasso",
        "tool.select.polygon": "polygon",
        "tool.move": "arrows-out-cardinal",
        "tool.transform": "arrows-out",
        "tool.crop": "crop",
        "tool.fill": "paint-bucket",
        "tool.gradient": "gradient",
        "tool.eyedropper": "eyedropper",
        "tool.text": "text-t",
        "tool.shape": "shapes",
        "tool.pan": "hand",
        "tool.zoom": "magnifying-glass"
    })

    function toolIconStem(iconKey) {
        return root.toolIconStemMap[iconKey] || iconKey
    }

    function shortcutForAction(actionId) {
        return root.actionShortcutMap[actionId] || ""
    }

    function refreshShortcutYield() {
        var item = root.activeFocusItem
        var yieldKeys = false
        if (item) {
            // TextInput covers TextField; TextEdit covers multiline editors.
            yieldKeys = (item instanceof TextInput) || (item instanceof TextEdit)
        }
        AppSession.setShortcutInputYield(yieldKeys || AppSession.preferencesOpen)
    }

    onActiveFocusItemChanged: root.refreshShortcutYield()

    function actionsForMenu(menuName) {
        var out = []
        var all = root.actionDescriptors
        for (var i = 0; i < all.length; ++i) {
            if (all[i].menu === menuName)
                out.push(all[i])
        }
        return out
    }

    function actionsForContext(ctxName) {
        var out = []
        var all = root.actionDescriptors
        for (var i = 0; i < all.length; ++i) {
            var ctxs = all[i].contexts || []
            for (var j = 0; j < ctxs.length; ++j) {
                if (ctxs[j] === ctxName) {
                    out.push(all[i])
                    break
                }
            }
        }
        return out
    }

    function actionBindingDeps() {
        // Touch session props so MenuItem.enabled bindings refresh.
        return AppSession.canUndo
                + AppSession.canRedo
                + AppSession.hasDocument
                + AppSession.ioBusy
                + AppSession.selectionActive
                + AppSession.layerCount
                + root.activeLayerHasMask
                + AppSession.activeLayerIndex
                + AppSession.prefShowGuides
                + AppSession.prefShowGrid
                + AppSession.prefShowRulers
                + AppSession.prefSnap
                + AppSession.panelNavigatorVisible
                + AppSession.panelSwatchesVisible
                + AppSession.panelLayersVisible
                + AppSession.panelHistoryVisible
                + AppSession.panelPropertiesVisible
                + root.activeLayerClips
                + root.activeMaskEnabled
    }

    function actionIsEnabled(actionId) {
        var _ = root.actionBindingDeps()
        return AppSession.actionEnabled(actionId)
    }

    function actionIsCheckable(actionId) {
        return actionId.indexOf("action.view.toggle-") === 0
                || actionId.indexOf("action.window.panel-") === 0
                || actionId === "action.layer.toggle-clip"
    }

    function actionIsChecked(actionId) {
        switch (actionId) {
        case "action.view.toggle-guides":
            return AppSession.prefShowGuides
        case "action.view.toggle-grid":
            return AppSession.prefShowGrid
        case "action.view.toggle-rulers":
            return AppSession.prefShowRulers
        case "action.view.toggle-snap":
            return AppSession.prefSnap
        case "action.window.panel-navigator":
            return AppSession.panelNavigatorVisible
        case "action.window.panel-swatches":
            return AppSession.panelSwatchesVisible
        case "action.window.panel-layers":
            return AppSession.panelLayersVisible
        case "action.window.panel-history":
            return AppSession.panelHistoryVisible
        case "action.window.panel-properties":
            return AppSession.panelPropertiesVisible
        case "action.layer.toggle-clip":
            return root.activeLayerClips
        default:
            return false
        }
    }

    function applyCheckableAction(actionId, checked) {
        switch (actionId) {
        case "action.view.toggle-guides":
            AppSession.setGuidesVisible(checked)
            break
        case "action.view.toggle-grid":
            AppSession.setGridVisible(checked)
            break
        case "action.view.toggle-rulers":
            AppSession.setRulersVisible(checked)
            break
        case "action.view.toggle-snap":
            AppSession.setSnapEnabled(checked)
            break
        case "action.window.panel-navigator":
            AppSession.setPanelNavigatorVisible(checked)
            break
        case "action.window.panel-swatches":
            AppSession.setPanelSwatchesVisible(checked)
            break
        case "action.window.panel-layers":
            AppSession.setPanelLayersVisible(checked)
            break
        case "action.window.panel-history":
            AppSession.setPanelHistoryVisible(checked)
            break
        case "action.window.panel-properties":
            AppSession.setPanelPropertiesVisible(checked)
            break
        case "action.layer.toggle-clip":
            AppSession.setClipsToBelowOnActive(checked)
            break
        default:
            AppSession.invokeAction(actionId)
            break
        }
    }

    function runAction(actionId) {
        AppSession.invokeAction(actionId)
    }

    function handleHostStatusMarker(text) {
        if (!text || text.indexOf("host:") !== 0)
            return false
        if (text === "host:document.new") {
            root.requestDestructiveAction("new")
            return true
        }
        if (text === "host:document.open") {
            root.requestDestructiveAction("open")
            return true
        }
        if (text === "host:document.save_as") {
            saveFileDialog.open()
            return true
        }
        if (text === "host:document.export") {
            exportFileDialog.open()
            return true
        }
        if (text === "host:document.close") {
            root.requestDestructiveAction("close")
            return true
        }
        if (text === "host:app.quit") {
            root.requestDestructiveAction("quit")
            return true
        }
        if (text === "host:help.about") {
            aboutDialog.open()
            return true
        }
        if (text === "host:palette.open") {
            commandPalette.showPalette()
            return true
        }
        return false
    }

    Connections {
        target: AppSession
        function onStatusTextChanged() {
            root.handleHostStatusMarker(AppSession.statusText)
        }
    }

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
    function isLassoTool() {
        return AppSession.activeTool === "tool.select.lasso"
    }
    function syncBlendCombo() {
        if (typeof blendCombo === "undefined" || !blendCombo)
            return
        var id = AppSession.activeBlend
        for (var i = 0; i < blendCombo.model.length; i++) {
            if (blendCombo.model[i].id === id) {
                blendCombo.currentIndex = i
                return
            }
        }
        blendCombo.currentIndex = 0
    }
    function isPolygonTool() {
        return AppSession.activeTool === "tool.select.polygon"
    }
    function selectionPathToSvg(pointsJoined) {
        if (!pointsJoined || pointsJoined.length === 0)
            return ""
        var parts = pointsJoined.split("|")
        if (parts.length < 1)
            return ""
        var d = ""
        for (var i = 0; i < parts.length; ++i) {
            var xy = parts[i].split(",")
            if (xy.length < 2)
                continue
            var sx = root.docToScreenX(Number(xy[0]))
            var sy = root.docToScreenY(Number(xy[1]))
            d += (d.length === 0 ? "M " : " L ") + sx + " " + sy
        }
        return d
    }
    function appendPathPoint(joined, x, y, minDist) {
        var dx = Math.round(x)
        var dy = Math.round(y)
        if (!joined || joined.length === 0)
            return dx + "," + dy
        var parts = joined.split("|")
        var last = parts[parts.length - 1].split(",")
        if (last.length >= 2) {
            var lx = Number(last[0])
            var ly = Number(last[1])
            var dist = Math.hypot(dx - lx, dy - ly)
            if (dist < minDist)
                return joined
        }
        return joined + "|" + dx + "," + dy
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

    Component {
        id: actionMenuItem
        MenuItem {
            required property var modelData
            // Shortcut activation is owned by the Instantiator below (yield-aware).
            // Show the chord in the native shortcut column for display only via Action
            // that shares the sequence but is not used for invoke when Instantiator runs —
            // so leave `shortcut` empty here and append a compact hint to the label.
            text: {
                var sc = root.shortcutForAction(modelData.id)
                return sc.length > 0 ? (modelData.label + "\t" + sc) : modelData.label
            }
            enabled: root.actionIsEnabled(modelData.id)
            checkable: root.actionIsCheckable(modelData.id)
            checked: root.actionIsChecked(modelData.id)
            icon.source: modelData.icon_key ? root.iconUrl(modelData.icon_key) : ""
            onTriggered: {
                if (checkable)
                    root.applyCheckableAction(modelData.id, checked)
                else
                    root.runAction(modelData.id)
            }
        }
    }

    Instantiator {
        model: root.shortcutChordList
        Shortcut {
            required property string modelData
            sequence: modelData
            context: Qt.ApplicationShortcut
            enabled: !AppSession.shortcutInputYield && !AppSession.preferencesOpen
            onActivated: AppSession.handleShortcut(modelData)
        }
    }

    Connections {
        target: AppSession
        function onPreferencesOpenChanged() {
            root.refreshShortcutYield()
        }
    }

    Menu {
        id: layerContextMenu
        property int targetIndex: -1
        Instantiator {
            model: root.actionsForContext("layer")
            delegate: MenuItem {
                required property var modelData
                text: modelData.label
                enabled: root.actionIsEnabled(modelData.id)
                icon.source: modelData.icon_key ? root.iconUrl(modelData.icon_key) : ""
                onTriggered: {
                    if (layerContextMenu.targetIndex >= 0)
                        AppSession.setActiveLayer(layerContextMenu.targetIndex)
                    root.runAction(modelData.id)
                }
            }
            onObjectAdded: (index, object) => layerContextMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => layerContextMenu.removeItem(object)
        }
    }

    Menu {
        id: canvasContextMenu
        Instantiator {
            model: root.actionsForContext("canvas")
            delegate: actionMenuItem
            onObjectAdded: (index, object) => canvasContextMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => canvasContextMenu.removeItem(object)
        }
    }

    Menu {
        id: selectionContextMenu
        Instantiator {
            model: root.actionsForContext("selection")
            delegate: actionMenuItem
            onObjectAdded: (index, object) => selectionContextMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => selectionContextMenu.removeItem(object)
        }
    }

    menuBar: MenuBar {
        Menu {
            id: fileMenu
            title: qsTr("&File")
            Instantiator {
                model: root.actionsForMenu("file")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => fileMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => fileMenu.removeItem(object)
            }
        }
        Menu {
            id: editMenu
            title: qsTr("&Edit")
            Instantiator {
                model: root.actionsForMenu("edit")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => editMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => editMenu.removeItem(object)
            }
        }
        Menu {
            id: selectMenu
            title: qsTr("&Select")
            Instantiator {
                model: root.actionsForMenu("select")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => selectMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => selectMenu.removeItem(object)
            }
        }
        Menu {
            id: imageMenu
            title: qsTr("&Image")
            Instantiator {
                model: root.actionsForMenu("image")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => imageMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => imageMenu.removeItem(object)
            }
        }
        Menu {
            id: layerMenu
            title: qsTr("&Layer")
            Instantiator {
                model: root.actionsForMenu("layer")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => layerMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => layerMenu.removeItem(object)
            }
        }
        Menu {
            id: filterMenu
            title: qsTr("Filte&r")
            Instantiator {
                model: root.actionsForMenu("filter")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => filterMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => filterMenu.removeItem(object)
            }
        }
        Menu {
            id: viewMenu
            title: qsTr("&View")
            Instantiator {
                model: root.actionsForMenu("view")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => viewMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => viewMenu.removeItem(object)
            }
        }
        Menu {
            id: windowMenu
            title: qsTr("&Window")
            Instantiator {
                model: root.actionsForMenu("window")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => windowMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => windowMenu.removeItem(object)
            }
        }
        Menu {
            id: helpMenu
            title: qsTr("&Help")
            Instantiator {
                model: root.actionsForMenu("help")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => helpMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => helpMenu.removeItem(object)
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
                icon.source: root.iconUrl("file-plus")
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                enabled: root.actionIsEnabled("action.file.new")
                onClicked: root.runAction("action.file.new")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("New…")
            }

            ToolButton {
                icon.source: root.iconUrl("folder-open")
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                enabled: root.actionIsEnabled("action.file.open")
                onClicked: root.runAction("action.file.open")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Open…")
            }

            ToolButton {
                icon.source: root.iconUrl("export")
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                enabled: root.actionIsEnabled("action.file.export")
                onClicked: root.runAction("action.file.export")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Export PNG, JPEG, or PSD subset")
            }

            ToolSeparator {
                contentItem: Rectangle { implicitWidth: 1; color: Theme.border }
            }

            ToolButton {
                icon.source: root.iconUrl("arrow-counter-clockwise")
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                enabled: root.actionIsEnabled("action.edit.undo")
                onClicked: root.runAction("action.edit.undo")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Undo")
            }
            ToolButton {
                icon.source: root.iconUrl("arrow-clockwise")
                display: AbstractButton.IconOnly
                icon.width: 16
                icon.height: 16
                enabled: root.actionIsEnabled("action.edit.redo")
                onClicked: root.runAction("action.edit.redo")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Redo")
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
                text: AppSession.statusText
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
                    model: root.toolDescriptors
                    delegate: Item {
                        width: toolColumn.width
                        height: Theme.toolHit
                        readonly property string toolId: modelData.id
                        readonly property string toolGroup: modelData.group || ""
                        readonly property string prevGroup: index > 0
                                                           ? root.toolDescriptors[index - 1].group
                                                           : ""

                        Rectangle {
                            visible: index > 0 && toolGroup !== prevGroup
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            width: parent.width - 8
                            height: 1
                            y: -Theme.spaceXs / 2
                            color: Theme.border
                        }

                        Rectangle {
                            anchors.fill: parent
                            anchors.leftMargin: 2
                            anchors.rightMargin: 2
                            radius: Theme.radiusSm
                            color: AppSession.activeTool === toolId
                                   ? Theme.toolActiveBg : (toolHover.hovered ? Theme.surfaceContainerHigh : "transparent")

                            Rectangle {
                                visible: AppSession.activeTool === toolId
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.bottom: parent.bottom
                                width: 2
                                color: Theme.primary
                            }

                            Image {
                                anchors.centerIn: parent
                                source: root.iconUrl(root.toolIconStem(modelData.icon_key))
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
                                            && toolId !== "tool.transform")
                                        AppSession.cancelTransform()
                                    if (AppSession.cropPreviewActive
                                            && toolId !== "tool.crop")
                                        AppSession.cancelCrop()
                                    AppSession.setActiveTool(toolId)
                                    if (toolId === "tool.transform")
                                        AppSession.beginTransform()
                                }
                                ToolTip.visible: containsMouse
                                ToolTip.text: modelData.title
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
                selectionAnts: AppSession.selectionActive
                                && AppSession.selectionShape === "mask"
            }

            // Document grid overlay
            Canvas {
                id: gridOverlay
                anchors.fill: parent
                z: 2
                visible: AppSession.hasDocument && AppSession.prefShowGrid
                onPaint: {
                    var ctx = getContext("2d")
                    ctx.reset()
                    if (!visible)
                        return
                    var spacing = Math.max(4, AppSession.gridSpacing)
                    var zoom = Math.max(0.001, AppSession.zoom)
                    var step = spacing * zoom
                    if (step < 4)
                        return
                    ctx.strokeStyle = "#40FFFFFF"
                    ctx.lineWidth = 1
                    var x0 = root.docToScreenX(0)
                    var y0 = root.docToScreenY(0)
                    var x1 = root.docToScreenX(AppSession.docWidth)
                    var y1 = root.docToScreenY(AppSession.docHeight)
                    ctx.beginPath()
                    for (var x = x0; x <= x1 + 0.5; x += step) {
                        ctx.moveTo(x, y0)
                        ctx.lineTo(x, y1)
                    }
                    for (var y = y0; y <= y1 + 0.5; y += step) {
                        ctx.moveTo(x0, y)
                        ctx.lineTo(x1, y)
                    }
                    ctx.stroke()
                }
                Connections {
                    target: AppSession
                    function onZoomChanged() { gridOverlay.requestPaint() }
                    function onPanXChanged() { gridOverlay.requestPaint() }
                    function onPanYChanged() { gridOverlay.requestPaint() }
                    function onPrefShowGridChanged() { gridOverlay.requestPaint() }
                    function onGridSpacingChanged() { gridOverlay.requestPaint() }
                    function onDocWidthChanged() { gridOverlay.requestPaint() }
                    function onDocHeightChanged() { gridOverlay.requestPaint() }
                }
            }

            // Guide lines overlay
            Repeater {
                model: AppSession.prefShowGuides ? root.guidesModel : []
                delegate: Rectangle {
                    required property var modelData
                    z: 3
                    color: "#E0FF6A00"
                    visible: AppSession.hasDocument
                    x: modelData.o === "v" ? root.docToScreenX(modelData.p) : 0
                    y: modelData.o === "h" ? root.docToScreenY(modelData.p) : 0
                    width: modelData.o === "v" ? 1 : canvasHost.width
                    height: modelData.o === "h" ? 1 : canvasHost.height
                }
            }

            // Live text preview (editable text layers before bake)
            Text {
                id: textPreview
                z: 3
                visible: AppSession.hasDocument && AppSession.textLayerActive
                x: root.docToScreenX(4)
                y: root.docToScreenY(4)
                width: Math.max(8, (AppSession.docWidth - 8) * AppSession.zoom)
                text: AppSession.textBody
                color: AppSession.textColorHex
                font.family: AppSession.textFontFamily
                font.pixelSize: Math.max(6, AppSession.textFontSize * AppSession.zoom)
                lineHeight: AppSession.textLineSpacing
                lineHeightMode: Text.ProportionalHeight
                horizontalAlignment: AppSession.textAlignment === 1
                                     ? Text.AlignHCenter
                                     : (AppSession.textAlignment === 2
                                        ? Text.AlignRight : Text.AlignLeft)
                wrapMode: Text.Wrap
            }

            // Rulers
            Rectangle {
                id: rulerTop
                z: 6
                visible: AppSession.hasDocument && AppSession.prefShowRulers
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 18
                color: Theme.surfaceRaised
                opacity: 0.92
                Canvas {
                    id: rulerTopCanvas
                    anchors.fill: parent
                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.reset()
                        ctx.fillStyle = Theme.colorOnSurfaceMuted
                        ctx.font = "10px sans-serif"
                        var zoom = Math.max(0.001, AppSession.zoom)
                        var step = 50
                        while (step * zoom < 40)
                            step *= 2
                        for (var d = 0; d <= AppSession.docWidth; d += step) {
                            var sx = root.docToScreenX(d)
                            ctx.fillRect(sx, 10, 1, 8)
                            ctx.fillText(String(d), sx + 2, 10)
                        }
                    }
                    Connections {
                        target: AppSession
                        function onZoomChanged() { rulerTopCanvas.requestPaint() }
                        function onPanXChanged() { rulerTopCanvas.requestPaint() }
                        function onPrefShowRulersChanged() { rulerTopCanvas.requestPaint() }
                    }
                }
            }
            Rectangle {
                id: rulerLeft
                z: 6
                visible: AppSession.hasDocument && AppSession.prefShowRulers
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.topMargin: AppSession.prefShowRulers ? 18 : 0
                width: 18
                color: Theme.surfaceRaised
                opacity: 0.92
                Canvas {
                    id: rulerLeftCanvas
                    anchors.fill: parent
                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.reset()
                        ctx.fillStyle = Theme.colorOnSurfaceMuted
                        ctx.font = "10px sans-serif"
                        var zoom = Math.max(0.001, AppSession.zoom)
                        var step = 50
                        while (step * zoom < 40)
                            step *= 2
                        for (var d = 0; d <= AppSession.docHeight; d += step) {
                            var sy = root.docToScreenY(d)
                            ctx.fillRect(10, sy, 8, 1)
                            ctx.save()
                            ctx.translate(2, sy + 2)
                            ctx.rotate(-Math.PI / 2)
                            ctx.fillText(String(d), 0, 0)
                            ctx.restore()
                        }
                    }
                    Connections {
                        target: AppSession
                        function onZoomChanged() { rulerLeftCanvas.requestPaint() }
                        function onPanYChanged() { rulerLeftCanvas.requestPaint() }
                        function onPrefShowRulersChanged() { rulerLeftCanvas.requestPaint() }
                    }
                }
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

            // Live lasso / polygonal path preview
            Shape {
                id: selectionPathPreview
                anchors.fill: parent
                z: 4
                visible: AppSession.selectionPathActive && AppSession.hasDocument
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeWidth: 1
                    strokeColor: root.primary
                    fillColor: "transparent"
                    strokeStyle: ShapePath.DashLine
                    dashPattern: [4, 4]
                    PathSvg {
                        path: root.selectionPathToSvg(AppSession.selectionPath)
                    }
                }
            }

            // Linear gradient drag preview
            Shape {
                id: gradientPreview
                anchors.fill: parent
                z: 4
                visible: canvasInput.gradienting && AppSession.hasDocument
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeWidth: 2
                    strokeColor: root.primary
                    fillColor: "transparent"
                    startX: root.docToScreenX(canvasInput.gradStartX)
                    startY: root.docToScreenY(canvasInput.gradStartY)
                    PathLine {
                        x: root.docToScreenX(canvasInput.gradEndX)
                        y: root.docToScreenY(canvasInput.gradEndY)
                    }
                }
            }

            // Marching ants for committed rect/ellipse (mask shape uses GPU ants)
            Item {
                id: selectionAnts
                visible: AppSession.selectionActive && AppSession.hasDocument
                         && AppSession.selectionW > 0 && AppSession.selectionH > 0
                         && AppSession.selectionShape !== "mask"
                z: 5
                x: root.docToScreenX(AppSession.selectionX)
                y: root.docToScreenY(AppSession.selectionY)
                width: Math.max(1, AppSession.selectionW * AppSession.zoom)
                height: Math.max(1, AppSession.selectionH * AppSession.zoom)

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.RightButton
                    onClicked: selectionContextMenu.popup()
                }

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
                            selectionContextMenu.popup()
                        else
                            canvasContextMenu.popup()
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
                        // pressure: tablet via mouse.pressure if available, else 1
                        var p = (typeof mouse.pressure === "number" && mouse.pressure > 0)
                                ? mouse.pressure : 1.0
                        AppSession.strokeBegin(mouse.x, mouse.y, p)
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
                    dragging = false
                    if (AppSession.activeTool === "tool.pan")
                        cursorShape = Qt.OpenHandCursor
                }
                onPositionChanged: function (mouse) {
                    if (!AppSession.hasDocument)
                        return
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
                    wheel.accepted = true
                }
                onDoubleClicked: function (mouse) {
                    if (AppSession.hasDocument)
                        AppSession.zoomToFit()
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

            GridLayout {
                anchors.fill: parent
                columns: 1
                rowSpacing: 0
                columnSpacing: 0

                // Properties panel header
                Rectangle {
                    visible: AppSession.panelPropertiesVisible
                    Layout.row: root.dockStackRow("panel.properties") * 2
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? Theme.panelHeaderHeight : 0
                    color: Theme.surfaceContainer
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
                        Label {
                            text: qsTr(root.panelTitle("panel.properties"))
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.Medium
                            Layout.fillWidth: true
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↑"
                            enabled: root.dockStackRow("panel.properties") > 0
                            onClicked: AppSession.movePanelInStack("panel.properties", -1)
                            Accessible.name: qsTr("Move panel up")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↓"
                            enabled: root.dockStackRow("panel.properties") >= 0
                                     && root.dockStackRow("panel.properties") < root.dockRightStack.length - 1
                            onClicked: AppSession.movePanelInStack("panel.properties", 1)
                            Accessible.name: qsTr("Move panel down")
                        }
                    }
                }

                Flickable {
                    visible: AppSession.panelPropertiesVisible
                    Layout.row: root.dockStackRow("panel.properties") * 2 + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? parent.height * 0.52 : 0
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

                        // Edit target + selection context (distinct chrome)
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: AppSession.hasDocument
                            Label {
                                text: qsTr("Edit target")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            Label {
                                text: {
                                    var kind = AppSession.activeLayerKind.length > 0
                                               ? AppSession.activeLayerKind
                                               : qsTr("layer")
                                    var sel = AppSession.pixelSelectionActive
                                              ? qsTr("pixel selection active")
                                              : qsTr("no pixel selection")
                                    return qsTr("%1 · %2 · %3")
                                           .arg(kind)
                                           .arg(AppSession.editTargetLabel)
                                           .arg(sel)
                                }
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceXs
                                Button {
                                    text: qsTr("Layer pixels")
                                    checkable: true
                                    checked: AppSession.editTarget === "layer"
                                    enabled: AppSession.hasDocument
                                    Layout.fillWidth: true
                                    onClicked: AppSession.setMaskEditTarget(false)
                                }
                                Button {
                                    text: qsTr("Layer mask")
                                    checkable: true
                                    checked: AppSession.editTarget === "mask"
                                    enabled: AppSession.hasDocument && root.activeLayerHasMask
                                    Layout.fillWidth: true
                                    onClicked: AppSession.setMaskEditTarget(true)
                                }
                            }
                        }

                        // Selection combine modes
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: root.isSelectTool()
                            Label {
                                text: qsTr("Pixel selection")
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

                        // Character / text layer chrome
                        ColumnLayout {
                            id: characterProps
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: AppSession.textLayerActive
                                     || AppSession.activeTool === "tool.text"
                                     || root.activeLayerKind === "text"

                            function pushText() {
                                AppSession.updateActiveText(
                                            textBodyField.text,
                                            fontFamilyCombo.currentText,
                                            fontSizeSpin.value,
                                            trackingSpin.value,
                                            lineSpacingSpin.value / 100.0,
                                            alignCombo.currentIndex,
                                            textColorField.text)
                            }

                            Label {
                                text: qsTr("Character")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            Label {
                                visible: !AppSession.textLayerActive
                                text: qsTr("Click the canvas with the Text tool to create a text layer.")
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                                wrapMode: Text.Wrap
                                Layout.fillWidth: true
                            }
                            TextField {
                                id: textBodyField
                                Layout.fillWidth: true
                                enabled: AppSession.textLayerActive
                                text: AppSession.textBody
                                placeholderText: qsTr("Text")
                                onEditingFinished: characterProps.pushText()
                            }
                            ComboBox {
                                id: fontFamilyCombo
                                Layout.fillWidth: true
                                enabled: AppSession.textLayerActive
                                model: ["Noto Sans", "Noto Sans Mono", "DejaVu Sans"]
                                Component.onCompleted: {
                                    var i = model.indexOf(AppSession.textFontFamily)
                                    currentIndex = i >= 0 ? i : 0
                                }
                                onActivated: characterProps.pushText()
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Size")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                }
                                SpinBox {
                                    id: fontSizeSpin
                                    from: 4
                                    to: 512
                                    value: Math.round(AppSession.textFontSize)
                                    enabled: AppSession.textLayerActive
                                    onValueModified: characterProps.pushText()
                                }
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Tracking")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                }
                                SpinBox {
                                    id: trackingSpin
                                    from: -20
                                    to: 40
                                    value: Math.round(AppSession.textTracking)
                                    enabled: AppSession.textLayerActive
                                    onValueModified: characterProps.pushText()
                                }
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Leading")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                }
                                SpinBox {
                                    id: lineSpacingSpin
                                    from: 50
                                    to: 400
                                    value: Math.round(AppSession.textLineSpacing * 100)
                                    enabled: AppSession.textLayerActive
                                    textFromValue: function (v) { return (v / 100).toFixed(2) }
                                    valueFromText: function (t) { return Math.round(parseFloat(t) * 100) }
                                    onValueModified: {
                                        // push uses /100 via custom path
                                        AppSession.updateActiveText(
                                                    textBodyField.text,
                                                    fontFamilyCombo.currentText,
                                                    fontSizeSpin.value,
                                                    trackingSpin.value,
                                                    lineSpacingSpin.value / 100.0,
                                                    alignCombo.currentIndex,
                                                    textColorField.text)
                                    }
                                }
                            }
                            ComboBox {
                                id: alignCombo
                                Layout.fillWidth: true
                                enabled: AppSession.textLayerActive
                                model: [qsTr("Left"), qsTr("Center"), qsTr("Right")]
                                currentIndex: AppSession.textAlignment
                                onActivated: characterProps.pushText()
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Color")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                }
                                TextField {
                                    id: textColorField
                                    Layout.fillWidth: true
                                    enabled: AppSession.textLayerActive
                                    text: AppSession.textColorHex
                                    onEditingFinished: characterProps.pushText()
                                }
                            }
                            Button {
                                text: qsTr("Bake Text")
                                enabled: AppSession.textLayerActive && !AppSession.ioBusy
                                onClicked: AppSession.bakeTextLayer()
                            }
                            Connections {
                                target: AppSession
                                function onTextBodyChanged() {
                                    if (!textBodyField.activeFocus)
                                        textBodyField.text = AppSession.textBody
                                }
                                function onTextFontSizeChanged() {
                                    fontSizeSpin.value = Math.round(AppSession.textFontSize)
                                }
                                function onTextColorHexChanged() {
                                    if (!textColorField.activeFocus)
                                        textColorField.text = AppSession.textColorHex
                                }
                                function onTextAlignmentChanged() {
                                    alignCombo.currentIndex = AppSession.textAlignment
                                }
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

                        // Adjustment layer params
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: AppSession.adjustmentKind === "brightness"
                                     || AppSession.adjustmentKind === "levels"
                            Label {
                                text: AppSession.adjustmentKind === "levels"
                                      ? qsTr("Levels") : qsTr("Brightness/Contrast")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "brightness"
                                Label {
                                    text: qsTr("Brightness")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(AppSession.adjustmentP0 * 100)
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "brightness"
                                from: -1
                                to: 1
                                value: AppSession.adjustmentP0
                                onMoved: AppSession.setAdjustmentParams(
                                             value, AppSession.adjustmentP1, 0)
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "brightness"
                                Label {
                                    text: qsTr("Contrast")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(AppSession.adjustmentP1 * 100)
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "brightness"
                                from: -1
                                to: 1
                                value: AppSession.adjustmentP1
                                onMoved: AppSession.setAdjustmentParams(
                                             AppSession.adjustmentP0, value, 0)
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "levels"
                                Label {
                                    text: qsTr("Black")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(AppSession.adjustmentP0 * 255)
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "levels"
                                from: 0
                                to: 1
                                value: AppSession.adjustmentP0
                                onMoved: AppSession.setAdjustmentParams(
                                             value, AppSession.adjustmentP1, AppSession.adjustmentP2)
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "levels"
                                Label {
                                    text: qsTr("White")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: Math.round(AppSession.adjustmentP1 * 255)
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "levels"
                                from: 0
                                to: 1
                                value: AppSession.adjustmentP1
                                onMoved: AppSession.setAdjustmentParams(
                                             AppSession.adjustmentP0, value, AppSession.adjustmentP2)
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "levels"
                                Label {
                                    text: qsTr("Gamma")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: AppSession.adjustmentP2.toFixed(2)
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                Layout.fillWidth: true
                                visible: AppSession.adjustmentKind === "levels"
                                from: 0.1
                                to: 3
                                value: AppSession.adjustmentP2
                                onMoved: AppSession.setAdjustmentParams(
                                             AppSession.adjustmentP0, AppSession.adjustmentP1, value)
                            }
                        }

                        // Gaussian blur effect
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: AppSession.hasGaussianBlur
                            Label {
                                text: qsTr("Gaussian Blur")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: qsTr("Radius")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: AppSession.gaussianRadius.toFixed(1) + " px"
                                    color: Theme.primary
                                    font.pixelSize: Theme.fontMono
                                    font.family: "Noto Sans Mono"
                                }
                            }
                            Slider {
                                Layout.fillWidth: true
                                from: 0
                                to: 64
                                value: AppSession.gaussianRadius
                                onMoved: AppSession.setGaussianRadius(value)
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
                            visible: AppSession.activeTool === "tool.fill"
                                     || AppSession.activeTool === "tool.gradient"
                                     || AppSession.activeTool === "tool.eyedropper"
                            Label {
                                text: AppSession.activeTool === "tool.gradient"
                                      ? qsTr("Gradient (Linear)")
                                      : (AppSession.activeTool === "tool.eyedropper"
                                         ? qsTr("Eyedropper")
                                         : qsTr("Paint Bucket"))
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            Label {
                                text: AppSession.activeTool === "tool.gradient"
                                      ? qsTr("Drag FG→BG. Respects selection.")
                                      : (AppSession.activeTool === "tool.eyedropper"
                                         ? qsTr("Click canvas to sample foreground.")
                                         : qsTr("Click to fill with FG. Respects selection."))
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            visible: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
                            Label {
                                text: qsTr("Blend Mode")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            ComboBox {
                                id: blendCombo
                                Layout.fillWidth: true
                                model: [
                                    { label: qsTr("Normal"), id: "normal" },
                                    { label: qsTr("Multiply"), id: "multiply" },
                                    { label: qsTr("Screen"), id: "screen" },
                                    { label: qsTr("Overlay"), id: "overlay" },
                                    { label: qsTr("Soft Light"), id: "soft_light" },
                                    { label: qsTr("Hard Light"), id: "hard_light" },
                                    { label: qsTr("Darken"), id: "darken" },
                                    { label: qsTr("Lighten"), id: "lighten" }
                                ]
                                textRole: "label"
                                valueRole: "id"
                                enabled: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
                                Component.onCompleted: root.syncBlendCombo()
                                onActivated: AppSession.setActiveBlend(currentValue)
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            Label {
                                text: qsTr("Foreground")
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
                                        onMoved: AppSession.setForegroundRgb(value, colorG.value, colorB.value)
                                    }
                                    Slider {
                                        id: colorG
                                        Layout.fillWidth: true
                                        from: 0; to: 1
                                        value: AppSession.brushG
                                        enabled: AppSession.hasDocument
                                        onMoved: AppSession.setForegroundRgb(colorR.value, value, colorB.value)
                                    }
                                    Slider {
                                        id: colorB
                                        Layout.fillWidth: true
                                        from: 0; to: 1
                                        value: AppSession.brushB
                                        enabled: AppSession.hasDocument
                                        onMoved: AppSession.setForegroundRgb(colorR.value, colorG.value, value)
                                    }
                                }
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            Label {
                                text: qsTr("View")
                                color: Theme.colorOnSurface
                                font.pixelSize: Theme.fontBodySm
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceSm
                                Button {
                                    text: qsTr("Fit")
                                    Layout.fillWidth: true
                                    enabled: AppSession.hasDocument
                                    onClicked: AppSession.zoomToFit()
                                }
                                Button {
                                    text: qsTr("100%")
                                    Layout.fillWidth: true
                                    enabled: AppSession.hasDocument
                                    onClicked: AppSession.setZoom(1.0)
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

                // Navigator
                Rectangle {
                    visible: AppSession.panelNavigatorVisible
                    Layout.row: root.dockStackRow("panel.navigator") * 2
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? Theme.panelHeaderHeight : 0
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
                        Label {
                            text: qsTr(root.panelTitle("panel.navigator"))
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.Medium
                            Layout.fillWidth: true
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↑"
                            enabled: root.dockStackRow("panel.navigator") > 0
                            onClicked: AppSession.movePanelInStack("panel.navigator", -1)
                            Accessible.name: qsTr("Move panel up")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↓"
                            enabled: root.dockStackRow("panel.navigator") >= 0
                                     && root.dockStackRow("panel.navigator") < root.dockRightStack.length - 1
                            onClicked: AppSession.movePanelInStack("panel.navigator", 1)
                            Accessible.name: qsTr("Move panel down")
                        }
                    }
                }

                Rectangle {
                    id: navigatorPane
                    visible: AppSession.panelNavigatorVisible
                    Layout.row: root.dockStackRow("panel.navigator") * 2 + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? 132 : 0
                    color: Theme.surfaceSunken
                    clip: true

                    readonly property real pad: Theme.spaceSm
                    readonly property real docW: Math.max(1, AppSession.docWidth)
                    readonly property real docH: Math.max(1, AppSession.docHeight)
                    readonly property real availW: width - pad * 2
                    readonly property real availH: height - pad * 2
                    readonly property real scale: Math.min(availW / docW, availH / docH)
                    readonly property real frameW: docW * scale
                    readonly property real frameH: docH * scale
                    readonly property real frameX: (width - frameW) / 2
                    readonly property real frameY: (height - frameH) / 2
                    readonly property real viewW: Math.max(8, AppSession.viewportWidth / Math.max(0.001, AppSession.zoom) * scale)
                    readonly property real viewH: Math.max(8, AppSession.viewportHeight / Math.max(0.001, AppSession.zoom) * scale)
                    readonly property real viewX: frameX + (AppSession.panX - viewW / (2 * scale)) * scale
                    readonly property real viewY: frameY + (AppSession.panY - viewH / (2 * scale)) * scale

                    function panToLocal(lx, ly) {
                        if (!AppSession.hasDocument || frameW < 1 || frameH < 1)
                            return
                        var docX = ((lx - frameX) / frameW) * docW
                        var docY = ((ly - frameY) / frameH) * docH
                        AppSession.centerViewOn(docX, docY)
                    }

                    // Checkerboard backdrop
                    Canvas {
                        anchors.fill: parent
                        onPaint: {
                            var ctx = getContext("2d")
                            var s = 8
                            for (var y = 0; y < height; y += s) {
                                for (var x = 0; x < width; x += s) {
                                    ctx.fillStyle = ((x / s + y / s) % 2 === 0) ? "#2a2a2e" : "#222226"
                                    ctx.fillRect(x, y, s, s)
                                }
                            }
                        }
                        Component.onCompleted: requestPaint()
                        onWidthChanged: requestPaint()
                        onHeightChanged: requestPaint()
                    }

                    Rectangle {
                        x: navigatorPane.frameX
                        y: navigatorPane.frameY
                        width: navigatorPane.frameW
                        height: navigatorPane.frameH
                        color: Theme.surfaceContainerHigh
                        border.color: Theme.border
                        border.width: 1
                        opacity: AppSession.hasDocument ? 1 : 0.35
                    }

                    Rectangle {
                        visible: AppSession.hasDocument
                        x: Math.max(navigatorPane.frameX,
                                    Math.min(navigatorPane.viewX,
                                             navigatorPane.frameX + navigatorPane.frameW - width))
                        y: Math.max(navigatorPane.frameY,
                                    Math.min(navigatorPane.viewY,
                                             navigatorPane.frameY + navigatorPane.frameH - height))
                        width: Math.min(navigatorPane.viewW, navigatorPane.frameW)
                        height: Math.min(navigatorPane.viewH, navigatorPane.frameH)
                        color: "transparent"
                        border.color: Theme.primary
                        border.width: 1.5
                    }

                    MouseArea {
                        anchors.fill: parent
                        enabled: AppSession.hasDocument
                        cursorShape: Qt.OpenHandCursor
                        onPressed: function (mouse) {
                            navigatorPane.panToLocal(mouse.x, mouse.y)
                        }
                        onPositionChanged: function (mouse) {
                            if (pressed)
                                navigatorPane.panToLocal(mouse.x, mouse.y)
                        }
                    }
                }

                // Swatches
                Rectangle {
                    visible: AppSession.panelSwatchesVisible
                    Layout.row: root.dockStackRow("panel.swatches") * 2
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? Theme.panelHeaderHeight : 0
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
                        Label {
                            text: qsTr(root.panelTitle("panel.swatches"))
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.Medium
                            Layout.fillWidth: true
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↑"
                            enabled: root.dockStackRow("panel.swatches") > 0
                            onClicked: AppSession.movePanelInStack("panel.swatches", -1)
                            Accessible.name: qsTr("Move panel up")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↓"
                            enabled: root.dockStackRow("panel.swatches") >= 0
                                     && root.dockStackRow("panel.swatches") < root.dockRightStack.length - 1
                            onClicked: AppSession.movePanelInStack("panel.swatches", 1)
                            Accessible.name: qsTr("Move panel down")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            icon.source: root.iconUrl("arrows-left-right")
                            icon.width: 14
                            icon.height: 14
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Swap foreground / background")
                            onClicked: AppSession.swapFgBg()
                        }
                    }
                }

                Rectangle {
                    visible: AppSession.panelSwatchesVisible
                    Layout.row: root.dockStackRow("panel.swatches") * 2 + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? (swatchesCol.implicitHeight + Theme.spaceMd * 2) : 0
                    color: Theme.surfaceSunken

                    ColumnLayout {
                        id: swatchesCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: Theme.spaceMd
                        spacing: Theme.spaceSm

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceMd

                            Item {
                                width: 44
                                height: 36
                                Rectangle {
                                    x: 12
                                    y: 10
                                    width: 26
                                    height: 26
                                    radius: Theme.radiusSm
                                    color: AppSession.backgroundHex
                                    border.color: Theme.border
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: AppSession.swapFgBg()
                                        ToolTip.visible: containsMouse
                                        ToolTip.text: qsTr("Background (click to swap)")
                                        hoverEnabled: true
                                    }
                                }
                                Rectangle {
                                    x: 0
                                    y: 0
                                    width: 26
                                    height: 26
                                    radius: Theme.radiusSm
                                    color: AppSession.foregroundHex
                                    border.color: Theme.primary
                                    border.width: 1
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: hexField.forceActiveFocus()
                                        ToolTip.visible: containsMouse
                                        ToolTip.text: qsTr("Foreground")
                                        hoverEnabled: true
                                    }
                                }
                            }

                            TextField {
                                id: hexField
                                Layout.fillWidth: true
                                text: AppSession.foregroundHex
                                selectByMouse: true
                                font.family: "Noto Sans Mono"
                                font.pixelSize: Theme.fontMono
                                color: Theme.colorOnSurface
                                background: Rectangle {
                                    color: Theme.surfaceContainer
                                    border.color: parent.activeFocus ? Theme.primary : Theme.border
                                    radius: Theme.radiusSm
                                }
                                onEditingFinished: AppSession.setForegroundHex(text)
                                Keys.onReturnPressed: {
                                    AppSession.setForegroundHex(text)
                                    event.accepted = true
                                }
                            }
                        }

                        Flow {
                            Layout.fillWidth: true
                            spacing: 4
                            Repeater {
                                model: [
                                    "#000000", "#FFFFFF", "#FF0000", "#00FF00",
                                    "#0000FF", "#FFFF00", "#FF00FF", "#00FFFF",
                                    "#808080", "#C0C0C0", "#800000", "#008080"
                                ]
                                delegate: Rectangle {
                                    width: 18
                                    height: 18
                                    radius: 2
                                    color: modelData
                                    border.color: Theme.border
                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: AppSession.setForegroundHex(modelData)
                                    }
                                }
                            }
                        }

                        Flow {
                            Layout.fillWidth: true
                            spacing: 4
                            visible: AppSession.recentColors.length > 0
                            Repeater {
                                model: AppSession.recentColors.length > 0
                                       ? AppSession.recentColors.split("|") : []
                                delegate: Rectangle {
                                    width: 18
                                    height: 18
                                    radius: 2
                                    color: modelData
                                    border.color: Theme.border
                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: AppSession.pickRecentColor(index)
                                    }
                                }
                            }
                        }
                    }
                }

                // Layers panel
                Rectangle {
                    visible: AppSession.panelLayersVisible
                    Layout.row: root.dockStackRow("panel.layers") * 2
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? Theme.panelHeaderHeight : 0
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
                            text: qsTr(root.panelTitle("panel.layers"))
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.Medium
                            Layout.fillWidth: true
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↑"
                            enabled: root.dockStackRow("panel.layers") > 0
                            onClicked: AppSession.movePanelInStack("panel.layers", -1)
                            Accessible.name: qsTr("Move panel up")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↓"
                            enabled: root.dockStackRow("panel.layers") >= 0
                                     && root.dockStackRow("panel.layers") < root.dockRightStack.length - 1
                            onClicked: AppSession.movePanelInStack("panel.layers", 1)
                            Accessible.name: qsTr("Move panel down")
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
                    visible: AppSession.panelLayersVisible
                    Layout.row: root.dockStackRow("panel.layers") * 2 + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.fillHeight: visible
                    Layout.preferredHeight: visible ? 180 : 0
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
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                onClicked: function (mouse) {
                                    AppSession.setActiveLayer(stackIndex)
                                    if (mouse.button === Qt.RightButton) {
                                        layerContextMenu.targetIndex = stackIndex
                                        layerContextMenu.popup()
                                    }
                                }
                                onPressAndHold: {
                                    layerContextMenu.targetIndex = stackIndex
                                    layerContextMenu.popup()
                                }
                            }
                        }
                    }
                }

                // History panel
                Rectangle {
                    visible: AppSession.panelHistoryVisible
                    Layout.row: root.dockStackRow("panel.history") * 2
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? Theme.panelHeaderHeight : 0
                    color: Theme.surfaceContainer
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceSm
                        anchors.rightMargin: Theme.spaceXs
                        Label {
                            text: qsTr(root.panelTitle("panel.history"))
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.Medium
                            Layout.fillWidth: true
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↑"
                            enabled: root.dockStackRow("panel.history") > 0
                            onClicked: AppSession.movePanelInStack("panel.history", -1)
                            Accessible.name: qsTr("Move panel up")
                        }
                        ToolButton {
                            implicitWidth: 22
                            implicitHeight: 22
                            text: "↓"
                            enabled: root.dockStackRow("panel.history") >= 0
                                     && root.dockStackRow("panel.history") < root.dockRightStack.length - 1
                            onClicked: AppSession.movePanelInStack("panel.history", 1)
                            Accessible.name: qsTr("Move panel down")
                        }
                    }
                }
                Rectangle {
                    visible: AppSession.panelHistoryVisible
                    Layout.row: root.dockStackRow("panel.history") * 2 + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? 120 : 0
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

                // Placeholder slots for descriptor panels without a body yet.
                Repeater {
                    model: {
                        var _ = AppSession.panelVisibilityJson
                        var _t = AppSession.dockTopologyJson
                        var stack = root.dockRightStack
                        var all = root.panelDescriptors
                        var out = []
                        var seen = ({})
                        for (var s = 0; s < stack.length; ++s) {
                            var id = stack[s]
                            if (root.panelIsVisible(id) && !root.panelHasBody(id)) {
                                out.push(id)
                                seen[id] = true
                            }
                        }
                        for (var i = 0; i < all.length; ++i) {
                            var pid = all[i].id
                            if (!seen[pid] && root.panelIsVisible(pid) && !root.panelHasBody(pid))
                                out.push(pid)
                        }
                        return out
                    }
                    delegate: ColumnLayout {
                        required property string modelData
                        Layout.row: root.dockStackRow(modelData) * 2
                        Layout.column: 0
                        Layout.fillWidth: true
                        spacing: 0
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: Theme.panelHeaderHeight
                            color: Theme.surfaceContainer
                            Label {
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.left: parent.left
                                anchors.leftMargin: Theme.spaceSm
                                text: qsTr(root.panelTitle(modelData))
                                color: Theme.colorOnSurfaceVariant
                                font.pixelSize: Theme.fontLabel
                                font.weight: Font.Medium
                            }
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 48
                            color: Theme.surfaceSunken
                            Label {
                                anchors.centerIn: parent
                                text: qsTr("Coming soon")
                                color: Theme.colorOnSurfaceVariant
                                font.pixelSize: Theme.fontBodySm
                                opacity: 0.7
                            }
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
            qsTr("TIFF images (*.tif *.tiff)"),
            qsTr("Photoshop subset (*.psd)")
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
        id: preferencesDialog
        anchors.centerIn: parent
        modal: true
        title: qsTr("Preferences")
        standardButtons: Dialog.Close
        width: 480
        height: 560
        visible: AppSession.preferencesOpen
        onRejected: AppSession.closePreferences()
        onAccepted: AppSession.closePreferences()
        onClosed: {
            preferencesDialog.capturingActionId = ""
            preferencesDialog.shortcutConflictHint = ""
            AppSession.closePreferences()
        }

        property string capturingActionId: ""
        property string shortcutConflictHint: ""

        function chordFromKeyEvent(event) {
            if (event.key === Qt.Key_Escape
                    || event.key === Qt.Key_Tab
                    || event.key === Qt.Key_Backtab
                    || event.key === Qt.Key_Control
                    || event.key === Qt.Key_Shift
                    || event.key === Qt.Key_Alt
                    || event.key === Qt.Key_Meta)
                return ""
            var parts = []
            if (event.modifiers & Qt.ControlModifier)
                parts.push("Ctrl")
            if (event.modifiers & Qt.AltModifier)
                parts.push("Alt")
            if (event.modifiers & Qt.ShiftModifier)
                parts.push("Shift")
            if (event.modifiers & Qt.MetaModifier)
                parts.push("Meta")
            var name = ""
            switch (event.key) {
            case Qt.Key_Comma: name = ","; break
            case Qt.Key_Period: name = "."; break
            case Qt.Key_Slash: name = "/"; break
            case Qt.Key_Space: name = "Space"; break
            case Qt.Key_Return: case Qt.Key_Enter: name = "Return"; break
            case Qt.Key_Left: name = "Left"; break
            case Qt.Key_Right: name = "Right"; break
            case Qt.Key_Up: name = "Up"; break
            case Qt.Key_Down: name = "Down"; break
            case Qt.Key_F1: name = "F1"; break
            case Qt.Key_F2: name = "F2"; break
            case Qt.Key_F3: name = "F3"; break
            case Qt.Key_F4: name = "F4"; break
            case Qt.Key_F5: name = "F5"; break
            case Qt.Key_F6: name = "F6"; break
            case Qt.Key_F7: name = "F7"; break
            case Qt.Key_F8: name = "F8"; break
            case Qt.Key_F9: name = "F9"; break
            case Qt.Key_F10: name = "F10"; break
            case Qt.Key_F11: name = "F11"; break
            case Qt.Key_F12: name = "F12"; break
            default:
                if (event.key >= Qt.Key_A && event.key <= Qt.Key_Z)
                    name = String.fromCharCode(event.key)
                else if (event.key >= Qt.Key_0 && event.key <= Qt.Key_9)
                    name = String.fromCharCode(event.key)
                else
                    return ""
            }
            parts.push(name)
            return parts.join("+")
        }

        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
            radius: Theme.radiusMd
        }

        contentItem: Flickable {
            id: prefsFlick
            clip: true
            contentWidth: prefsCol.width
            contentHeight: prefsCol.implicitHeight
            boundsBehavior: Flickable.StopAtBounds
            focus: preferencesDialog.capturingActionId.length > 0
            Keys.onPressed: function (event) {
                if (preferencesDialog.capturingActionId.length === 0)
                    return
                if (event.key === Qt.Key_Escape) {
                    preferencesDialog.capturingActionId = ""
                    preferencesDialog.shortcutConflictHint = ""
                    event.accepted = true
                    return
                }
                if (event.key === Qt.Key_Backspace || event.key === Qt.Key_Delete) {
                    AppSession.setActionShortcut(preferencesDialog.capturingActionId, "")
                    preferencesDialog.capturingActionId = ""
                    preferencesDialog.shortcutConflictHint = ""
                    event.accepted = true
                    return
                }
                var chord = preferencesDialog.chordFromKeyEvent(event)
                if (!chord)
                    return
                var conflict = AppSession.shortcutConflictFor(
                            preferencesDialog.capturingActionId, chord)
                if (conflict && conflict.length > 0)
                    preferencesDialog.shortcutConflictHint =
                            qsTr("Replaces binding on %1").arg(conflict)
                else
                    preferencesDialog.shortcutConflictHint = ""
                AppSession.setActionShortcut(preferencesDialog.capturingActionId, chord)
                preferencesDialog.capturingActionId = ""
                event.accepted = true
            }

            ColumnLayout {
                id: prefsCol
                spacing: Theme.spaceMd
                width: 440

                Label {
                    text: qsTr("General")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontLabel
                    font.weight: Font.DemiBold
                }
                CheckBox {
                    text: qsTr("Show guides")
                    checked: AppSession.prefShowGuides
                    onToggled: AppSession.setPrefShowGuides(checked)
                }
                CheckBox {
                    text: qsTr("Show grid")
                    checked: AppSession.prefShowGrid
                    onToggled: AppSession.setGridVisible(checked)
                }
                CheckBox {
                    text: qsTr("Show rulers")
                    checked: AppSession.prefShowRulers
                    onToggled: AppSession.setRulersVisible(checked)
                }
                CheckBox {
                    text: qsTr("Snap to grid / guides")
                    checked: AppSession.prefSnap
                    onToggled: AppSession.setSnapEnabled(checked)
                }
                CheckBox {
                    text: qsTr("Restore last tool on launch")
                    checked: AppSession.prefRestoreLastTool
                    onToggled: AppSession.setPrefRestoreLastTool(checked)
                }

                Label {
                    Layout.topMargin: Theme.spaceSm
                    text: qsTr("Workspace panels")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontLabel
                    font.weight: Font.DemiBold
                }
                Repeater {
                    model: {
                        var _ = AppSession.panelVisibilityJson
                        var stack = root.dockRightStack
                        var all = root.panelDescriptors
                        var out = []
                        var seen = ({})
                        for (var s = 0; s < stack.length; ++s) {
                            for (var i = 0; i < all.length; ++i) {
                                if (all[i].id === stack[s]) {
                                    out.push(all[i])
                                    seen[all[i].id] = true
                                    break
                                }
                            }
                        }
                        for (var j = 0; j < all.length; ++j) {
                            if (!seen[all[j].id])
                                out.push(all[j])
                        }
                        return out
                    }
                    delegate: CheckBox {
                        required property var modelData
                        text: qsTr(modelData.title || modelData.id)
                        checked: root.panelIsVisible(modelData.id)
                        onToggled: AppSession.setPanelVisible(modelData.id, checked)
                    }
                }

                Button {
                    text: qsTr("Reset Workspace to Essentials")
                    onClicked: AppSession.resetWorkspace()
                }

                Label {
                    Layout.topMargin: Theme.spaceSm
                    text: qsTr("Keyboard")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontLabel
                    font.weight: Font.DemiBold
                }
                Label {
                    Layout.fillWidth: true
                    text: qsTr("Click a binding, then press a shortcut. Esc cancels; Backspace clears.")
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                    wrapMode: Text.WordWrap
                }
                Label {
                    visible: preferencesDialog.shortcutConflictHint.length > 0
                             || preferencesDialog.capturingActionId.length > 0
                    Layout.fillWidth: true
                    text: preferencesDialog.capturingActionId.length > 0
                          ? qsTr("Waiting for shortcut…")
                          : preferencesDialog.shortcutConflictHint
                    color: Theme.warning
                    font.pixelSize: Theme.fontLabelSm
                    wrapMode: Text.WordWrap
                }

                Repeater {
                    model: {
                        var _ = AppSession.actionShortcutsJson
                        var out = []
                        var all = root.actionDescriptors
                        for (var i = 0; i < all.length; ++i) {
                            if (all[i].shortcut || root.shortcutForAction(all[i].id))
                                out.push(all[i])
                        }
                        return out
                    }
                    delegate: RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spaceSm
                        Label {
                            Layout.fillWidth: true
                            text: modelData.label.replace(/&/g, "")
                            color: Theme.colorOnSurface
                            font.pixelSize: Theme.fontBodySm
                            elide: Text.ElideRight
                        }
                        Button {
                            implicitWidth: 140
                            text: preferencesDialog.capturingActionId === modelData.id
                                  ? qsTr("Press keys…")
                                  : (root.shortcutForAction(modelData.id) || qsTr("None"))
                            onClicked: {
                                preferencesDialog.capturingActionId = modelData.id
                                preferencesDialog.shortcutConflictHint = ""
                                prefsFlick.forceActiveFocus()
                            }
                        }
                    }
                }

                Button {
                    text: qsTr("Reset shortcuts to defaults")
                    onClicked: {
                        preferencesDialog.capturingActionId = ""
                        preferencesDialog.shortcutConflictHint = ""
                        AppSession.resetKeymap()
                    }
                }

                Label {
                    Layout.fillWidth: true
                    text: qsTr("Stored in XDG config: phototux/preferences.json")
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontBodySm
                    wrapMode: Text.WordWrap
                }
            }
        }
    }

    Popup {
        id: commandPalette
        anchors.centerIn: parent
        width: Math.min(520, root.width - 48)
        height: Math.min(420, root.height - 48)
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        padding: Theme.spaceMd

        property int selectedIndex: 0
        property string query: ""

        function filteredActions() {
            var q = commandPalette.query.trim().toLowerCase()
            var out = []
            var all = root.actionDescriptors
            for (var i = 0; i < all.length; ++i) {
                var a = all[i]
                if (a.id === "action.app.command-palette")
                    continue
                if (!q) {
                    out.push(a)
                    continue
                }
                var label = (a.label || "").replace(/&/g, "").toLowerCase()
                var id = (a.id || "").toLowerCase()
                var menu = (a.menu || "").toLowerCase()
                if (label.indexOf(q) >= 0 || id.indexOf(q) >= 0 || menu.indexOf(q) >= 0)
                    out.push(a)
            }
            return out
        }

        function showPalette() {
            query = ""
            selectedIndex = 0
            open()
            paletteField.forceActiveFocus()
            AppSession.setShortcutInputYield(true)
        }

        function closePalette() {
            close()
            AppSession.setShortcutInputYield(false)
            root.refreshShortcutYield()
        }

        function runSelected() {
            var list = commandPalette.filteredActions()
            if (list.length === 0)
                return
            var idx = Math.max(0, Math.min(commandPalette.selectedIndex, list.length - 1))
            var action = list[idx]
            if (!root.actionIsEnabled(action.id))
                return
            commandPalette.closePalette()
            root.runAction(action.id)
        }

        onClosed: {
            AppSession.setShortcutInputYield(false)
            root.refreshShortcutYield()
        }

        background: Rectangle {
            color: Theme.surface
            border.color: Theme.border
            radius: Theme.radiusMd
        }

        contentItem: ColumnLayout {
            spacing: Theme.spaceSm
            anchors.fill: parent

            Label {
                text: qsTr("Command palette")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontLabel
                font.weight: Font.DemiBold
            }

            TextField {
                id: paletteField
                Layout.fillWidth: true
                placeholderText: qsTr("Filter commands…")
                text: commandPalette.query
                onTextChanged: {
                    commandPalette.query = text
                    commandPalette.selectedIndex = 0
                }
                Keys.onPressed: function (event) {
                    var list = commandPalette.filteredActions()
                    if (event.key === Qt.Key_Down) {
                        if (list.length > 0)
                            commandPalette.selectedIndex =
                                    Math.min(commandPalette.selectedIndex + 1, list.length - 1)
                        event.accepted = true
                    } else if (event.key === Qt.Key_Up) {
                        commandPalette.selectedIndex = Math.max(0, commandPalette.selectedIndex - 1)
                        event.accepted = true
                    } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        commandPalette.runSelected()
                        event.accepted = true
                    } else if (event.key === Qt.Key_Escape) {
                        commandPalette.closePalette()
                        event.accepted = true
                    }
                }
            }

            ListView {
                id: paletteList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: commandPalette.filteredActions()
                currentIndex: commandPalette.selectedIndex
                delegate: ItemDelegate {
                    width: paletteList.width
                    height: Theme.toolHit
                    highlighted: index === commandPalette.selectedIndex
                    opacity: root.actionIsEnabled(modelData.id) ? 1.0 : 0.45
                    contentItem: RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceSm
                        anchors.rightMargin: Theme.spaceSm
                        spacing: Theme.spaceSm
                        Label {
                            Layout.fillWidth: true
                            text: (modelData.label || "").replace(/&/g, "")
                            color: Theme.colorOnSurface
                            font.pixelSize: Theme.fontBodySm
                            elide: Text.ElideRight
                        }
                        Label {
                            text: modelData.menu || ""
                            color: Theme.colorOnSurfaceMuted
                            font.pixelSize: Theme.fontLabelSm
                        }
                        Label {
                            text: root.shortcutForAction(modelData.id)
                            color: Theme.primary
                            font.pixelSize: Theme.fontMono
                            font.family: "Noto Sans Mono"
                        }
                    }
                    onClicked: {
                        commandPalette.selectedIndex = index
                        commandPalette.runSelected()
                    }
                }
                ScrollBar.vertical: ScrollBar { }
            }

            Label {
                Layout.fillWidth: true
                visible: paletteList.count === 0
                text: qsTr("No matching commands")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBodySm
            }
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
            brushSlider.value = AppSession.brushSize
            layerOpacitySlider.value = AppSession.activeOpacity
            root.syncBlendCombo()
        }
    }

    Connections {
        target: AppSession
        function onActiveOpacityChanged() {
            if (Math.abs(layerOpacitySlider.value - AppSession.activeOpacity) > 0.001)
                layerOpacitySlider.value = AppSession.activeOpacity
        }
        function onActiveBlendChanged() {
            root.syncBlendCombo()
        }
        function onForegroundHexChanged() {
            if (hexField && !hexField.activeFocus && hexField.text !== AppSession.foregroundHex)
                hexField.text = AppSession.foregroundHex
        }
        function onIoErrorChanged() {
            if (AppSession.ioError.length > 0)
                ioErrorDialog.open()
        }
    }

}
