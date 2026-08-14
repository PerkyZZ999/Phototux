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
    // Leave room for SSD title bar on 900px virtual screens so the status footer stays on-screen.
    height: 860
    title: AppSession.hasDocument
           ? (AppSession.dirty
              ? qsTr("%1* — PhotoTux").arg(AppSession.documentName)
              : qsTr("%1 — PhotoTux").arg(AppSession.documentName))
           : qsTr("PhotoTux")
    color: Theme.neutral
    property string pendingDestructiveAction: ""
    readonly property var historyLabelParts: AppSession.historyLabels.length > 0
                                            ? AppSession.historyLabels.split("|") : []
    readonly property var historyKindParts: AppSession.historyKinds.length > 0
                                           ? AppSession.historyKinds.split("|") : []
    readonly property var historyIdParts: AppSession.historyEntryIds.length > 0
                                         ? AppSession.historyEntryIds.split("|") : []
    // The session answers for the active layer directly. These used to split
    // the whole stack's flags and index by activeLayerIndex, which could read
    // past the end of a string that had not caught up yet and silently report
    // "no mask" for a layer that had one.
    // Read once here so delegates can read it from QML instead of from the
    // host. A model's dataChanged reaches its view synchronously, and the
    // session emits those updates from inside a slot that still holds itself
    // borrowed — so a delegate binding that reads `AppSession.x` directly is
    // a re-entrant borrow and aborts the process. Going through a root
    // property makes the host read happen on AppSession's own notify, which
    // Qt evaluates lazily, outside the borrow.
    readonly property bool maskEditActive: AppSession.maskEditActive
    readonly property int activeMaskFlag: AppSession.activeMaskFlag
    readonly property bool activeLayerHasMask: activeMaskFlag !== 0
    readonly property bool activeMaskEnabled: activeMaskFlag === 1
    readonly property bool activeLayerClips: AppSession.activeLayerClips
    readonly property string activeLayerKind: AppSession.activeLayerKind
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
    readonly property var documentTabs: {
        try {
            return JSON.parse(AppSession.documentTabsJson || "[]")
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

    /// Right dock as tab groups, derived host-side so the grouping rule lives
    /// in one place: `[{ tabs: [id, …], active: id }]`.
    readonly property var dockGroups: {
        try {
            return JSON.parse(AppSession.dockGroupsJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Row base for GridLayout dock (header = base*2, body = base*2+1).
    ///
    /// Panels in one tab group share a row pair; only the active tab's body is
    /// visible, so they occupy the same cell without overlapping.
    function dockStackRow(panelId) {
        var groups = root.dockGroups
        for (var g = 0; g < groups.length; ++g) {
            var tabs = groups[g].tabs || []
            for (var t = 0; t < tabs.length; ++t) {
                if (tabs[t] === panelId)
                    return g
            }
        }
        return 1000
    }

    /// Tabs sharing `panelId`'s group, in stack order.
    function dockGroupTabs(panelId) {
        var groups = root.dockGroups
        for (var g = 0; g < groups.length; ++g) {
            var tabs = groups[g].tabs || []
            for (var t = 0; t < tabs.length; ++t) {
                if (tabs[t] === panelId)
                    return tabs
            }
        }
        return [panelId]
    }

    /// True when `panelId` is the tab its group is currently showing.
    function panelIsActiveTab(panelId) {
        var groups = root.dockGroups
        for (var g = 0; g < groups.length; ++g) {
            var tabs = groups[g].tabs || []
            for (var t = 0; t < tabs.length; ++t) {
                if (tabs[t] === panelId)
                    return groups[g].active === panelId
            }
        }
        return true
    }

    /// Visible tabs of a group, so a hidden or auto-hidden panel leaves no
    /// dead tab behind.
    function dockGroupVisibleTabs(panelId) {
        var tabs = root.dockGroupTabs(panelId)
        var out = []
        for (var i = 0; i < tabs.length; ++i) {
            if (root.panelIsVisible(tabs[i]) && !root.panelIsAutoHidden(tabs[i]))
                out.push(tabs[i])
        }
        return out
    }

    function panelIsDocked(panelId) {
        return root.dockStackRow(panelId) < 1000
    }

    function panelIsAutoHidden(panelId) {
        var list = root.dockTopology.auto_hidden || []
        for (var i = 0; i < list.length; ++i) {
            if (list[i] === panelId)
                return true
        }
        return false
    }

    /// Whether this panel renders in the dock right now.
    ///
    /// Panels in a tab group share a row pair, so only the active tab draws —
    /// its header carries the whole group's tab strip.
    function panelShowsInDock(panelId) {
        return root.panelIsVisible(panelId)
                && root.panelIsDocked(panelId)
                && !root.panelIsAutoHidden(panelId)
                && root.panelIsActiveTab(panelId)
    }

    function commitHeaderDrag(panelId, dy) {
        var step = Theme.panelHeaderHeight > 0 ? Theme.panelHeaderHeight : 28
        var delta = Math.round(dy / step)
        if (delta !== 0)
            AppSession.movePanelInStack(panelId, delta)
    }

    readonly property var floatingPanels: dockTopology.floating || []
    readonly property var autoHiddenPanels: dockTopology.auto_hidden || []
    /// Stable Instantiator key — panel ids only so geometry persists do not recreate Windows.
    readonly property string floatingPanelIdKey: {
        var _ = AppSession.dockTopologyJson
        var panels = root.floatingPanels
        var ids = []
        for (var i = 0; i < panels.length; ++i)
            ids.push(panels[i].id)
        return ids.join("\n")
    }
    /// Block float geometry writes until the first screen clamp finishes.
    property bool floatingPersistEnabled: false

    function floatingPlacement(panelId) {
        var panels = root.floatingPanels
        for (var i = 0; i < panels.length; ++i) {
            if (panels[i].id === panelId)
                return panels[i]
        }
        return null
    }

    function tearOffAndClamp(panelId, x, y, width, height) {
        AppSession.tearOffPanel(panelId, x, y, width, height)
        root.reclampFloatingPanels()
    }

    function panelTitle(panelId) {
        var all = root.panelDescriptors
        for (var i = 0; i < all.length; ++i) {
            if (all[i].id === panelId)
                return all[i].title || panelId
        }
        return panelId
    }

    /// Phosphor stem for auto-hide strip / panel chrome (ICON_MAP-aligned).
    function panelIconStem(panelId) {
        if (panelId === "panel.properties")
            return "gear"
        if (panelId === "panel.navigator")
            return "magnifying-glass"
        if (panelId === "panel.swatches")
            return "image-square"
        if (panelId === "panel.layers")
            return "folder"
        if (panelId === "panel.history")
            return "arrow-counter-clockwise"
        return "dots-three"
    }

    function panelIsVisible(panelId) {
        return root.panelVisibilityMap[panelId] === true
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

    /// How many tool strip rows fit above the overflow control.
    function toolStripCapacity(stripHeight) {
        // Dense packing: 2px gaps match toolColumn.spacing so a 900px shell fits the
        // full essentials tool set without forcing overflow.
        // Literal hit size: Theme.toolHit can resolve to 0 under PHOTOTUX_QML.
        var gap = 2
        var hit = 40
        var row = hit + gap
        var reserve = hit + 8
        return Math.max(1, Math.floor((stripHeight - reserve - gap) / row))
    }

    /// Split tools into visible strip vs overflow menu; keep active tool on-strip.
    function toolStripPartitions(capacity) {
        var all = root.toolDescriptors
        var cap = Math.max(1, capacity)
        if (all.length <= cap)
            return ({ visible: all, overflow: [] })
        var visible = all.slice(0, cap)
        var overflow = all.slice(cap)
        var active = AppSession.activeTool
        var oi = -1
        for (var i = 0; i < overflow.length; ++i) {
            if (overflow[i].id === active) {
                oi = i
                break
            }
        }
        if (oi >= 0) {
            var swapped = visible[visible.length - 1]
            visible[visible.length - 1] = overflow[oi]
            overflow[oi] = swapped
        }
        return ({ visible: visible, overflow: overflow })
    }

    function activateToolFromStrip(toolId) {
        if (AppSession.transformActive && toolId !== "tool.transform")
            AppSession.cancelTransform()
        if (AppSession.cropPreviewActive && toolId !== "tool.crop")
            AppSession.cancelCrop()
        AppSession.setActiveTool(toolId)
        if (toolId === "tool.transform")
            AppSession.beginTransform()
    }

    function shortcutForAction(actionId) {
        return root.actionShortcutMap[actionId] || ""
    }

    function itemIsTextEditor(item) {
        if (!item)
            return false
        if (item instanceof TextInput || item instanceof TextEdit
                || item instanceof TextField || item instanceof TextArea
                || item instanceof SpinBox)
            return true
        // Controls may keep focus on the wrapper; editor is contentItem.
        if (item.contentItem && root.itemIsTextEditor(item.contentItem))
            return true
        // Duck-type editors (instanceof can fail across QML import boundaries).
        if (typeof item.text === "string"
                && typeof item.cursorPosition === "number"
                && typeof item.select === "function")
            return true
        return false
    }

    /// Recompute whether the host should hand single-key shortcuts to a text
    /// editor instead of consuming them.
    ///
    /// Always deferred. Every caller reacts to focus moving or a popup opening
    /// or closing, and both happen synchronously inside the host slot that
    /// showed or hid the dialog — opening the Filter Gallery moves focus into
    /// its popup while `openFilterGallery` is still on the stack. Calling the
    /// host from there aborts the process; see `afterHostSlot`.
    function refreshShortcutYield() {
        root.afterHostSlot(root.applyShortcutYield)
    }

    function applyShortcutYield() {
        var yieldKeys = root.itemIsTextEditor(root.activeFocusItem)
                || newDocDialog.opened
        AppSession.setShortcutInputYield(
                    yieldKeys || AppSession.preferencesOpen || AppSession.filterGalleryOpen)
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
                + AppSession.panelVisibilityJson
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
        case "action.window.panel-swatches":
        case "action.window.panel-layers":
        case "action.window.panel-history":
        case "action.window.panel-properties":
        case "action.window.panel-paths":
        case "action.window.panel-character":
            // Every Window-menu panel toggle reads the one registry map, so a
            // new panel needs no case of its own.
            return root.panelIsVisible(actionId.replace("action.window.panel-",
                                                        "panel."))
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
        case "action.window.panel-swatches":
        case "action.window.panel-layers":
        case "action.window.panel-history":
        case "action.window.panel-properties":
        case "action.window.panel-paths":
        case "action.window.panel-character":
            AppSession.setPanelVisible(
                        actionId.replace("action.window.panel-", "panel."), checked)
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

    /// Act on a shell capability request from the host.
    ///
    /// The vocabulary is `phototux_engine::HostRequest`; these names are its
    /// wire names, and every one has a round-trip test on the Rust side. This
    /// used to prefix-match a `"host:"` marker smuggled through the status-bar
    /// text, so the contract was ten string literals duplicated across two
    /// languages with nothing checking they agreed.
    function handleHostRequest(kind) {
        if (!kind)
            return
        switch (kind) {
        case "document.new":
            root.openNewDocumentDialog()
            break
        case "document.open":
            welcomeDialog.close()
            openFileDialog.open()
            break
        case "document.save_as":
            saveFileDialog.open()
            break
        case "document.export":
            exportFileDialog.open()
            break
        case "document.close":
            root.requestDestructiveAction("close")
            break
        case "app.quit":
            root.requestDestructiveAction("quit")
            break
        case "help.about":
            aboutDialogLoader.open()
            break
        case "document.embed_icc":
            embedIccFileDialog.open()
            break
        case "palette.open":
            commandPaletteLoader.ensure().showPalette()
            break
        default:
            return
        }
        AppSession.clearHostRequest()
    }

    /// Run `fn` once the host slot that is currently executing has returned.
    ///
    /// Every `AppSession` slot takes `&mut self`, and qtbridge holds that borrow
    /// for the whole slot body — including the notify signals the slot emits.
    /// A QML handler that reacts to one of those signals therefore runs *inside*
    /// the borrow, so calling any other slot from it fails the borrow check and
    /// aborts the process (`BorrowConflict` in `genericrustproxy.rs`, a hard
    /// abort rather than a catchable error). Reactive handlers must hand the
    /// call back to the event loop instead.
    ///
    /// Rule: a handler that fires from an `AppSession` notify signal — a
    /// `Connections` function, a binding-driven `on…Changed`, or anything a
    /// `Loader` builds in response to host state — must not call an
    /// `AppSession` slot directly. Route it through here. Direct calls are fine
    /// from user input (`onClicked`, `onMoved`, `Keys.onPressed`), which the
    /// event loop already delivers outside any borrow.
    ///
    /// Repeated calls with the same function collapse into one, so this also
    /// coalesces write-back storms such as window drags.
    function afterHostSlot(fn) {
        Qt.callLater(fn)
    }

    /// Re-read at drain time rather than capturing, so a request superseded
    /// before the event loop turns is not acted on twice.
    function drainHostRequest() {
        root.handleHostRequest(AppSession.pendingHostRequest)
    }

    Connections {
        target: AppSession
        function onPendingHostRequestChanged() {
            // Deferred: the request is published from inside a host slot, and
            // every branch of the handler calls back into AppSession — both to
            // act and to acknowledge. See `root.afterHostSlot`.
            root.afterHostSlot(root.drainHostRequest)
        }
    }

    readonly property int statusHeight: Theme.statusbarHeight
    readonly property color primary: Theme.primary
    readonly property color surface: Theme.surface
    readonly property color surfaceRaised: Theme.surfaceRaised
    readonly property color surfaceSunken: Theme.surfaceSunken
    readonly property color surfaceOverlay: Theme.surfaceOverlay
    readonly property color border: Theme.border

    // Cached rather than read through to the host on every call. `iconUrl` is
    // called from delegate bindings, which re-evaluate when a model row
    // changes — and the session emits those changes while it still holds
    // itself borrowed, so reading `AppSession.iconRoot` from inside a delegate
    // binding is a re-entrant borrow and aborts the process. The asset root
    // does not change at runtime, so caching it costs nothing.
    readonly property string iconRoot: AppSession.iconRoot
    function iconUrl(stem) {
        return Theme.iconUrl(root.iconRoot, stem)
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
    // The combo itself lives in PropertiesPanel; the shell only asks it to
    // resync. Guarded because the panel is built with the right dock, and
    // host state can change before that happens.
    function syncBlendCombo() {
        if (typeof propsCol !== "undefined" && propsCol)
            propsCol.syncBlendCombo()
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
    function selectionCombineLabel(mode) {
        if (mode === "add")
            return qsTr("Add")
        if (mode === "subtract")
            return qsTr("Subtract")
        if (mode === "intersect")
            return qsTr("Intersect")
        return qsTr("Replace")
    }

    // Resolved expansion per registered group (descriptor default merged with
    // the user's sparse overrides). Drives which way the panel-local toggle goes.
    readonly property var disclosureOpenMap: {
        try {
            return JSON.parse(AppSession.disclosureOpenJson || "{}")
        } catch (e) {
            return ({})
        }
    }
    readonly property bool anyDisclosureGroupExpanded: {
        var map = root.disclosureOpenMap
        for (var id in map) {
            if (map[id] === true)
                return true
        }
        return false
    }

    // Adjustment slider bounds come from the engine so the editor and the
    // out-of-range disclosure badge cannot disagree about what is showable.
    readonly property var adjustmentRanges: {
        try {
            return JSON.parse(AppSession.adjustmentRangesJson || "{}")
        } catch (e) {
            return ({})
        }
    }
    function adjRange(kind, slot, edge) {
        var params = root.adjustmentRanges[kind]
        var bounds = params ? params[slot] : undefined
        if (!bounds)
            return edge === 0 ? 0 : 1
        return bounds[edge]
    }
    readonly property color colorOnSurface: Theme.colorOnSurface
    readonly property color colorOnSurfaceMuted: Theme.colorOnSurfaceMuted
    readonly property color warning: Theme.warning
    readonly property color canvasLetterbox: Theme.canvasLetterbox
    readonly property color toolActiveBg: Theme.toolActiveBg

    function openNewDocumentDialog() {
        // Cancel in-progress gallery so New is not blocked by modal yield.
        if (AppSession.filterGalleryOpen)
            AppSession.filterGalleryCancel()
        welcomeDialog.close()
        // Defer so Welcome's modal Overlay is torn down before the next Popup mounts.
        Qt.callLater(function () {
            newDocDialog.open()
        })
    }

    function executeDestructiveAction(action) {
        pendingDestructiveAction = ""
        if (action === "new") {
            root.openNewDocumentDialog()
        } else if (action === "open") {
            welcomeDialog.close()
            openFileDialog.open()
        } else if (action === "close") {
            AppSession.closeDocument()
        } else if (action === "quit") {
            Qt.quit()
        }
    }

    function requestDestructiveAction(action) {
        // Hostile UX: close/quit while Filter Gallery is open must not stick the modal.
        if (AppSession.filterGalleryOpen)
            AppSession.filterGalleryCancel()
        if (AppSession.dirty) {
            pendingDestructiveAction = action
            unsavedDialogLoader.open()
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
            unsavedDialogLoader.open()
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
            icon.color: enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
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

    Shortcut {
        sequence: "Esc"
        context: Qt.ApplicationShortcut
        enabled: !AppSession.shortcutInputYield && !AppSession.preferencesOpen
                 && (AppSession.transformActive
                     || AppSession.cropPreviewActive
                     || root.autoHiddenPanels.length > 0)
        onActivated: {
            if (AppSession.transformActive) {
                AppSession.cancelTransform()
                return
            }
            if (AppSession.cropPreviewActive) {
                AppSession.cancelCrop()
                return
            }
            var id = root.autoHiddenPanels[0]
            if (id)
                AppSession.pinPanel(id)
        }
    }

    Connections {
        target: AppSession
        function onPreferencesOpenChanged() {
            root.refreshShortcutYield()
        }
    }

    function openContextMenu(menu, originItem, localX, localY) {
        var p = originItem.mapToItem(Overlay.overlay, localX, localY)
        // Keep menu on-screen when opened from bottom dock rows (layers).
        var approxH = Math.max(160, menu.contentHeight || 0)
        if (p.y + approxH > Overlay.overlay.height - 8)
            p.y = Math.max(8, Overlay.overlay.height - approxH - 8)
        if (p.x + 260 > Overlay.overlay.width - 8)
            p.x = Math.max(8, Overlay.overlay.width - 260 - 8)
        menu.popup(Overlay.overlay, p.x, p.y)
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
                icon.color: enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
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
        // Fusion paints a light menubar and ignores custom backgrounds here.
        // Keep default dark labels (WCAG ≥ 4.5:1 on that light bar).
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

    Binding { target: Theme; property: "highContrast"; value: AppSession.prefHighContrast }
    Binding { target: Theme; property: "reducedMotion"; value: AppSession.prefReducedMotion }
    Binding { target: Theme; property: "uiDensity"; value: AppSession.prefUiDensity }

    function reclampFloatingPanels() {
        if (Screen.width <= 0 || Screen.height <= 0)
            return
        if (!root.floatingPanels || root.floatingPanels.length === 0)
            return
        AppSession.clampFloatingPanels(0, 0, Screen.width, Screen.height)
    }

    onWidthChanged: Qt.callLater(root.reclampFloatingPanels)
    onHeightChanged: Qt.callLater(root.reclampFloatingPanels)

    Connections {
        target: Screen
        function onWidthChanged() { root.reclampFloatingPanels() }
        function onHeightChanged() { root.reclampFloatingPanels() }
    }

    Component.onCompleted: {
        if (Screen.height > 0 && height + 48 > Screen.height)
            height = Math.max(600, Screen.height - 48)
        // Clamp restored floating geometry before any Window persist can run.
        if (Screen.width > 0 && Screen.height > 0)
            AppSession.clampFloatingPanels(0, 0, Screen.width, Screen.height)
        root.floatingPersistEnabled = true
        AppSession.refreshRecoveryList()
        var entries = []
        try { entries = JSON.parse(AppSession.recoveryEntriesJson || "[]") } catch (e) {}
        if (entries.length > 0)
            recoveryDialogLoader.open()
        else if (!AppSession.hasDocument && !AppSession.ioBusy)
            welcomeDialog.open()
    }

    // Periodic recovery snapshot while the document is dirty (handbook §02).
    Timer {
        id: autosaveTimer
        interval: 8000
        repeat: true
        running: AppSession.hasDocument && AppSession.dirty && !AppSession.ioBusy
        onTriggered: AppSession.autosaveNow()
    }

    LazyDialog {
        id: recoveryDialogLoader

        Dialog {
            id: recoveryDialog
            parent: Overlay.overlay
            anchors.centerIn: parent
            modal: true
            title: qsTr("Recover unsaved work")
            header: ThemedDialogHeader { text: recoveryDialog.title }
            width: 440
            standardButtons: Dialog.Close
            onClosed: {
                if (!AppSession.hasDocument && !AppSession.ioBusy)
                    welcomeDialog.open()
            }
            background: Rectangle {
                color: Theme.surface
                border.color: Theme.border
                radius: Theme.radiusMd
            }
            contentItem: ColumnLayout {
                spacing: Theme.spaceSm
                width: 400
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: qsTr("PhotoTux found autosaved documents from a previous session.")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBody
                }
                Repeater {
                    model: {
                        try {
                            return JSON.parse(AppSession.recoveryEntriesJson || "[]")
                        } catch (e) {
                            return []
                        }
                    }
                    delegate: RowLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: Theme.spaceSm
                        Label {
                            Layout.fillWidth: true
                            elide: Text.ElideMiddle
                            text: modelData.original_path && modelData.original_path.length
                                  ? modelData.original_path
                                  : qsTr("Untitled (%1)").arg(modelData.document_id.slice(0, 8))
                            color: Theme.colorOnSurface
                            font.pixelSize: Theme.fontBodySm
                        }
                        Button {
                            text: qsTr("Restore")
                            onClicked: {
                                AppSession.restoreRecovery(modelData.document_id)
                                recoveryDialog.close()
                            }
                        }
                        Button {
                            text: qsTr("Discard")
                            flat: true
                            onClicked: AppSession.discardRecoveryEntry(modelData.document_id)
                        }
                    }
                }
            }
        }
    }

    Instantiator {
        model: root.floatingPanelIdKey.length > 0 ? root.floatingPanelIdKey.split("\n") : []
        onObjectRemoved: function (index, object) {
            if (!object)
                return
            // Deliberately no close(): closing re-emits `onClosing`, which would
            // call back into an AppSession slot while the slot that shrank this
            // model still holds the session borrowed. Hiding drops the platform
            // window on its own.
            object.retiring = true
            object.visible = false
            object.destroy()
        }
        delegate: Window {
            id: floatWin
            required property string modelData
            property bool syncingGeometry: false
            /// Set by the Instantiator just before teardown so the close that
            /// follows does not ask the host to redock a panel it already moved.
            property bool retiring: false
            readonly property var placement: {
                var _ = AppSession.dockTopologyJson
                return root.floatingPlacement(modelData)
            }
            title: qsTr(root.panelTitle(modelData))
            width: Math.max(200, (placement && placement.width) || 320)
            height: Math.max(120, (placement && placement.height) || 280)
            x: (placement && placement.x !== undefined) ? placement.x : 80
            y: (placement && placement.y !== undefined) ? placement.y : 80
            visible: true
            color: Theme.surface
            flags: Qt.Window | Qt.WindowTitleHint | Qt.WindowCloseButtonHint | Qt.WindowMinMaxButtonsHint

            onClosing: function (close) {
                close.accepted = true
                root.afterHostSlot(floatWin.requestRedock)
            }
            // Every write-back below is deferred. A tear-off builds this Window
            // from inside `tearOffPanel`, so anything that runs synchronously
            // here re-enters AppSession while it is still mutably borrowed and
            // aborts the process. Qt.callLater also coalesces the geometry
            // storm during a drag into one persist per event-loop turn.
            onXChanged: root.afterHostSlot(floatWin.persistGeometry)
            onYChanged: root.afterHostSlot(floatWin.persistGeometry)
            onWidthChanged: root.afterHostSlot(floatWin.persistGeometry)
            onHeightChanged: root.afterHostSlot(floatWin.persistGeometry)

            function requestRedock() {
                if (floatWin.retiring)
                    return
                AppSession.redockPanel(floatWin.modelData)
            }

            function applyModelGeometry() {
                var p = placement
                if (!p)
                    return
                var nx = p.x || 0
                var ny = p.y || 0
                var nw = Math.max(200, p.width || 320)
                var nh = Math.max(120, p.height || 280)
                if (x === nx && y === ny && width === nw && height === nh)
                    return
                syncingGeometry = true
                x = nx
                y = ny
                width = nw
                height = nh
                syncingGeometry = false
            }

            function persistGeometry() {
                if (retiring || !visible || syncingGeometry || !root.floatingPersistEnabled)
                    return
                AppSession.setFloatingPanelGeometry(modelData, Math.round(x), Math.round(y),
                                                   Math.round(width), Math.round(height))
            }

            Connections {
                target: AppSession
                function onDockTopologyJsonChanged() {
                    floatWin.applyModelGeometry()
                }
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Theme.spaceSm
                spacing: Theme.spaceSm
                Label {
                    Layout.fillWidth: true
                    text: qsTr("%1 (floating)").arg(qsTr(root.panelTitle(modelData)))
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontLabel
                    font.weight: Font.Medium
                }
                Label {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    text: qsTr("Close window or Dock to return this panel to the right stack.")
                    color: Theme.colorOnSurfaceVariant
                    font.pixelSize: Theme.fontBodySm
                    wrapMode: Text.WordWrap
                }
                Button {
                    text: qsTr("Dock")
                    onClicked: root.afterHostSlot(floatWin.requestRedock)
                }
            }
        }
    }

    // —— Top chrome ——
    // Two stacked bars: the operation toolbar, then the active tool's options.
    // Handbook 06 keeps these distinct — the first is document-scoped and
    // constant, the second changes with the tool and is disclosure level 1.
    header: ColumnLayout {
        spacing: 0

    Rectangle {
        id: mainToolBar
        Layout.fillWidth: true
        Layout.preferredHeight: Theme.toolbarHeight
        implicitHeight: Theme.toolbarHeight
        color: Theme.surface
        Accessible.role: Accessible.ToolBar
        Accessible.name: qsTr("Main toolbar")

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.borderEffective
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
                contentItem: Rectangle { implicitWidth: 1; color: Theme.borderEffective }
            }

            ChromeIconToolButton {
                icon.source: root.iconUrl("file-plus")
                enabled: root.actionIsEnabled("action.file.new")
                onClicked: root.runAction("action.file.new")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("New…")
                Accessible.name: ToolTip.text
            }

            ChromeIconToolButton {
                icon.source: root.iconUrl("folder-open")
                enabled: root.actionIsEnabled("action.file.open")
                onClicked: root.runAction("action.file.open")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Open…")
                Accessible.name: ToolTip.text
            }

            ChromeIconToolButton {
                icon.source: root.iconUrl("export")
                enabled: root.actionIsEnabled("action.file.export")
                onClicked: root.runAction("action.file.export")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Export PNG, JPEG, or PSD subset")
                Accessible.name: ToolTip.text
            }

            ToolSeparator {
                contentItem: Rectangle { implicitWidth: 1; color: Theme.borderEffective }
            }

            ChromeIconToolButton {
                icon.source: root.iconUrl("arrow-counter-clockwise")
                enabled: root.actionIsEnabled("action.edit.undo")
                onClicked: root.runAction("action.edit.undo")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Undo")
                Accessible.name: ToolTip.text
            }
            ChromeIconToolButton {
                icon.source: root.iconUrl("arrow-clockwise")
                enabled: root.actionIsEnabled("action.edit.redo")
                onClicked: root.runAction("action.edit.redo")
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Redo")
                Accessible.name: ToolTip.text
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

            ChromeIconToolButton {
                implicitWidth: 28
                implicitHeight: 28
                icon.source: root.iconUrl("question")
                onClicked: aboutDialogLoader.open()
                ToolTip.visible: hovered
                ToolTip.text: qsTr("About PhotoTux")
                Accessible.name: ToolTip.text
            }
        }
    }

    ToolOptionsBar {
        Layout.fillWidth: true
        Layout.preferredHeight: Theme.toolbarHeight
        // Presence, not disclosure: with no document there is no tool context
        // to describe, so the bar is absent rather than empty.
        visible: AppSession.hasDocument
    }
    }

    footer: Rectangle {
        id: statusToolBar
        implicitHeight: root.statusHeight
        height: root.statusHeight
        color: Theme.surfaceContainer
        Accessible.role: Accessible.ToolBar
        Accessible.name: qsTr("Status")

        Rectangle {
            anchors.top: parent.top
            width: parent.width
            height: 1
            color: Theme.borderEffective
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceMd
            anchors.rightMargin: Theme.spaceMd
            spacing: Theme.spaceLg

            Label {
                text: AppSession.statusText
                color: AppSession.gpuLost ? Theme.warning : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
                elide: Text.ElideRight
                Layout.fillWidth: true
                // Expose the latest status string for AT-SPI; FPS/comp stay ignored below.
                Accessible.name: text.length > 0 ? text : qsTr("Status")
                Accessible.role: Accessible.StatusBar
            }

            Button {
                visible: AppSession.gpuLost
                text: qsTr("Recover")
                flat: true
                Accessible.name: qsTr("Recover graphics after device loss")
                onClicked: AppSession.recoverGpu()
            }

            RowLayout {
                visible: AppSession.dirty
                spacing: Theme.spaceXs
                ThemedIcon {
                    source: root.iconUrl("circle-notch")
                    size: 12
                    color: Theme.warning
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
                Accessible.ignored: true
            }

            Label {
                text: AppSession.compositeMs > 0
                      ? (qsTr("comp %1 ms").arg(AppSession.compositeMs.toFixed(2)))
                      : ""
                color: AppSession.compositeMs > 0 && AppSession.compositeMs < 2.0
                       ? Theme.primary : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
                Accessible.ignored: true
            }

            Label {
                text: AppSession.fps > 0
                      ? (qsTr("FPS: %1").arg(Math.round(AppSession.fps)))
                      : qsTr("FPS: —")
                color: AppSession.fps >= 60 ? Theme.primary : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
                Accessible.ignored: true
            }

            Rectangle {
                radius: Theme.radiusXs
                color: Theme.successSubtle
                width: gpuBadge.implicitWidth + Theme.spaceSm
                height: Theme.controlHeight - 6
                Accessible.ignored: true
                Label {
                    id: gpuBadge
                    anchors.centerIn: parent
                    text: qsTr("GPU ACCELERATED")
                    color: Theme.success
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                    font.weight: Font.DemiBold
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        enabled: !AppSession.ioBusy

        TabBar {
            id: documentTabBar
            Layout.fillWidth: true
            Layout.preferredHeight: root.documentTabs.length > 0 ? Theme.panelHeaderHeight : 0
            visible: root.documentTabs.length > 0
            clip: true
            background: Rectangle { color: Theme.surfaceContainer }

            Repeater {
                model: root.documentTabs
                TabButton {
                    required property var modelData
                    width: Math.min(180, implicitWidth + Theme.spaceMd)
                    text: (modelData.dirty ? "* " : "") + (modelData.title || qsTr("Untitled"))
                    checked: modelData.active === true
                    Accessible.name: text
                    onClicked: AppSession.activateDocumentTab(Number(modelData.id))

                    contentItem: Text {
                        text: parent.text
                        color: parent.checked ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
                        font.pixelSize: Theme.fontLabel
                        font.weight: parent.checked ? Font.DemiBold : Font.Normal
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }
                    background: Item {
                        id: tabBg
                        implicitHeight: Theme.panelHeaderHeight
                        readonly property bool tabChecked: parent && parent.checked
                        Rectangle {
                            anchors.fill: parent
                            color: tabBg.tabChecked ? Theme.surface : Theme.tabInactive
                        }
                        Rectangle {
                            visible: tabBg.tabChecked
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: 2
                            color: Theme.primary
                        }
                        Rectangle {
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: 1
                            color: Theme.borderSubtle
                        }
                    }
                }
            }
        }

        // Anchored panes (not RowLayout): fixed tool strip + dock widths so the
        // fill-width canvas cannot collapse the right dock to zero.
        Item {
            id: mainPanes
            Layout.fillWidth: true
            Layout.fillHeight: true

        // Left tool strip (overflow menu when strip height is tight)
        Rectangle {
            id: toolStrip
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: Theme.toolStripWidth
            color: Theme.surface
            Accessible.role: Accessible.ToolBar
            Accessible.name: qsTr("Tools")

            readonly property int stripCapacity: root.toolStripCapacity(height)
            // Bind active tool so overflow partition refreshes when selection changes.
            readonly property string activeToolBind: AppSession.activeTool
            readonly property int toolCountBind: root.toolDescriptors.length
            readonly property var stripParts: {
                var _ = activeToolBind
                var __ = toolCountBind
                return root.toolStripPartitions(stripCapacity)
            }
            readonly property var stripVisible: stripParts.visible || []
            readonly property var stripOverflow: stripParts.overflow || []

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
                anchors.topMargin: 2
                spacing: 2
                // Literal width: Theme.spaceXs can be 0/NaN under filesystem QML.
                width: parent.width - 8

                Repeater {
                    model: toolStrip.stripVisible
                    delegate: Item {
                        width: toolColumn.width
                        height: 40
                        readonly property string toolId: modelData.id
                        readonly property string toolGroup: modelData.group || ""
                        readonly property string prevGroup: index > 0
                                                           ? toolStrip.stripVisible[index - 1].group
                                                           : ""

                        Rectangle {
                            visible: index > 0 && toolGroup !== prevGroup
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            width: parent.width - 8
                            height: 1
                            y: -2
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

                            ThemedIcon {
                                anchors.centerIn: parent
                                source: root.iconUrl(root.toolIconStem(modelData.icon_key))
                                size: 20
                                color: Theme.iconOnSurfaceEffective
                            }

                            HoverHandler { id: toolHover }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                Accessible.role: Accessible.Button
                                Accessible.name: modelData.title
                                Accessible.description: toolId
                                Accessible.checkable: true
                                Accessible.checked: AppSession.activeTool === toolId
                                onClicked: root.activateToolFromStrip(toolId)
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
                id: toolOverflowBtn
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 8
                implicitWidth: 36
                implicitHeight: 36
                visible: toolStrip.stripOverflow.length > 0
                Accessible.name: qsTr("More tools")
                Accessible.description: qsTr("Show tools that do not fit in the strip")
                icon.source: root.iconUrl("dots-three")
                icon.width: 18
                icon.height: 18
                contentItem: ThemedIcon {
                    anchors.centerIn: parent
                    source: toolOverflowBtn.icon.source
                    size: 18
                    color: toolOverflowBtn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                }
                background: Rectangle {
                    radius: Theme.radiusSm
                    color: toolOverflowBtn.hovered ? Theme.surfaceContainerHigh : "transparent"
                }
                ToolTip.visible: hovered
                ToolTip.text: qsTr("More tools")
                // Defer open so the activating press is not treated as PressOutside
                // (CloseOnPressOutside) — required for reliable EIS / AT clicks.
                onClicked: Qt.callLater(toolOverflowPopup.open)
            }

            Popup {
                id: toolOverflowPopup
                parent: Overlay.overlay
                modal: true
                focus: true
                padding: 4
                // Enable outside-close only after the opening click has settled.
                property int settledClosePolicy: Popup.CloseOnEscape
                closePolicy: settledClosePolicy
                onOpened: Qt.callLater(function () {
                    settledClosePolicy = Popup.CloseOnEscape | Popup.CloseOnPressOutside
                })
                onClosed: settledClosePolicy = Popup.CloseOnEscape
                x: {
                    var origin = toolOverflowBtn.mapToItem(Overlay.overlay, 0, 0)
                    return origin.x
                }
                y: {
                    var origin = toolOverflowBtn.mapToItem(Overlay.overlay, 0, 0)
                    return Math.max(0, origin.y - implicitHeight)
                }
                background: Rectangle {
                    color: Theme.surfaceContainer
                    border.color: Theme.border
                    radius: Theme.radiusSm
                }
                Accessible.role: Accessible.PopupMenu
                Accessible.name: qsTr("More tools")

                Column {
                    spacing: 2
                    Repeater {
                        model: toolStrip.stripOverflow
                        delegate: Item {
                            required property var modelData
                            width: 168
                            height: 36

                            Rectangle {
                                anchors.fill: parent
                                radius: Theme.radiusSm
                                color: AppSession.activeTool === modelData.id
                                       ? Theme.toolActiveBg
                                       : (overflowItemHover.hovered
                                          ? Theme.surfaceContainerHigh : "transparent")
                            }

                            Row {
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.left: parent.left
                                anchors.leftMargin: 8
                                spacing: 8

                                ThemedIcon {
                                    anchors.verticalCenter: parent.verticalCenter
                                    source: root.iconUrl(root.toolIconStem(modelData.icon_key))
                                    size: 18
                                    color: Theme.iconOnSurfaceEffective
                                }

                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: modelData.title
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBody
                                }
                            }

                            HoverHandler { id: overflowItemHover }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                Accessible.role: Accessible.MenuItem
                                Accessible.name: modelData.title
                                Accessible.description: modelData.id
                                Accessible.checkable: true
                                Accessible.checked: AppSession.activeTool === modelData.id
                                onClicked: {
                                    var toolId = modelData.id
                                    toolOverflowPopup.close()
                                    root.activateToolFromStrip(toolId)
                                }
                            }
                        }
                    }
                }
            }
        }

        // GPU canvas viewport
        Item {
            id: canvasHost
            anchors.left: toolStrip.right
            anchors.right: rightDock.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom

            Rectangle {
                anchors.fill: parent
                color: Theme.canvasLetterbox
                z: -1
            }

            // Anchored to the tool strip and right dock, so this width changes
            // synchronously whenever a host slot resizes either — deferring
            // keeps the write-back out of that slot's borrow, and coalesces the
            // resize storm into one call per event-loop turn.
            function pushViewportSize() {
                AppSession.setViewportSize(canvasHost.width, canvasHost.height)
            }
            onWidthChanged: root.afterHostSlot(canvasHost.pushViewportSize)
            onHeightChanged: root.afterHostSlot(canvasHost.pushViewportSize)
            Component.onCompleted: canvasHost.pushViewportSize()

            PhototuxCanvas {
                id: gpuCanvas
                anchors.fill: parent
                zoom: AppSession.zoom
                panX: AppSession.panX
                panY: AppSession.panY
                docWidth: AppSession.docWidth
                docHeight: AppSession.docHeight
                hasDocument: AppSession.hasDocument
                // The shader reads phase only after an early-out on
                // selectionAnts, so animating it with ants off forced a full
                // RHI sync and render pass every vsync for a value nothing
                // sampled. Repaints are driven by contentTick instead, which
                // moves exactly when a new composite is published.
                phase: gpuCanvas.selectionAnts ? frameClock.phase : 0
                contentTick: AppSession.compositeGeneration
                selectionAnts: AppSession.selectionActive
                                && AppSession.selectionShape === "mask"
                Accessible.role: Accessible.Canvas
                Accessible.name: AppSession.hasDocument
                                 ? qsTr("Canvas %1×%2").arg(AppSession.docWidth).arg(AppSession.docHeight)
                                 : qsTr("Empty canvas")
                Accessible.description: qsTr("Document viewport")
            }

            // Document grid overlay (clips to dirty rect when present; full redraw on view bump)
            Canvas {
                id: gridOverlay
                anchors.fill: parent
                z: 2
                visible: AppSession.hasDocument && AppSession.prefShowGrid
                property int lastViewGen: -1
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
                    var viewBump = AppSession.overlayViewGeneration !== gridOverlay.lastViewGen
                    gridOverlay.lastViewGen = AppSession.overlayViewGeneration
                    if (!viewBump && AppSession.dirtyRectJson.length > 2) {
                        try {
                            var d = JSON.parse(AppSession.dirtyRectJson)
                            if (d.length === 4) {
                                var dx0 = root.docToScreenX(d[0])
                                var dy0 = root.docToScreenY(d[1])
                                var dx1 = root.docToScreenX(d[0] + d[2])
                                var dy1 = root.docToScreenY(d[1] + d[3])
                                ctx.beginPath()
                                ctx.rect(Math.min(dx0, dx1) - 1, Math.min(dy0, dy1) - 1,
                                         Math.abs(dx1 - dx0) + 2, Math.abs(dy1 - dy0) + 2)
                                ctx.clip()
                            }
                        } catch (e) { /* full grid */ }
                    }
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
                    function onDirtyRectJsonChanged() { gridOverlay.requestPaint() }
                    function onOverlayViewGenerationChanged() { gridOverlay.requestPaint() }
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

            // Live on-canvas text editor (presentation until bake/commit).
            // Qt Quick TextEdit has no `background` property (Controls TextArea does);
            // wrapping with a Rectangle keeps chrome without aborting QML creation.
            Item {
                id: textCanvasEditorHost
                z: 3
                visible: AppSession.hasDocument && AppSession.textLayerActive
                         && AppSession.activeTool === "tool.text"
                x: root.docToScreenX(AppSession.textOriginX + 4)
                y: root.docToScreenY(AppSession.textOriginY + 4)
                width: Math.max(
                    48,
                    (AppSession.textFrameW > 0
                     ? AppSession.textFrameW
                     : Math.max(64, AppSession.docWidth - AppSession.textOriginX - 8))
                    * AppSession.zoom)
                height: Math.max(
                    28,
                    (AppSession.textFrameH > 0
                     ? AppSession.textFrameH
                     : Math.max(AppSession.textFontSize * 2, 48))
                    * AppSession.zoom)

                Rectangle {
                    anchors.fill: parent
                    color: "#22000000"
                    border.color: Theme.primary
                    border.width: 1
                    radius: 2
                }

                TextEdit {
                    id: textCanvasEditor
                    anchors.fill: parent
                    anchors.margins: 2
                    text: AppSession.textBody
                    color: AppSession.textColorHex
                    selectedTextColor: Theme.colorOnPrimary
                    selectionColor: Theme.primary
                    font.family: AppSession.textFontFamily
                    font.pixelSize: Math.max(6, AppSession.textFontSize * AppSession.zoom)
                    wrapMode: AppSession.textWrap ? TextEdit.Wrap : TextEdit.NoWrap
                    horizontalAlignment: AppSession.textAlignment === 1
                                         ? TextEdit.AlignHCenter
                                         : (AppSession.textAlignment === 2
                                            ? TextEdit.AlignRight : TextEdit.AlignLeft)
                    Accessible.name: qsTr("On-canvas text editor")
                    // Focus can move here from inside a host slot (picking the
                    // Text tool), so never call the host synchronously.
                    onActiveFocusChanged: root.refreshShortcutYield()
                    onTextChanged: {
                        if (activeFocus && text !== AppSession.textBody) {
                            AppSession.updateActiveText(
                                        text,
                                        AppSession.textFontFamily,
                                        AppSession.textFontSize,
                                        AppSession.textTracking,
                                        AppSession.textLineSpacing,
                                        AppSession.textAlignment,
                                        AppSession.textColorHex)
                        }
                    }
                    Connections {
                        target: AppSession
                        function onTextBodyChanged() {
                            if (!textCanvasEditor.activeFocus)
                                textCanvasEditor.text = AppSession.textBody
                        }
                    }
                }
            }
            // Read-only preview when text layer active but Text tool not selected
            Text {
                id: textPreview
                z: 3
                visible: AppSession.hasDocument && AppSession.textLayerActive
                         && AppSession.activeTool !== "tool.text"
                x: root.docToScreenX(AppSession.textOriginX + 4)
                y: root.docToScreenY(AppSession.textOriginY + 4)
                width: Math.max(8, (AppSession.textFrameW > 0
                                    ? AppSession.textFrameW
                                    : AppSession.docWidth - 8) * AppSession.zoom)
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
                              ? Theme.error : root.primary
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
                    onClicked: function (mouse) {
                        root.openContextMenu(selectionContextMenu, this, mouse.x, mouse.y)
                    }
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
                        dashOffset: selectionAnts.visible ? frameClock.phase * 12 : 0
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
                        dashOffset: selectionAnts.visible ? frameClock.phase * 12 + 4 : 0
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
                        if (!canvasInput.painting)
                            return
                        // Clear first so a second tool change cannot queue a
                        // second end, then defer: `activeTool` flips inside
                        // `setActiveTool`, and ending the stroke from here would
                        // re-enter AppSession while it is still borrowed.
                        canvasInput.painting = false
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
                            root.openContextMenu(selectionContextMenu, canvasInput, mouse.x, mouse.y)
                        else
                            root.openContextMenu(canvasContextMenu, canvasInput, mouse.x, mouse.y)
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
                                               canvasInput.strokePressure())
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
                                              canvasInput.strokePressure())
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
            id: rightDock
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: Theme.dockWidth
            color: Theme.surface
            z: 10

            Rectangle {
                anchors.left: parent.left
                width: 1
                height: parent.height
                color: Theme.border
            }

            // Auto-hide edge strip (keyboard/pin reopen — not hover-only).
            Column {
                id: autoHideStrip
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: Theme.spaceXs
                spacing: Theme.spaceXs
                z: 20
                visible: root.autoHiddenPanels.length > 0
                Repeater {
                    model: {
                        var _ = AppSession.dockTopologyJson
                        return root.autoHiddenPanels
                    }
                    ToolButton {
                        id: autoHidePinBtn
                        required property string modelData
                        implicitWidth: Theme.panelHeaderBtn
                        implicitHeight: Theme.panelHeaderBtn
                        padding: 0
                        display: AbstractButton.IconOnly
                        icon.source: root.iconUrl(root.panelIconStem(modelData))
                        icon.width: Theme.iconMd
                        icon.height: Theme.iconMd
                        ToolTip.visible: hovered
                        ToolTip.text: qsTr("Show %1").arg(qsTr(root.panelTitle(modelData)))
                        Accessible.name: ToolTip.text
                        contentItem: Item {
                            implicitWidth: Theme.iconMd
                            implicitHeight: Theme.iconMd
                            ThemedIcon {
                                anchors.centerIn: parent
                                source: autoHidePinBtn.icon.source
                                size: Theme.iconMd
                                color: Theme.iconOnSurfaceEffective
                            }
                        }
                        background: Rectangle {
                            radius: Theme.radiusXs
                            color: autoHidePinBtn.hovered ? Theme.surfaceContainerHigh : "transparent"
                        }
                        onClicked: AppSession.pinPanel(modelData)
                    }
                }
            }

            GridLayout {
                anchors.fill: parent
                anchors.rightMargin: autoHideStrip.visible ? 32 : 0
                columns: 1
                rowSpacing: 0
                columnSpacing: 0

                // Properties panel header
                Rectangle {
                    visible: root.panelShowsInDock("panel.properties")
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
                        PanelTabStrip {
                            panelId: "panel.properties"
                            tabs: root.dockGroupVisibleTabs("panel.properties")
                            Layout.fillWidth: true
                        }
                        PanelHeaderControls {
                            id: propertiesHeaderControls
                            // Properties is the only panel whose body is
                            // disclosure groups, so it is the only one that
                            // carries the panel-local expand/collapse action.
                            showsDisclosureToggle: true
                            anyGroupExpanded: root.anyDisclosureGroupExpanded
                            onDisclosureToggleRequested: {
                                if (root.anyDisclosureGroupExpanded)
                                    AppSession.collapseAllDisclosureGroups()
                                else
                                    AppSession.expandAllDisclosureGroups()
                            }
                            canMoveUp: root.dockStackRow("panel.properties") > 0
                            canMoveDown: root.dockStackRow("panel.properties") >= 0
                                         && root.dockStackRow("panel.properties") < root.dockRightStack.length - 1
                            canTearOff: root.dockRightStack.length > 1
                            onMoveUpRequested: AppSession.movePanelInStack("panel.properties", -1)
                            onMoveDownRequested: AppSession.movePanelInStack("panel.properties", 1)
                            onAutoHideRequested: AppSession.togglePanelAutoHide("panel.properties")
                            onTearOffRequested: root.tearOffAndClamp("panel.properties",
                                                                       root.x + root.width - 360, root.y + 80, 320, 400)
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        // Leave room for panel chrome controls so they receive
                        // clicks. Measured, not a literal: the buttons scale
                        // with density and the Properties header carries five.
                        anchors.rightMargin: propertiesHeaderControls.width + Theme.spaceXs
                        z: -1
                        property real pressY: 0
                        cursorShape: Qt.SizeVerCursor
                        onPressed: function (mouse) { pressY = mouse.y }
                        onClicked: {
                            AppSession.setWorkspaceFocusPath("panel.properties")
                            AppSession.setWorkspacePanelContext("panel.properties")
                        }
                        onReleased: function (mouse) {
                            root.commitHeaderDrag("panel.properties", mouse.y - pressY)
                        }
                    }
                }

                Flickable {
                    visible: root.panelShowsInDock("panel.properties")
                    Layout.row: root.dockStackRow("panel.properties") * 2 + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                    // Cap height so Layers/History stay above the status bar. Never go
                    // negative when parent.height is 0 during the first layout pass —
                    // negative preferredHeight collapses the whole right dock to width 0.
                    Layout.preferredHeight: {
                        if (!visible)
                            return 0
                        var h = parent.height
                        if (h <= 0)
                            return 0
                        return Math.min(h * 0.42, Math.max(0, h - Theme.dockStackReserve))
                    }
                    contentHeight: propsCol.implicitHeight
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: ScrollBar {
                        policy: ScrollBar.AsNeeded
                    }

                    PropertiesPanel {
                        id: propsCol
                        width: parent.width
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.margins: Theme.spaceMd
                        spacing: Theme.spaceMd

                        adjRange: root.adjRange
                        iconUrl: root.iconUrl
                        runAction: root.runAction
                        isTransformTool: root.isTransformTool
                        isCropTool: root.isCropTool
                        isSelectTool: root.isSelectTool
                        selectionCombineLabel: root.selectionCombineLabel
                        activeLayerKind: root.activeLayerKind
                        activeLayerHasMask: root.activeLayerHasMask
                        activeMaskEnabled: root.activeMaskEnabled
                        gpuStatus: gpuCanvas.gpuStatus
                        onEmbedIccRequested: embedIccFileDialog.open()
                    }
                }

                // Navigator
                Rectangle {
                    visible: root.panelShowsInDock("panel.navigator")
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
                        PanelTabStrip {
                            panelId: "panel.navigator"
                            tabs: root.dockGroupVisibleTabs("panel.navigator")
                            Layout.fillWidth: true
                        }
                        PanelHeaderControls {
                            id: navigatorHeaderControls
                            canMoveUp: root.dockStackRow("panel.navigator") > 0
                            canMoveDown: root.dockStackRow("panel.navigator") >= 0
                                         && root.dockStackRow("panel.navigator") < root.dockRightStack.length - 1
                            canTearOff: root.dockRightStack.length > 1
                            onMoveUpRequested: AppSession.movePanelInStack("panel.navigator", -1)
                            onMoveDownRequested: AppSession.movePanelInStack("panel.navigator", 1)
                            onAutoHideRequested: AppSession.togglePanelAutoHide("panel.navigator")
                            onTearOffRequested: root.tearOffAndClamp("panel.navigator",
                                                                       root.x + root.width - 360, root.y + 120, 320, 280)
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        // Leave room for panel chrome controls so they receive
                        // clicks. Measured, not a literal: buttons scale with density.
                        anchors.rightMargin: navigatorHeaderControls.width + Theme.spaceXs
                        z: -1
                        property real pressY: 0
                        cursorShape: Qt.SizeVerCursor
                        onPressed: function (mouse) { pressY = mouse.y }
                        onReleased: function (mouse) {
                            root.commitHeaderDrag("panel.navigator", mouse.y - pressY)
                        }
                    }
                }

                Rectangle {
                    id: navigatorPane
                    visible: root.panelShowsInDock("panel.navigator")
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
                    visible: root.panelShowsInDock("panel.swatches")
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
                        PanelTabStrip {
                            panelId: "panel.swatches"
                            tabs: root.dockGroupVisibleTabs("panel.swatches")
                            Layout.fillWidth: true
                        }
                        PanelHeaderControls {
                            id: swatchesHeaderControls
                            canMoveUp: root.dockStackRow("panel.swatches") > 0
                            canMoveDown: root.dockStackRow("panel.swatches") >= 0
                                         && root.dockStackRow("panel.swatches") < root.dockRightStack.length - 1
                            canTearOff: root.dockRightStack.length > 1
                            onMoveUpRequested: AppSession.movePanelInStack("panel.swatches", -1)
                            onMoveDownRequested: AppSession.movePanelInStack("panel.swatches", 1)
                            onAutoHideRequested: AppSession.togglePanelAutoHide("panel.swatches")
                            onTearOffRequested: root.tearOffAndClamp("panel.swatches",
                                                                       root.x + root.width - 360, root.y + 160, 320, 280)
                        }
                        ToolButton {
                            id: swapFgBgBtn
                            implicitWidth: Theme.panelHeaderBtn
                            implicitHeight: Theme.panelHeaderBtn
                            padding: 0
                            leftPadding: 0
                            rightPadding: 0
                            topPadding: 0
                            bottomPadding: 0
                            display: AbstractButton.IconOnly
                            icon.source: root.iconUrl("arrows-left-right")
                            icon.width: Theme.iconMd
                            icon.height: Theme.iconMd
                            contentItem: Item {
                                implicitWidth: Theme.iconMd
                                implicitHeight: Theme.iconMd
                                ThemedIcon {
                                    anchors.centerIn: parent
                                    source: swapFgBgBtn.icon.source
                                    size: Theme.iconMd
                                    color: swapFgBgBtn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                                }
                            }
                            background: Rectangle {
                                radius: Theme.radiusXs
                                color: swapFgBgBtn.hovered ? Theme.surfaceContainerHigh : "transparent"
                            }
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Swap foreground / background")
                            Accessible.name: qsTr("Swap foreground / background")
                            onClicked: AppSession.swapFgBg()
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        // Leave room for panel chrome controls so they receive
                        // clicks. Measured, not a literal: buttons scale with density.
                        anchors.rightMargin: swatchesHeaderControls.width + Theme.spaceXs
                        z: -1
                        property real pressY: 0
                        cursorShape: Qt.SizeVerCursor
                        onPressed: function (mouse) { pressY = mouse.y }
                        onReleased: function (mouse) {
                            root.commitHeaderDrag("panel.swatches", mouse.y - pressY)
                        }
                    }
                }

                Rectangle {
                    visible: root.panelShowsInDock("panel.swatches")
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
                                        onClicked: {
                                            Qt.callLater(function () {
                                                hexField.forceActiveFocus()
                                                AppSession.setShortcutInputYield(true)
                                            })
                                        }
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
                                Accessible.name: qsTr("Foreground hex")
                                font.family: "Noto Sans Mono"
                                font.pixelSize: Theme.fontMono
                                color: Theme.colorOnSurface
                                background: Rectangle {
                                    color: Theme.surfaceContainer
                                    border.color: parent.activeFocus ? Theme.primary : Theme.border
                                    radius: Theme.radiusSm
                                }
                                onActiveFocusChanged: root.refreshShortcutYield()
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
                    visible: root.panelShowsInDock("panel.layers")
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
                        PanelTabStrip {
                            panelId: "panel.layers"
                            tabs: root.dockGroupVisibleTabs("panel.layers")
                            Layout.fillWidth: true
                        }
                        PanelHeaderControls {
                            id: layersHeaderControls
                            canMoveUp: root.dockStackRow("panel.layers") > 0
                            canMoveDown: root.dockStackRow("panel.layers") >= 0
                                         && root.dockStackRow("panel.layers") < root.dockRightStack.length - 1
                            canTearOff: root.dockRightStack.length > 1
                            onMoveUpRequested: AppSession.movePanelInStack("panel.layers", -1)
                            onMoveDownRequested: AppSession.movePanelInStack("panel.layers", 1)
                            onAutoHideRequested: AppSession.togglePanelAutoHide("panel.layers")
                            onTearOffRequested: root.tearOffAndClamp("panel.layers",
                                                                       root.x + root.width - 360, root.y + 200, 320, 360)
                        }
                        ToolButton {
                            id: addLayerBtn
                            implicitWidth: Theme.panelHeaderBtn
                            implicitHeight: Theme.panelHeaderBtn
                            padding: 0
                            leftPadding: 0
                            rightPadding: 0
                            topPadding: 0
                            bottomPadding: 0
                            display: AbstractButton.IconOnly
                            icon.source: root.iconUrl("plus")
                            icon.width: Theme.iconMd
                            icon.height: Theme.iconMd
                            enabled: AppSession.hasDocument
                            contentItem: Item {
                                implicitWidth: Theme.iconMd
                                implicitHeight: Theme.iconMd
                                ThemedIcon {
                                    anchors.centerIn: parent
                                    source: addLayerBtn.icon.source
                                    size: Theme.iconMd
                                    color: addLayerBtn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                                }
                            }
                            background: Rectangle {
                                radius: Theme.radiusXs
                                color: addLayerBtn.hovered && addLayerBtn.enabled ? Theme.surfaceContainerHigh : "transparent"
                            }
                            onClicked: AppSession.addLayer()
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Add layer")
                            Accessible.name: qsTr("Add layer")
                        }
                        ToolButton {
                            id: addGroupBtn
                            implicitWidth: Theme.panelHeaderBtn
                            implicitHeight: Theme.panelHeaderBtn
                            padding: 0
                            leftPadding: 0
                            rightPadding: 0
                            topPadding: 0
                            bottomPadding: 0
                            display: AbstractButton.IconOnly
                            icon.source: root.iconUrl("folder")
                            icon.width: Theme.iconMd
                            icon.height: Theme.iconMd
                            enabled: AppSession.hasDocument
                            contentItem: Item {
                                implicitWidth: Theme.iconMd
                                implicitHeight: Theme.iconMd
                                ThemedIcon {
                                    anchors.centerIn: parent
                                    source: addGroupBtn.icon.source
                                    size: Theme.iconMd
                                    color: addGroupBtn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                                }
                            }
                            background: Rectangle {
                                radius: Theme.radiusXs
                                color: addGroupBtn.hovered && addGroupBtn.enabled ? Theme.surfaceContainerHigh : "transparent"
                            }
                            onClicked: AppSession.addGroupLayer()
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Add group")
                            Accessible.name: qsTr("Add group")
                        }
                        ToolButton {
                            id: delLayerBtn
                            implicitWidth: Theme.panelHeaderBtn
                            implicitHeight: Theme.panelHeaderBtn
                            padding: 0
                            leftPadding: 0
                            rightPadding: 0
                            topPadding: 0
                            bottomPadding: 0
                            display: AbstractButton.IconOnly
                            icon.source: root.iconUrl("trash")
                            icon.width: Theme.iconMd
                            icon.height: Theme.iconMd
                            enabled: AppSession.hasDocument && AppSession.layerCount > 1
                            contentItem: Item {
                                implicitWidth: Theme.iconMd
                                implicitHeight: Theme.iconMd
                                ThemedIcon {
                                    anchors.centerIn: parent
                                    source: delLayerBtn.icon.source
                                    size: Theme.iconMd
                                    color: delLayerBtn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                                }
                            }
                            background: Rectangle {
                                radius: Theme.radiusXs
                                color: delLayerBtn.hovered && delLayerBtn.enabled ? Theme.surfaceContainerHigh : "transparent"
                            }
                            onClicked: AppSession.deleteActiveLayer()
                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("Delete layer")
                            Accessible.name: qsTr("Delete layer")
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        // Leave room for panel chrome controls so they receive
                        // clicks. Measured, not a literal: buttons scale with density.
                        anchors.rightMargin: layersHeaderControls.width + Theme.spaceXs
                        z: -1
                        property real pressY: 0
                        cursorShape: Qt.SizeVerCursor
                        onPressed: function (mouse) { pressY = mouse.y }
                        onReleased: function (mouse) {
                            root.commitHeaderDrag("panel.layers", mouse.y - pressY)
                        }
                    }
                }

                Rectangle {
                    visible: root.panelShowsInDock("panel.layers")
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
                        reuseItems: true
                        cacheBuffer: Theme.toolHit * 4
                        model: AppSession.layerModel
                        delegate: Rectangle {
                            width: layerList.width
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
                                        layerContextMenu.targetIndex = stack_index
                                        root.openContextMenu(layerContextMenu, this, mouse.x, mouse.y)
                                    }
                                }
                                onPressAndHold: {
                                    layerContextMenu.targetIndex = stack_index
                                    root.openContextMenu(layerContextMenu, this, width / 2, height / 2)
                                }
                            }
                        }
                    }
                }

                // History panel
                Rectangle {
                    visible: root.panelShowsInDock("panel.history")
                    Layout.row: root.dockStackRow("panel.history") * 2
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? Theme.panelHeaderHeight : 0
                    color: Theme.surfaceContainer
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceSm
                        anchors.rightMargin: Theme.spaceXs
                        PanelTabStrip {
                            panelId: "panel.history"
                            tabs: root.dockGroupVisibleTabs("panel.history")
                            Layout.fillWidth: true
                        }
                        PanelHeaderControls {
                            id: historyHeaderControls
                            canMoveUp: root.dockStackRow("panel.history") > 0
                            canMoveDown: root.dockStackRow("panel.history") >= 0
                                         && root.dockStackRow("panel.history") < root.dockRightStack.length - 1
                            canTearOff: root.dockRightStack.length > 1
                            onMoveUpRequested: AppSession.movePanelInStack("panel.history", -1)
                            onMoveDownRequested: AppSession.movePanelInStack("panel.history", 1)
                            onAutoHideRequested: AppSession.togglePanelAutoHide("panel.history")
                            onTearOffRequested: root.tearOffAndClamp("panel.history",
                                                                       root.x + root.width - 360, root.y + 240, 320, 240)
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        // Leave room for panel chrome controls so they receive
                        // clicks. Measured, not a literal: buttons scale with density.
                        anchors.rightMargin: historyHeaderControls.width + Theme.spaceXs
                        z: -1
                        property real pressY: 0
                        cursorShape: Qt.SizeVerCursor
                        onPressed: function (mouse) { pressY = mouse.y }
                        onReleased: function (mouse) {
                            root.commitHeaderDrag("panel.history", mouse.y - pressY)
                        }
                    }
                }
                Rectangle {
                    visible: root.panelShowsInDock("panel.history")
                    Layout.row: root.dockStackRow("panel.history") * 2 + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: visible ? 120 : 0
                    color: Theme.surfaceSunken
                    ListView {
                        id: historyList
                        anchors.fill: parent
                        clip: true
                        reuseItems: true
                        cacheBuffer: 88
                        model: root.historyLabelParts
                        delegate: Label {
                            width: historyList.width
                            height: 22
                            leftPadding: Theme.spaceSm
                            text: {
                                var kind = index < root.historyKindParts.length
                                           ? root.historyKindParts[index] : ""
                                return kind.length > 0 ? (modelData + " · " + kind) : modelData
                            }
                            color: Theme.colorOnSurfaceVariant
                            font.pixelSize: Theme.fontBodySm
                            elide: Text.ElideRight
                            Accessible.name: text
                            MouseArea {
                                anchors.fill: parent
                                onClicked: {
                                    if (index < root.historyIdParts.length)
                                        AppSession.jumpHistoryEntry(Number(root.historyIdParts[index]))
                                }
                            }
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
        } // mainPanes (tool strip + canvas + docks)
    } // ColumnLayout (tabs + main)

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
        onRejected: {
            if (!AppSession.hasDocument && !AppSession.ioBusy)
                welcomeDialog.open()
        }
    }

    FileDialog {
        id: embedIccFileDialog
        title: qsTr("Embed ICC Profile")
        currentFolder: StandardPaths.writableLocation(StandardPaths.HomeLocation)
        fileMode: FileDialog.OpenFile
        nameFilters: [
            qsTr("ICC profiles (*.icc *.icm)"),
            qsTr("All files (*)")
        ]
        onAccepted: AppSession.embedIccFromFile(selectedFile.toString())
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

    LazyDialog {
        id: unsavedDialogLoader

        Dialog {
            id: unsavedDialog
            parent: Overlay.overlay
            anchors.centerIn: parent
            modal: true
            title: qsTr("Unsaved changes")
            header: ThemedDialogHeader { text: unsavedDialog.title }
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
    }

    LazyDialog {
        id: compatibilityDialogLoader
        requested: AppSession.compatibilityReport.length > 0

        Dialog {
            id: compatibilityDialog
            parent: Overlay.overlay
            anchors.centerIn: parent
            modal: true
            title: qsTr("Compatibility report")
            header: ThemedDialogHeader { text: compatibilityDialog.title }
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
    }

    LazyDialog {
        id: ioErrorDialogLoader

        Dialog {
            id: ioErrorDialog
            parent: Overlay.overlay
            anchors.centerIn: parent
            modal: true
            title: qsTr("File operation failed")
            header: ThemedDialogHeader { text: ioErrorDialog.title }
            standardButtons: Dialog.Ok
            width: Math.min(560, parent ? parent.width - 48 : 560)

            background: Rectangle {
                color: Theme.surface
                border.color: Theme.border
                radius: Theme.radiusMd
            }

            contentItem: ScrollView {
                clip: true
                implicitHeight: Math.min(320, ioErrorLabel.implicitHeight + 16)
                Label {
                    id: ioErrorLabel
                    width: ioErrorDialog.availableWidth
                    text: AppSession.ioError
                    wrapMode: Text.WordWrap
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                    Accessible.name: qsTr("File error details")
                }
            }
        }
    }

    LazyDialog {
        id: filterGalleryDialogLoader
        requested: AppSession.filterGalleryOpen

        FilterGalleryDialog {
            afterHostSlot: root.afterHostSlot
        }
    }

    LazyDialog {
        id: preferencesDialogLoader
        requested: AppSession.preferencesOpen

        PreferencesDialog {
            actionDescriptors: root.actionDescriptors
            afterHostSlot: root.afterHostSlot
            dockRightStack: root.dockRightStack
            iconUrl: root.iconUrl
            panelDescriptors: root.panelDescriptors
            panelIsVisible: root.panelIsVisible
            shortcutForAction: root.shortcutForAction
            hostHeight: root.height
        }
    }


    LazyDialog {
        id: commandPaletteLoader

        CommandPaletteDialog {
            actionDescriptors: root.actionDescriptors
            actionIsEnabled: root.actionIsEnabled
            refreshShortcutYield: root.refreshShortcutYield
            runAction: root.runAction
            shortcutForAction: root.shortcutForAction
            hostWidth: root.width
            hostHeight: root.height
        }
    }


    LazyDialog {
        id: aboutDialogLoader

        Dialog {
            id: aboutDialog
            parent: Overlay.overlay
            anchors.centerIn: parent
            modal: true
            title: qsTr("About PhotoTux")
            header: ThemedDialogHeader { text: aboutDialog.title }
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
    }

    WelcomeDialog {
        id: welcomeDialog
        anchors.centerIn: parent
        onRequestNew: root.openNewDocumentDialog()
        onRequestOpen: openFileDialog.open()
    }

    NewDocumentDialog {
        id: newDocDialog
        anchors.centerIn: parent
        onOpened: {
            welcomeDialog.close()
            root.refreshShortcutYield()
        }
        onClosed: root.refreshShortcutYield()
        onCreateRequested: function (presetLabel, w, h) {
            welcomeDialog.close()
            AppSession.setViewportSize(canvasHost.width, canvasHost.height)
            if (presetLabel && presetLabel.length > 0)
                AppSession.applySizePreset(presetLabel)
            else
                AppSession.applyDocumentSize(w, h)
            if (typeof propsCol !== "undefined" && propsCol)
                propsCol.setLayerOpacity(AppSession.activeOpacity)
            root.syncBlendCombo()
        }
    }

    Connections {
        target: AppSession
        function onHasDocumentChanged() {
            if (AppSession.hasDocument)
                welcomeDialog.close()
            else if (!AppSession.ioBusy
                     && !openFileDialog.visible
                     && !newDocDialog.visible
                     && !recoveryDialogLoader.dialogVisible)
                welcomeDialog.open()
        }
    }

    Connections {
        target: AppSession
        function onActiveOpacityChanged() {
            if (typeof propsCol !== "undefined" && propsCol)
                propsCol.setLayerOpacity(AppSession.activeOpacity)
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
                ioErrorDialogLoader.open()
        }
    }

}
