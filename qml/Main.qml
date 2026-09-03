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
    /// Where the next file dialog opens when there is no document to follow.
    ///
    /// Each `FileDialog` owns its own `currentFolder`, so four dialogs meant
    /// four independent memories and none of them shared what the user had
    /// just done: saving into `~/work` and then pressing Ctrl+O landed back in
    /// Pictures. Photoshop follows the open document first and the last folder
    /// second, and so does `browseForFile`.
    property url lastBrowsedFolder: StandardPaths.writableLocation(StandardPaths.PicturesLocation)
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
    /// The blend-mode vocabulary, as the engine declares it.
    ///
    /// The combo used to carry its own list of eight, so the other nineteen
    /// modes — every component mode among them — were unreachable from the
    /// chrome however well the compositor drew them.
    readonly property var blendModes: {
        try {
            return JSON.parse(AppSession.blendModesJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Tool shelf as slots, the way Photoshop stacks it.
    readonly property var toolSlots: {
        try {
            return JSON.parse(AppSession.toolSlotsJson || "[]")
        } catch (e) {
            return []
        }
    }
    /// Slot key → the tool last picked from that slot.
    ///
    /// Photoshop's shelf remembers which member of a stack you were using, so
    /// the button keeps showing the burn tool once you have chosen it rather
    /// than snapping back to dodge. Session-local by design: it is a working
    /// habit, not a setting worth persisting into prefs.
    property var toolSlotChoice: ({})

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

    /// Live per-panel body heights while a grip is being dragged.
    ///
    /// The committed value lives in the dock topology; this is the in-flight
    /// one, so the panel follows the pointer without writing preferences on
    /// every motion event.
    property var panelHeightDrafts: ({})
    /// Bumped whenever a draft changes.
    ///
    /// A binding that reads a `var` property does not reliably re-evaluate when
    /// that property is reassigned — the resolved map went stale after the
    /// first drag step while the drag itself kept reporting the right numbers.
    /// An int changes by value and always notifies, so the bindings read this
    /// as well; the same `var _ = …` idiom the tool shelf already uses to make
    /// a dependency explicit.
    property int panelHeightRevision: 0

    /// The height the shell gives a panel that the user has never dragged.
    ///
    /// The one home for these numbers. `-1` means the panel is not resizable:
    /// Swatches sizes to its content and Layers takes whatever is left, so
    /// neither has a height of its own to drag — resizing Layers really means
    /// resizing everything above it, which is what the other grips already do.
    function panelDefaultHeight(panelId, dockHeight) {
        switch (panelId) {
        case "panel.properties":
            // Keeps Layers and History above the status bar in an untouched
            // dock. Only a default now — a dragged height overrides it.
            return Math.min(dockHeight * 0.42,
                            Math.max(0, dockHeight - Theme.dockStackReserve))
        case "panel.navigator":
            return 132
        case "panel.history":
            return 120
        }
        return -1
    }

    function panelIsResizable(panelId, dockHeight) {
        return root.panelDefaultHeight(panelId, dockHeight) >= 0
    }

    /// The docked panel immediately above `panelId`, or "" when it is the first.
    ///
    /// A group's identity for sizing is its *active tab*, because that is the
    /// panel whose body is on screen and therefore the one a seam resizes.
    function panelAboveInStack(panelId) {
        var row = root.dockStackRow(panelId)
        if (row <= 0 || row >= 1000)
            return ""
        var groups = root.dockGroups
        var above = groups[row - 1]
        if (!above)
            return ""
        return above.active || (above.tabs || [])[0] || ""
    }

    /// Panel id → the height the user has chosen, live drag winning over the
    /// committed value. Absent means "the shell decides".
    ///
    /// A property rather than a function, because a binding that resolves a
    /// height by *calling* a helper did not re-evaluate when the drafts
    /// changed: the panel stayed put while the drag reported the right numbers
    /// the whole way. Bindings index this directly, so the dependency is a
    /// plain property read and cannot be missed.
    readonly property var resolvedPanelHeights: {
        var _ = root.panelHeightRevision
        var out = {}
        var stored = root.dockTopology.panel_heights || {}
        for (var id in stored)
            out[id] = stored[id]
        var drafts = root.panelHeightDrafts
        for (var live in drafts)
            out[live] = drafts[live]
        return out
    }

    // Both of these build a *new* object rather than mutating and reassigning.
    // A `property var` holding the same reference emits no change signal, so
    // mutating in place left every binding on it stale — the panel did not
    // follow the drag, and the commit then wrote back the height it started
    // with.
    function setPanelHeightDraft(panelId, height) {
        var drafts = Object.assign({}, root.panelHeightDrafts)
        drafts[panelId] = height
        root.panelHeightDrafts = drafts
        root.panelHeightRevision += 1
    }

    function commitPanelHeight(panelId, height) {
        var drafts = Object.assign({}, root.panelHeightDrafts)
        delete drafts[panelId]
        root.panelHeightDrafts = drafts
        root.panelHeightRevision += 1
        AppSession.setPanelHeight(panelId, height)
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
        // Dense packing: 2px gaps match toolColumn.spacing.
        // Literal hit size: Theme.toolHit can resolve to 0 under PHOTOTUX_QML.
        var gap = 2
        var hit = 40
        var row = hit + gap
        var bare = Math.max(1, Math.floor((stripHeight - gap) / row))
        // The overflow button's own space is reserved only when it is going to
        // exist. Subtracting it unconditionally *created* overflow at the
        // boundary: a shelf with room for exactly every slot lost its last one
        // to a menu, to make space for the button that menu needed.
        if (bare >= root.toolSlots.length)
            return bare
        var reserve = hit + 8
        return Math.max(1, Math.floor((stripHeight - reserve - gap) / row))
    }

    /// Whether `slot` holds the active tool.
    function slotHoldsActive(slot) {
        var list = slot.tools || []
        for (var i = 0; i < list.length; ++i) {
            if (list[i].id === AppSession.activeTool)
                return true
        }
        return false
    }

    /// The tool a slot's button stands for: the active one when the slot holds
    /// it, otherwise the one last picked from the slot, otherwise the first.
    function slotFace(slot) {
        var list = slot.tools || []
        if (list.length === 0)
            return ({ id: "", title: "", icon: "", shortcut: "" })
        for (var i = 0; i < list.length; ++i) {
            if (list[i].id === AppSession.activeTool)
                return list[i]
        }
        var remembered = root.toolSlotChoice[slot.slot]
        for (var j = 0; j < list.length; ++j) {
            if (list[j].id === remembered)
                return list[j]
        }
        return list[0]
    }

    /// Split slots into visible shelf vs overflow menu; keep the active slot on
    /// the shelf. Slots make this a safety net rather than the usual case —
    /// eighteen of them fit a maximized 1080p window with room to spare.
    function toolStripPartitions(capacity) {
        var all = root.toolSlots
        var cap = Math.max(1, capacity)
        if (all.length <= cap)
            return ({ visible: all, overflow: [] })
        var visible = all.slice(0, cap)
        var overflow = all.slice(cap)
        var oi = -1
        for (var i = 0; i < overflow.length; ++i) {
            if (root.slotHoldsActive(overflow[i])) {
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

    function activateToolFromSlot(slotKey, toolId) {
        var choice = root.toolSlotChoice
        choice[slotKey] = toolId
        root.toolSlotChoice = choice
        root.activateToolFromStrip(toolId)
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
        case "action.layer.toggle-clip":
            return root.activeLayerClips
        }
        // Every Window-menu panel toggle reads the one registry map, so a new
        // panel needs no case of its own. It used to say that above a list of
        // seven cases, two of which named panels this shell does not draw.
        if (actionId.indexOf(root.panelActionPrefix) === 0)
            return root.panelIsVisible(root.panelIdForAction(actionId))
        return false
    }

    /// Window-menu panel toggles share this id prefix; see `panel_action_id`.
    readonly property string panelActionPrefix: "action.window.panel-"

    function panelIdForAction(actionId) {
        return "panel." + actionId.substring(root.panelActionPrefix.length)
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
        case "action.layer.toggle-clip":
            AppSession.setClipsToBelowOnActive(checked)
            break
        default:
            if (actionId.indexOf(root.panelActionPrefix) === 0)
                AppSession.setPanelVisible(root.panelIdForAction(actionId), checked)
            else
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
    /// Open a file dialog where the user last was.
    ///
    /// The open document wins when there is one — that is the folder the next
    /// Save As, Export or Open almost always means — and `lastBrowsedFolder`
    /// carries the answer between documents. Assigned rather than bound,
    /// because navigating inside the dialog writes `currentFolder` and a
    /// binding would drag the user back out of the folder they just entered.
    function browseForFile(dialog) {
        var folder = root.documentFolder()
        dialog.currentFolder = folder.length > 0 ? folder : root.lastBrowsedFolder
        dialog.open()
    }

    /// The open document's folder as a `file:` url string, or `""`.
    ///
    /// `documentPath` is a plain filesystem path, so it has to be percent
    /// encoded before it can stand in for a url — a folder with a space in it
    /// otherwise produces a url the dialog cannot resolve and silently ignores.
    function documentFolder() {
        var path = AppSession.documentPath || ""
        var cut = path.lastIndexOf("/")
        if (cut <= 0)
            return ""
        return "file://" + encodeURI(path.substring(0, cut))
    }

    function handleHostRequest(kind) {
        if (!kind)
            return
        switch (kind) {
        case "document.new":
            root.openNewDocumentDialog()
            break
        case "document.open":
            welcomeDialog.close()
            root.browseForFile(openFileDialog)
            break
        case "document.save_as":
            root.browseForFile(saveFileDialog)
            break
        case "document.export":
            root.browseForFile(exportFileDialog)
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
            root.browseForFile(embedIccFileDialog)
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
    /// Whether the active tool paints dabs — the brush, the eraser and every
    /// retouch tool, which are all one brush with a different dab mode.
    function isDabTool() {
        var t = AppSession.activeTool
        return t === "tool.brush" || t === "tool.eraser"
                || t === "tool.clone" || t === "tool.dodge" || t === "tool.burn"
                || t === "tool.sponge" || t === "tool.blur" || t === "tool.sharpen"
                || t === "tool.smudge"
    }
    function isSelectTool() {
        return AppSession.activeTool === "tool.select.rect"
                || AppSession.activeTool === "tool.select.ellipse"
    }
    function isLassoTool() {
        return AppSession.activeTool === "tool.select.lasso"
    }
    // The combo itself lives in the Layers panel's control strip; the shell
    // only asks it to resync. Guarded because the strip is built with the right
    // dock, and host state can change before that happens.
    function syncBlendCombo() {
        if (typeof layerControls !== "undefined" && layerControls)
            layerControls.syncBlendCombo()
    }
    function syncLayerOpacity() {
        if (typeof layerControls !== "undefined" && layerControls)
            layerControls.setLayerOpacity(AppSession.activeOpacity)
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
            root.browseForFile(openFileDialog)
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
        ThemedMenuItem {
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

    ThemedMenu {
        id: layerContextMenu
        property int targetIndex: -1
        Instantiator {
            model: root.actionsForContext("layer")
            delegate: ThemedMenuItem {
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

    ThemedMenu {
        id: canvasContextMenu
        Instantiator {
            model: root.actionsForContext("canvas")
            delegate: actionMenuItem
            onObjectAdded: (index, object) => canvasContextMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => canvasContextMenu.removeItem(object)
        }
    }

    ThemedMenu {
        id: selectionContextMenu
        Instantiator {
            model: root.actionsForContext("selection")
            delegate: actionMenuItem
            onObjectAdded: (index, object) => selectionContextMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => selectionContextMenu.removeItem(object)
        }
    }

    menuBar: MenuBar {
        id: mainMenuBar

        // The comment here used to say Fusion paints the bar and ignores a
        // custom background. The shell links only the **Basic** style plugin,
        // which honours both — so the bar had been shipping as a light strip
        // above dark chrome on the strength of a style it does not run.
        delegate: MenuBarItem {
            id: barItem
            padding: Theme.spaceSm
            leftPadding: Theme.spaceMd
            rightPadding: Theme.spaceMd
            background: Rectangle {
                radius: Theme.radiusSm
                color: barItem.highlighted || barItem.down
                       ? Theme.toolActiveBg
                       : (barItem.hovered ? Theme.surfaceContainerHigh : "transparent")
            }
            contentItem: Text {
                text: Theme.withoutMnemonic(barItem.text)
                font: barItem.font
                color: barItem.enabled ? Theme.colorOnSurfaceEffective
                                       : Theme.colorOnSurfaceDisabled
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        background: Rectangle {
            color: Theme.surfaceContainer
            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: Theme.border
            }
        }

        ThemedMenu {
            id: fileMenu
            title: qsTr("&File")
            Instantiator {
                model: root.actionsForMenu("file")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => fileMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => fileMenu.removeItem(object)
            }
        }
        ThemedMenu {
            id: editMenu
            title: qsTr("&Edit")
            Instantiator {
                model: root.actionsForMenu("edit")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => editMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => editMenu.removeItem(object)
            }
            ThemedMenu {
                id: editTransformMenu
                title: qsTr("&Transform")
                Instantiator {
                    model: root.actionsForMenu("edit.transform")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => editTransformMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => editTransformMenu.removeItem(object)
                }
            }
        }
        ThemedMenu {
            id: imageMenu
            title: qsTr("&Image")
            // Declared submenus land after the instantiated entries, which is
            // the order Photoshop uses: Image Size, then Image Rotation, then
            // the colour-management group — eight flat entries until they were
            // given a menu of their own.
            ThemedMenu {
                id: imageRotationMenu
                title: qsTr("Image &Rotation")
                Instantiator {
                    model: root.actionsForMenu("image.rotation")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => imageRotationMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => imageRotationMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: imageColorMenu
                title: qsTr("&Color")
                Instantiator {
                    model: root.actionsForMenu("image.color")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => imageColorMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => imageColorMenu.removeItem(object)
                }
            }
            Instantiator {
                model: root.actionsForMenu("image")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => imageMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => imageMenu.removeItem(object)
            }
        }
        ThemedMenu {
            id: layerMenu
            title: qsTr("&Layer")
            Instantiator {
                model: root.actionsForMenu("layer")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => layerMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => layerMenu.removeItem(object)
            }
            // Ten submenus, because the flat Layer menu carried thirty-one
            // entries and ran past the bottom of a 1080p window — the last of
            // them unreachable however correctly they rendered. Each is
            // declared here and populated from the engine; a submenu the
            // engine declares and this file does not is a test failure.
            ThemedMenu {
                id: adjustmentMenu
                title: qsTr("New &Adjustment Layer")
                Instantiator {
                    model: root.actionsForMenu("layer.adjustment")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => adjustmentMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => adjustmentMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: shapeMenu
                title: qsTr("&Shape")
                Instantiator {
                    model: root.actionsForMenu("layer.shape")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => shapeMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => shapeMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: smartMenu
                title: qsTr("Smart &Objects")
                Instantiator {
                    model: root.actionsForMenu("layer.smart")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => smartMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => smartMenu.removeItem(object)
                }
            }
            // Photoshop files Arrange between the group entries and Combine
            // Shapes, next to Align and Distribute — the other things that
            // move a layer without changing what is in it.
            ThemedMenu {
                id: arrangeMenu
                title: qsTr("&Arrange")
                Instantiator {
                    model: root.actionsForMenu("layer.arrange")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => arrangeMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => arrangeMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: booleanMenu
                title: qsTr("&Combine Shapes")
                Instantiator {
                    model: root.actionsForMenu("layer.boolean")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => booleanMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => booleanMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: styleMenu
                title: qsTr("Layer St&yle")
                Instantiator {
                    model: root.actionsForMenu("layer.style")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => styleMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => styleMenu.removeItem(object)
                }
            }
            // Align and Distribute sit directly under Layer, next to each
            // other, the way Photoshop files them.
            ThemedMenu {
                id: alignMenu
                title: qsTr("Ali&gn")
                Instantiator {
                    model: root.actionsForMenu("layer.align")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => alignMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => alignMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: distributeMenu
                title: qsTr("&Distribute")
                Instantiator {
                    model: root.actionsForMenu("layer.distribute")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => distributeMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => distributeMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: maskMenu
                title: qsTr("&Mask")
                Instantiator {
                    model: root.actionsForMenu("layer.mask")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => maskMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => maskMenu.removeItem(object)
                }
            }
            ThemedMenu {
                id: lockMenu
                title: qsTr("Loc&k")
                Instantiator {
                    model: root.actionsForMenu("layer.lock")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => lockMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => lockMenu.removeItem(object)
                }
            }
        }
        // Select sits between Layer and Filter, which is where Photoshop keeps
        // it. It used to be between Edit and Image, so anyone reaching for
        // Image by position opened Select instead — the exact relearning cost
        // matching Photoshop's layout is meant to remove.
        ThemedMenu {
            id: selectMenu
            title: qsTr("&Select")
            Instantiator {
                model: root.actionsForMenu("select")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => selectMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => selectMenu.removeItem(object)
            }
            ThemedMenu {
                id: selectModifyMenu
                title: qsTr("&Modify")
                Instantiator {
                    model: root.actionsForMenu("select.modify")
                    delegate: actionMenuItem
                    onObjectAdded: (index, object) => selectModifyMenu.insertItem(index, object)
                    onObjectRemoved: (index, object) => selectModifyMenu.removeItem(object)
                }
            }
        }
        ThemedMenu {
            id: filterMenu
            title: qsTr("Filte&r")
            Instantiator {
                model: root.actionsForMenu("filter")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => filterMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => filterMenu.removeItem(object)
            }
        }
        ThemedMenu {
            id: viewMenu
            title: qsTr("&View")
            Instantiator {
                model: root.actionsForMenu("view")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => viewMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => viewMenu.removeItem(object)
            }
        }
        ThemedMenu {
            id: windowMenu
            title: qsTr("&Window")
            Instantiator {
                model: root.actionsForMenu("window")
                delegate: actionMenuItem
                onObjectAdded: (index, object) => windowMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => windowMenu.removeItem(object)
            }
        }
        ThemedMenu {
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
            footer: ThemedDialogFooter {}
            width: 460
            standardButtons: Dialog.Close
            onClosed: {
                recoveryDialog.confirmingDiscardAll = false
                if (!AppSession.hasDocument && !AppSession.ioBusy)
                    welcomeDialog.open()
            }

            readonly property var entries: {
                try {
                    return JSON.parse(AppSession.recoveryEntriesJson || "[]")
                } catch (e) {
                    return []
                }
            }
            /// Second press of "Discard All" is the one that deletes.
            ///
            /// Discarding every snapshot destroys unsaved work permanently, and
            /// a second modal stacked on this one is worse than arming the
            /// button in place — the confirmation stays where the user's
            /// attention already is, and moving the pointer away disarms it.
            property bool confirmingDiscardAll: false

            background: Rectangle {
                color: Theme.surface
                border.color: Theme.border
                radius: Theme.radiusMd
            }
            contentItem: ColumnLayout {
                spacing: Theme.spaceSm
                width: 420
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: recoveryDialog.entries.length === 1
                          ? qsTr("PhotoTux found 1 autosaved document from a previous session.")
                          : qsTr("PhotoTux found %1 autosaved documents from a previous session.")
                            .arg(recoveryDialog.entries.length)
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBody
                }
                // Capped and scrollable. A bare Repeater in this column grew the
                // dialog by one row per snapshot, and a session that had crashed
                // a few times pushed Close off the bottom of the screen — the
                // chooser became unclosable exactly when it was needed most.
                ListView {
                    id: recoveryList
                    Layout.fillWidth: true
                    // A whole number of rows, measured rather than assumed: a
                    // guessed row height leaves the last one sliced in half,
                    // which reads as a broken list rather than a scrollable one.
                    Layout.preferredHeight: {
                        if (count <= 0)
                            return 0
                        var row = contentHeight / count
                        return Math.min(contentHeight, Math.floor(5) * row)
                    }
                    visible: count > 0
                    clip: true
                    spacing: Theme.spaceXs
                    boundsBehavior: Flickable.StopAtBounds
                    // Always on when there is more than fits: five rows out of
                    // twenty-three with no scrollbar reads as "eighteen are
                    // missing", not as "scroll for the rest".
                    ScrollBar.vertical: ThemedScrollBar {
                        policy: recoveryList.contentHeight > recoveryList.height
                                ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
                    }
                    model: recoveryDialog.entries
                    // The band is the delegate's own root rather than a child
                    // anchored inside the row: a Layout manages its children's
                    // geometry, so an anchored sibling in there fights it and
                    // takes a slot of its own — which collapsed every row after
                    // the first.
                    delegate: Rectangle {
                        id: recoveryRow
                        required property var modelData
                        width: recoveryList.width
                        implicitHeight: rowBody.implicitHeight + Theme.spaceXs
                        radius: Theme.radiusSm
                        // A hover band, the way Plasma's list views mark the row
                        // under the pointer. With two actions per row and rows
                        // that differ only by a timestamp, it is what keeps a
                        // click aimed at the row it looks aimed at.
                        color: rowHover.hovered ? Theme.surfaceContainerHigh : "transparent"
                        HoverHandler { id: rowHover }

                        RowLayout {
                            id: rowBody
                            anchors.fill: parent
                            anchors.leftMargin: Theme.spaceXs
                            anchors.rightMargin: Theme.spaceXs
                            spacing: Theme.spaceSm

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Label {
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                    text: recoveryRow.modelData.path
                                          && recoveryRow.modelData.path.length
                                          ? recoveryRow.modelData.path
                                          : qsTr("Untitled (%1)").arg(recoveryRow.modelData.shortId)
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontBodySm
                                }
                                // The time is what actually tells two untitled
                                // snapshots apart; the id is only a tiebreaker.
                                Label {
                                    text: Qt.formatDateTime(
                                              new Date(recoveryRow.modelData.savedMs),
                                              Locale.ShortFormat)
                                    color: Theme.colorOnSurfaceMuted
                                    font.pixelSize: Theme.fontLabelSm
                                }
                            }
                            ThemedButton {
                                text: qsTr("Restore")
                                onClicked: {
                                    AppSession.restoreRecovery(recoveryRow.modelData.id)
                                    recoveryDialog.close()
                                }
                            }
                            ThemedButton {
                                text: qsTr("Discard")
                                flat: true
                                prominence: "danger"
                                Accessible.name: qsTr("Discard this snapshot permanently")
                                onClicked: {
                                    AppSession.discardRecoveryEntry(recoveryRow.modelData.id)
                                    recoveryDialog.confirmingDiscardAll = false
                                }
                            }
                        }
                    }
                }
                Label {
                    Layout.fillWidth: true
                    visible: recoveryDialog.entries.length === 0
                    wrapMode: Text.WordWrap
                    text: qsTr("Nothing left to recover.")
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontBodySm
                }
                RowLayout {
                    Layout.fillWidth: true
                    visible: recoveryDialog.entries.length > 1
                    Item { Layout.fillWidth: true }
                    ThemedButton {
                        flat: true
                        text: recoveryDialog.confirmingDiscardAll
                              ? qsTr("Delete %1 permanently").arg(recoveryDialog.entries.length)
                              : qsTr("Discard All")
                        prominence: "danger"
                        onClicked: {
                            if (recoveryDialog.confirmingDiscardAll) {
                                AppSession.discardAllRecovery()
                                recoveryDialog.confirmingDiscardAll = false
                            } else {
                                recoveryDialog.confirmingDiscardAll = true
                            }
                        }
                        // Wandering off disarms it, so an armed button cannot be
                        // triggered later by a click aimed at something else.
                        onHoveredChanged: if (!hovered) recoveryDialog.confirmingDiscardAll = false
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
                ThemedButton {
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
                Accessible.name: qsTr("New…")
                ThemedToolTip {
                    visible: parent.hovered
                    text: parent.Accessible.name
                }
            }

            ChromeIconToolButton {
                icon.source: root.iconUrl("folder-open")
                enabled: root.actionIsEnabled("action.file.open")
                onClicked: root.runAction("action.file.open")
                Accessible.name: qsTr("Open…")
                ThemedToolTip {
                    visible: parent.hovered
                    text: parent.Accessible.name
                }
            }

            ChromeIconToolButton {
                icon.source: root.iconUrl("export")
                enabled: root.actionIsEnabled("action.file.export")
                onClicked: root.runAction("action.file.export")
                Accessible.name: qsTr("Export PNG, JPEG, or PSD subset")
                ThemedToolTip {
                    visible: parent.hovered
                    text: parent.Accessible.name
                }
            }

            ToolSeparator {
                contentItem: Rectangle { implicitWidth: 1; color: Theme.borderEffective }
            }

            ChromeIconToolButton {
                icon.source: root.iconUrl("arrow-counter-clockwise")
                enabled: root.actionIsEnabled("action.edit.undo")
                onClicked: root.runAction("action.edit.undo")
                Accessible.name: qsTr("Undo")
                ThemedToolTip {
                    visible: parent.hovered
                    text: parent.Accessible.name
                }
            }
            ChromeIconToolButton {
                icon.source: root.iconUrl("arrow-clockwise")
                enabled: root.actionIsEnabled("action.edit.redo")
                onClicked: root.runAction("action.edit.redo")
                Accessible.name: qsTr("Redo")
                ThemedToolTip {
                    visible: parent.hovered
                    text: parent.Accessible.name
                }
            }

            Item { Layout.fillWidth: true }

            ThemedBusyIndicator {
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

            // Saving a large PSD or exporting a 4K composite is the one thing
            // in this editor that takes long enough to regret starting, and
            // the worker has honoured a cancel token the whole time — nothing
            // ever offered the user a way to set it. It only appears while
            // there is something to cancel.
            ChromeIconToolButton {
                visible: AppSession.ioBusy
                implicitWidth: 24
                implicitHeight: 24
                icon.source: root.iconUrl("x")
                icon.width: 14
                icon.height: 14
                onClicked: AppSession.cancelIo()
                Accessible.name: qsTr("Cancel the file operation")
                ThemedToolTip {
                    visible: parent.hovered
                    text: parent.Accessible.name
                }
            }

            ChromeIconToolButton {
                implicitWidth: 28
                implicitHeight: 28
                icon.source: root.iconUrl("question")
                onClicked: aboutDialogLoader.open()
                Accessible.name: qsTr("About PhotoTux")
                ThemedToolTip {
                    visible: parent.hovered
                    text: parent.Accessible.name
                }
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

            /// The accessibility live region.
            ///
            /// `AppSession.lastAnnounce` is the sentence a command wrote about
            /// what it just did — "Grouped layers", "Merged Group — 1 hidden
            /// layer discarded". The property was published and *nothing read
            /// it*, so nineteen command handlers were writing announcements no
            /// screen reader ever heard.
            ///
            /// It draws nothing but is not `visible: false`: Qt drops
            /// invisible items from the accessibility tree, and a live region
            /// nobody can see is the entire point. `Accessible.name` carries
            /// the text so the region is inspectable from AT-SPI;
            /// `announce()` is what raises the event a screen reader speaks.
            Item {
                id: liveRegion
                objectName: "liveRegion"
                implicitWidth: 1
                implicitHeight: 1
                opacity: 0
                Accessible.role: Accessible.StaticText
                Accessible.name: AppSession.lastAnnounce
                Accessible.description: qsTr("Announcements")

                Connections {
                    target: AppSession
                    function onLastAnnounceChanged() {
                        const message = AppSession.lastAnnounce
                        if (message.length > 0)
                            liveRegion.Accessible.announce(message, Accessible.Polite)
                    }
                }
            }

            ThemedButton {
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

            // Zoom is not repeated here. It is the second field of
            // `statusText` on the left, which is where Photoshop puts it, and
            // this cluster is per-frame metrics — the things deliberately kept
            // out of the summary because they would churn its AT-SPI name on
            // every frame.
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
                implicitWidth: gpuBadge.implicitWidth + Theme.spaceSm
                implicitHeight: Theme.controlHeight - 6
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
            readonly property int toolCountBind: root.toolSlots.length
            readonly property var stripParts: {
                var _ = activeToolBind
                var __ = toolCountBind
                return root.toolStripPartitions(stripCapacity)
            }
            readonly property var stripVisible: stripParts.visible || []
            readonly property var stripOverflow: stripParts.overflow || []
            // The overflow menu is flat: a flyout inside a popup is a worse
            // place to look for a tool than a plain list of the ones that did
            // not fit. It should be empty at any usable window size.
            readonly property var stripOverflowTools: {
                var out = []
                for (var i = 0; i < toolStrip.stripOverflow.length; ++i) {
                    var list = toolStrip.stripOverflow[i].tools || []
                    for (var j = 0; j < list.length; ++j)
                        out.push(list[j])
                }
                return out
            }

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

                // One button per slot. A slot holding several tools shows the
                // one in use (or last used), marks itself with a corner wedge,
                // and opens a flyout on right-click or press-and-hold — which
                // is how Photoshop has stacked a tool shelf for thirty years,
                // and what lets twenty-five tools live in eighteen buttons.
                Repeater {
                    model: toolStrip.stripVisible
                    delegate: Item {
                        id: slotItem
                        required property var modelData
                        required property int index

                        width: toolColumn.width
                        height: 40
                        readonly property var slotTools: slotItem.modelData.tools || []
                        readonly property var face: root.slotFace(slotItem.modelData)
                        readonly property string toolId: slotItem.face.id
                        readonly property bool stacked: slotItem.slotTools.length > 1
                        readonly property bool holdsActive: root.slotHoldsActive(slotItem.modelData)
                        readonly property string toolGroup: slotItem.modelData.group || ""
                        readonly property string prevGroup: slotItem.index > 0
                                                           ? toolStrip.stripVisible[slotItem.index - 1].group
                                                           : ""

                        Rectangle {
                            visible: slotItem.index > 0 && slotItem.toolGroup !== slotItem.prevGroup
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
                            color: slotItem.holdsActive
                                   ? Theme.toolActiveBg
                                   : (toolHover.hovered ? Theme.surfaceContainerHigh : "transparent")

                            Rectangle {
                                visible: slotItem.holdsActive
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.bottom: parent.bottom
                                width: 2
                                color: Theme.primary
                            }

                            ThemedIcon {
                                anchors.centerIn: parent
                                source: root.iconUrl(root.toolIconStem(slotItem.face.icon))
                                size: 20
                                color: Theme.iconOnSurfaceEffective
                            }

                            // The wedge says "there is more here". Without it a
                            // stacked slot is indistinguishable from a plain
                            // one, and the tools behind it are undiscoverable.
                            Canvas {
                                visible: slotItem.stacked
                                width: 6
                                height: 6
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                anchors.margins: 2
                                onPaint: {
                                    var ctx = getContext("2d")
                                    ctx.reset()
                                    ctx.fillStyle = Theme.colorOnSurfaceMuted
                                    ctx.beginPath()
                                    ctx.moveTo(width, 0)
                                    ctx.lineTo(width, height)
                                    ctx.lineTo(0, height)
                                    ctx.closePath()
                                    ctx.fill()
                                }
                            }

                            HoverHandler { id: toolHover }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                pressAndHoldInterval: 400
                                Accessible.role: Accessible.Button
                                Accessible.name: slotItem.face.title
                                Accessible.description: slotItem.stacked
                                    ? qsTr("%1 — hold or right-click for %2 more")
                                      .arg(slotItem.toolId)
                                      .arg(slotItem.slotTools.length - 1)
                                    : slotItem.toolId
                                Accessible.checkable: true
                                Accessible.checked: slotItem.holdsActive
                                onClicked: (mouse) => {
                                    if (mouse.button === Qt.RightButton) {
                                        if (slotItem.stacked)
                                            slotItem.openFlyout()
                                    } else {
                                        root.activateToolFromSlot(slotItem.modelData.slot,
                                                                  slotItem.toolId)
                                    }
                                }
                                onPressAndHold: if (slotItem.stacked) slotItem.openFlyout()
                                ThemedToolTip {
                                    visible: parent.containsMouse && !slotFlyout.visible
                                    // Shorter than the default: the shelf is
                                    // the one place a pointer rests on a
                                    // control while deciding, and 450 ms there
                                    // reads as the tip not coming.
                                    delay: 400
                                    text: slotItem.stacked
                                          ? qsTr("%1  (hold for %2 more)")
                                            .arg(slotItem.face.title)
                                            .arg(slotItem.slotTools.length - 1)
                                          : slotItem.face.title
                                }
                                hoverEnabled: true
                            }
                        }

                        // Same shape as the overflow popup a few lines down:
                        // parented to the overlay so the 48px-wide shelf does
                        // not clip it, and opened via callLater so the press
                        // that opened it is not then read as a press-outside.
                        function openFlyout() {
                            var origin = slotItem.mapToItem(Overlay.overlay, slotItem.width + 2, 0)
                            slotFlyout.x = origin.x
                            slotFlyout.y = origin.y
                            Qt.callLater(slotFlyout.open)
                        }

                        Popup {
                            id: slotFlyout
                            parent: Overlay.overlay
                            padding: Theme.spaceXxs
                            modal: false
                            focus: true
                            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
                            background: Rectangle {
                                color: Theme.surfaceOverlay
                                border.color: Theme.border
                                radius: Theme.radiusSm
                            }
                            // Explicit width. Sizing the popup from its rows
                            // meant each row's implicitWidth read ids living
                            // inside its own contentItem, which resolves to
                            // nothing during construction — so the popup came
                            // out zero wide and opened invisibly.
                            width: 208
                            contentItem: ColumnLayout {
                                spacing: 0
                                Repeater {
                                    model: slotItem.slotTools
                                    delegate: ItemDelegate {
                                        id: flyoutRow
                                        required property var modelData
                                        Layout.fillWidth: true
                                        implicitHeight: Theme.controlHeight
                                        Accessible.name: flyoutRow.modelData.title
                                        onClicked: {
                                            root.activateToolFromSlot(slotItem.modelData.slot,
                                                                      flyoutRow.modelData.id)
                                            slotFlyout.close()
                                        }
                                        background: Rectangle {
                                            radius: Theme.radiusXs
                                            color: flyoutRow.hovered
                                                   ? Theme.surfaceContainerHigh : "transparent"
                                        }
                                        contentItem: RowLayout {
                                            spacing: Theme.spaceSm
                                            ThemedIcon {
                                                source: root.iconUrl(
                                                    root.toolIconStem(flyoutRow.modelData.icon))
                                                size: Theme.iconMd
                                                color: Theme.iconOnSurfaceEffective
                                            }
                                            Label {
                                                text: flyoutRow.modelData.title
                                                color: AppSession.activeTool === flyoutRow.modelData.id
                                                       ? Theme.primary : Theme.colorOnSurface
                                                font.pixelSize: Theme.fontBodySm
                                                Layout.fillWidth: true
                                            }
                                            Label {
                                                text: flyoutRow.modelData.shortcut
                                                color: Theme.colorOnSurfaceMuted
                                                font.pixelSize: Theme.fontLabelSm
                                                font.family: "Noto Sans Mono"
                                            }
                                        }
                                    }
                                }
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
                ThemedToolTip {
                    visible: parent.hovered
                    text: qsTr("More tools")
                }
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
                        model: toolStrip.stripOverflowTools
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
                                    source: root.iconUrl(root.toolIconStem(modelData.icon))
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

            // Toasts live over the canvas, not in the chrome: they are the one
            // surface that must not be missed, and the canvas is where the eye
            // already is. Last child of the host so they draw above everything
            // in it, and `z` so a later sibling cannot bury them.
            NoticeToasts {
                anchors.fill: parent
                z: 100
                iconUrl: root.iconUrl
            }

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
                // Accent at 12%, alpha first: an eight-digit hex is
                // `#AARRGGBB` to Qt, so this had been a pale green fill inside
                // a cyan border.
                color: "#1F3DAEE9"
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
            CanvasInput {
                id: canvasInput
                anchors.fill: parent

                screenToDocX: root.screenToDocX
                screenToDocY: root.screenToDocY
                isDabTool: root.isDabTool
                isSelectTool: root.isSelectTool
                isLassoTool: root.isLassoTool
                isPolygonTool: root.isPolygonTool
                isCropTool: root.isCropTool
                isTransformTool: root.isTransformTool
                selectionCombineFromModifiers: root.selectionCombineFromModifiers
                appendPathPoint: root.appendPathPoint
                afterHostSlot: root.afterHostSlot

                onSelectionContextMenuRequested: function (localX, localY) {
                    root.openContextMenu(selectionContextMenu, canvasInput, localX, localY)
                }
                onCanvasContextMenuRequested: function (localX, localY) {
                    root.openContextMenu(canvasContextMenu, canvasInput, localX, localY)
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
                    ChromeIconToolButton {
                        required property string modelData
                        implicitWidth: Theme.panelHeaderBtn
                        implicitHeight: Theme.panelHeaderBtn
                        icon.source: root.iconUrl(root.panelIconStem(modelData))
                        icon.width: Theme.iconMd
                        icon.height: Theme.iconMd
                        Accessible.name: qsTr("Show %1").arg(qsTr(root.panelTitle(modelData)))
                        ThemedToolTip {
                            visible: parent.hovered
                            text: parent.Accessible.name
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
                    // The seam between this group and the one above it. The dock
                    // had no such affordance at all — every panel's height was a
                    // constant in the shell, so Properties was permanently capped
                    // at a fraction of the dock and its longer groups could only
                    // be scrolled. Hidden on the topmost group, and on a neighbour
                    // that sizes to its own content and so has no height to drag.
                    PanelResizeGrip {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.topMargin: -2
                        readonly property string above: root.panelAboveInStack("panel.properties")
                        readonly property real dockHeight: rightDock.height
                        visible: above.length > 0 && root.panelIsResizable(above, dockHeight)
                        panelId: above
                        currentHeight: root.panelHeightRevision >= 0
                                       && root.resolvedPanelHeights[above] !== undefined
                                       ? root.resolvedPanelHeights[above]
                                       : root.panelDefaultHeight(above, dockHeight)
                        onPreviewed: (height) => root.setPanelHeightDraft(above, height)
                        onCommitted: (height) => root.commitPanelHeight(above, height)
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
                            panelId: "panel.properties"
                            tabs: root.dockGroupVisibleTabs("panel.properties")
                            Layout.fillWidth: true
                        }
                        PanelHeaderControls {
                            panelId: "panel.properties"
                            stackRow: root.dockStackRow("panel.properties")
                            stackLength: root.dockRightStack.length
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
                    id: propertiesFlick
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
                        // The automatic cap is only the *default* now: it keeps
                        // Layers and History above the status bar in a dock the
                        // user has never touched. A dragged height overrides it.
                        var _rev = root.panelHeightRevision
                        var chosen = root.resolvedPanelHeights["panel.properties"]
                        return chosen !== undefined
                               ? chosen
                               : root.panelDefaultHeight("panel.properties", h)
                    }
                    // Pinned to the preferred height, or the layout squeezes it
                    // straight back: once a panel is dragged taller the stack's
                    // preferred sizes add up to more than the dock, and a
                    // GridLayout resolves that by compressing whatever has no
                    // minimum. The Layers body fills, so it is what yields.
                    Layout.minimumHeight: Layout.preferredHeight
                    contentHeight: propsCol.implicitHeight
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    // Pinned on whenever there is more than fits. `AsNeeded`
                    // shows the bar only while flicking, so a panel that had
                    // clipped a section's heading mid-word looked broken
                    // rather than scrollable — the one thing a dense dock has
                    // to say without being touched.
                    ScrollBar.vertical: ThemedScrollBar {
                        policy: propertiesFlick.contentHeight > propertiesFlick.height
                                ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
                    }

                    // A fade at the cut, for the same reason: a heading sliced
                    // in half by a hard edge reads as a rendering fault, and
                    // the same heading fading out reads as "there is more".
                    Rectangle {
                        parent: propertiesFlick
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: Theme.spaceLg
                        z: 10
                        visible: propertiesFlick.contentHeight - propertiesFlick.contentY
                                 > propertiesFlick.height + 1
                        gradient: Gradient {
                            GradientStop { position: 0.0; color: "transparent" }
                            GradientStop { position: 1.0; color: Theme.surface }
                        }
                    }

                    PropertiesPanel {
                        id: propsCol
                        width: parent.width
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.margins: Theme.spaceMd
                        // The scroll bar is an overlay, so it lands on top of
                        // whatever is at the right edge — it was clipping the
                        // border of every full-width button. Reserved whether
                        // or not the bar is showing: making the margin depend
                        // on its visibility would feed the content width back
                        // into the height that decides that visibility.
                        anchors.rightMargin: Theme.spaceMd + Theme.spaceSm
                        spacing: Theme.spaceMd

                        iconUrl: root.iconUrl
                        runAction: root.runAction
                        isTransformTool: root.isTransformTool
                        isCropTool: root.isCropTool
                        activeLayerHasMask: root.activeLayerHasMask
                        activeMaskEnabled: root.activeMaskEnabled
                        gpuStatus: gpuCanvas.gpuStatus
                        onEmbedIccRequested: root.browseForFile(embedIccFileDialog)
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
                    PanelResizeGrip {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.topMargin: -2
                        readonly property string above: root.panelAboveInStack("panel.navigator")
                        readonly property real dockHeight: rightDock.height
                        visible: above.length > 0 && root.panelIsResizable(above, dockHeight)
                        panelId: above
                        currentHeight: root.panelHeightRevision >= 0
                                       && root.resolvedPanelHeights[above] !== undefined
                                       ? root.resolvedPanelHeights[above]
                                       : root.panelDefaultHeight(above, dockHeight)
                        onPreviewed: (height) => root.setPanelHeightDraft(above, height)
                        onCommitted: (height) => root.commitPanelHeight(above, height)
                    }
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
                            panelId: "panel.navigator"
                            stackRow: root.dockStackRow("panel.navigator")
                            stackLength: root.dockRightStack.length
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
                    Layout.preferredHeight: visible && root.panelHeightRevision >= 0
                                            ? (root.resolvedPanelHeights["panel.navigator"]
                                               !== undefined
                                               ? root.resolvedPanelHeights["panel.navigator"]
                                               : root.panelDefaultHeight("panel.navigator",
                                                                         parent.height))
                                            : 0
                    Layout.minimumHeight: Layout.preferredHeight
                    color: Theme.surfaceSunken
                    clip: true

                    readonly property real pad: Theme.spaceSm
                    readonly property real docW: Math.max(1, AppSession.docWidth)
                    readonly property real docH: Math.max(1, AppSession.docHeight)
                    readonly property real availW: width - pad * 2
                    readonly property real availH: height - pad * 2
                    // `fitScale`, not `scale`: every Item already has a `scale`,
                    // the render transform. A property of that name shadows it,
                    // so a reader cannot tell which one a binding meant and a
                    // stray `scale: …` elsewhere in the pane would resize the
                    // whole navigator instead of setting this ratio.
                    readonly property real fitScale: Math.min(availW / docW, availH / docH)
                    readonly property real frameW: docW * fitScale
                    readonly property real frameH: docH * fitScale
                    readonly property real frameX: (width - frameW) / 2
                    readonly property real frameY: (height - frameH) / 2
                    readonly property real viewW: Math.max(8, AppSession.viewportWidth / Math.max(0.001, AppSession.zoom) * fitScale)
                    readonly property real viewH: Math.max(8, AppSession.viewportHeight / Math.max(0.001, AppSession.zoom) * fitScale)
                    readonly property real viewX: frameX + (AppSession.panX - viewW / (2 * fitScale)) * fitScale
                    readonly property real viewY: frameY + (AppSession.panY - viewH / (2 * fitScale)) * fitScale

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

                        // The document itself. Without it the Navigator drew a
                        // flat rectangle, so it told the user where they were
                        // relative to nothing — which is the one question the
                        // panel exists to answer. The host rebuilds this on a
                        // throttle; the fill above shows through until the
                        // first one arrives.
                        Image {
                            anchors.fill: parent
                            anchors.margins: 1
                            source: AppSession.navigatorThumbnail
                            // Tested on the string, not on `source`: assigning
                            // to a `url` property normalises the value, so
                            // comparing the result against "" says nothing
                            // useful about whether the host has published one.
                            visible: AppSession.navigatorThumbnail.length > 0
                            fillMode: Image.Stretch
                            // Already downsampled to panel size by the host, so
                            // smoothing here would only soften it again.
                            smooth: false
                            asynchronous: true
                            cache: false
                        }
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
                    PanelResizeGrip {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.topMargin: -2
                        readonly property string above: root.panelAboveInStack("panel.swatches")
                        readonly property real dockHeight: rightDock.height
                        visible: above.length > 0 && root.panelIsResizable(above, dockHeight)
                        panelId: above
                        currentHeight: root.panelHeightRevision >= 0
                                       && root.resolvedPanelHeights[above] !== undefined
                                       ? root.resolvedPanelHeights[above]
                                       : root.panelDefaultHeight(above, dockHeight)
                        onPreviewed: (height) => root.setPanelHeightDraft(above, height)
                        onCommitted: (height) => root.commitPanelHeight(above, height)
                    }
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
                            panelId: "panel.swatches"
                            stackRow: root.dockStackRow("panel.swatches")
                            stackLength: root.dockRightStack.length
                            onTearOffRequested: root.tearOffAndClamp("panel.swatches",
                                                                       root.x + root.width - 360, root.y + 160, 320, 280)
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

                        /// Which of the pair the hex field and the palette edit.
                        ///
                        /// The background swatch used to *swap* on click, which
                        /// left no gesture for setting it: you swapped, edited
                        /// what had become the foreground, and swapped back.
                        /// `setBackgroundHex` existed the whole time with
                        /// nothing calling it. Photoshop opens the picker for
                        /// whichever swatch you click, and keeps swap as its own
                        /// control — which this panel's header already has.
                        property bool editingBackground: false

                        /// The hex the field shows, written in one place.
                        ///
                        /// This was a conditional binding —
                        /// `editingBackground ? backgroundHex : foregroundHex`
                        /// — and it went stale: after a commit, a palette click
                        /// and a reset in sequence, the field showed the
                        /// foreground while the background swatch was the one
                        /// selected. A conditional binding only tracks the
                        /// branch it evaluated, and `TextField` drops its `text`
                        /// binding the moment the user types, so the two
                        /// mechanisms were repairing each other in an order
                        /// nothing states. One writer and three explicit
                        /// triggers is smaller than the rule you would have to
                        /// remember otherwise.
                        property string editedHex: AppSession.foregroundHex
                        function refreshHex() {
                            editedHex = editingBackground ? AppSession.backgroundHex
                                                          : AppSession.foregroundHex
                        }
                        onEditingBackgroundChanged: refreshHex()
                        Connections {
                            target: AppSession
                            // Reads only. Calling a slot from a handler that
                            // reacts to an AppSession signal re-enters a
                            // borrowed session — see qml/AGENTS.md.
                            function onForegroundHexChanged() { swatchesCol.refreshHex() }
                            function onBackgroundHexChanged() { swatchesCol.refreshHex() }
                        }
                        function applyHex(hex) {
                            if (editingBackground)
                                AppSession.setBackgroundHex(hex)
                            else
                                AppSession.setForegroundHex(hex)
                            // The host may have refused it, and a refusal emits
                            // no change signal — so put the field back from the
                            // colour that actually survived.
                            refreshHex()
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceMd

                            // Photoshop's colour widget: two overlapping
                            // squares, a swap arrow at the top right, and the
                            // black-and-white default mark at the bottom left.
                            // Both controls used to live in the panel header,
                            // which is not where anyone coming from Photoshop
                            // looks for them — and which had run out of room.
                            Item {
                                implicitWidth: 58
                                implicitHeight: 50
                                Rectangle {
                                    x: 14
                                    y: 18
                                    width: 26
                                    height: 26
                                    radius: Theme.radiusSm
                                    color: AppSession.backgroundHex
                                    border.color: swatchesCol.editingBackground
                                                  ? Theme.primary : Theme.border
                                    border.width: swatchesCol.editingBackground ? 2 : 1
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: {
                                            swatchesCol.editingBackground = true
                                            Qt.callLater(function () {
                                                hexField.forceActiveFocus()
                                                AppSession.setShortcutInputYield(true)
                                            })
                                        }
                                        ThemedToolTip {
                                            visible: parent.containsMouse
                                            text: qsTr("Background")
                                        }
                                        hoverEnabled: true
                                    }
                                }
                                Rectangle {
                                    x: 2
                                    y: 4
                                    width: 26
                                    height: 26
                                    radius: Theme.radiusSm
                                    color: AppSession.foregroundHex
                                    border.color: swatchesCol.editingBackground
                                                  ? Theme.border : Theme.primary
                                    border.width: swatchesCol.editingBackground ? 1 : 2
                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: {
                                            swatchesCol.editingBackground = false
                                            Qt.callLater(function () {
                                                hexField.forceActiveFocus()
                                                AppSession.setShortcutInputYield(true)
                                            })
                                        }
                                        ThemedToolTip {
                                            visible: parent.containsMouse
                                            text: qsTr("Foreground")
                                        }
                                        hoverEnabled: true
                                    }
                                }

                                // Top right, over the corner of the background
                                // square, exactly where Photoshop keeps it.
                                ChromeIconToolButton {
                                    x: 42
                                    y: 0
                                    implicitWidth: 16
                                    implicitHeight: 16
                                    padding: 0
                                    icon.source: root.iconUrl("arrows-left-right")
                                    icon.width: 12
                                    icon.height: 12
                                    onClicked: AppSession.swapFgBg()
                                    Accessible.name: qsTr("Swap foreground and background")
                                    ThemedToolTip {
                                        visible: parent.hovered
                                        text: parent.Accessible.name
                                    }
                                }

                                // Bottom left. `ColorState::reset_default` has
                                // been in the engine the whole time with nothing
                                // reaching it. Not `arrow-counter-clockwise`:
                                // that is Undo's icon, and a second meaning for
                                // it in the same window is worse than no icon.
                                ChromeIconToolButton {
                                    x: 0
                                    y: 34
                                    implicitWidth: 16
                                    implicitHeight: 16
                                    padding: 0
                                    icon.source: root.iconUrl("square-half")
                                    icon.width: 12
                                    icon.height: 12
                                    onClicked: AppSession.resetFgBg()
                                    Accessible.name: qsTr("Reset to black and white")
                                    ThemedToolTip {
                                        visible: parent.hovered
                                        text: parent.Accessible.name
                                    }
                                }
                            }

                            ThemedTextField {
                                id: hexField
                                Layout.fillWidth: true
                                // `source`, not `text`: a `TextField` drops its
                                // `text` binding the moment the user types.
                                source: swatchesCol.editedHex
                                Accessible.name: swatchesCol.editingBackground
                                                 ? qsTr("Background hex")
                                                 : qsTr("Foreground hex")
                                font.family: "Noto Sans Mono"
                                font.pixelSize: Theme.fontMono
                                onActiveFocusChanged: root.refreshShortcutYield()
                                // A `TextField` drops its `text` binding the
                                // moment the user types, so without restoring
                                // it the field keeps showing what was typed —
                                // `notacolour` stayed on screen for good while
                                // the swatch beside it never moved.
                                // `applyHex` ends by writing the field, so a
                                // value the host refused never survives on
                                // screen.
                                onEditingFinished: swatchesCol.applyHex(text)
                                Keys.onReturnPressed: {
                                    swatchesCol.applyHex(text)
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
                                        onClicked: swatchesCol.applyHex(modelData)
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
                                        // The hex, not `pickRecentColor(index)`:
                                        // that one always sets the foreground,
                                        // so clicking a recent colour while the
                                        // background swatch was selected changed
                                        // the wrong half of the pair.
                                        onClicked: swatchesCol.applyHex(modelData)
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
                    PanelResizeGrip {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.topMargin: -2
                        readonly property string above: root.panelAboveInStack("panel.layers")
                        readonly property real dockHeight: rightDock.height
                        visible: above.length > 0 && root.panelIsResizable(above, dockHeight)
                        panelId: above
                        currentHeight: root.panelHeightRevision >= 0
                                       && root.resolvedPanelHeights[above] !== undefined
                                       ? root.resolvedPanelHeights[above]
                                       : root.panelDefaultHeight(above, dockHeight)
                        onPreviewed: (height) => root.setPanelHeightDraft(above, height)
                        onCommitted: (height) => root.commitPanelHeight(above, height)
                    }
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
                            panelId: "panel.layers"
                            stackRow: root.dockStackRow("panel.layers")
                            stackLength: root.dockRightStack.length
                            onTearOffRequested: root.tearOffAndClamp("panel.layers",
                                                                       root.x + root.width - 360, root.y + 200, 320, 360)
                        }
                        ChromeIconToolButton {
                            implicitWidth: Theme.panelHeaderBtn
                            implicitHeight: Theme.panelHeaderBtn
                            icon.source: root.iconUrl("plus")
                            icon.width: Theme.iconMd
                            icon.height: Theme.iconMd
                            enabled: AppSession.hasDocument
                            onClicked: AppSession.addLayer()
                            ThemedToolTip {
                                visible: parent.hovered
                                text: qsTr("Add layer")
                            }
                            Accessible.name: qsTr("Add layer")
                        }
                        ChromeIconToolButton {
                            implicitWidth: Theme.panelHeaderBtn
                            implicitHeight: Theme.panelHeaderBtn
                            icon.source: root.iconUrl("folder")
                            icon.width: Theme.iconMd
                            icon.height: Theme.iconMd
                            enabled: AppSession.hasDocument
                            onClicked: AppSession.addGroupLayer()
                            ThemedToolTip {
                                visible: parent.hovered
                                text: qsTr("Add group")
                            }
                            Accessible.name: qsTr("Add group")
                        }
                        ChromeIconToolButton {
                            implicitWidth: Theme.panelHeaderBtn
                            implicitHeight: Theme.panelHeaderBtn
                            icon.source: root.iconUrl("trash")
                            icon.width: Theme.iconMd
                            icon.height: Theme.iconMd
                            enabled: AppSession.hasDocument && AppSession.layerCount > 1
                            onClicked: AppSession.deleteActiveLayer()
                            ThemedToolTip {
                                visible: parent.hovered
                                text: qsTr("Delete layer")
                            }
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

                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0

                        // Blend, opacity and locks sit above the list, where
                        // Photoshop keeps them. They used to be in Properties,
                        // three panels away from the layer they act on.
                        LayerControlStrip {
                            id: layerControls
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            // Controls for a layer that is not there read as
                            // broken chrome rather than as an empty document.
                            visible: AppSession.hasDocument
                            blendModes: root.blendModes
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: visible ? 1 : 0
                            visible: AppSession.hasDocument
                            color: Theme.border
                        }
                        Item {
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            // A document always has at least one layer, so an
                            // empty list means there is no document — not that
                            // the layers went missing.
                            PanelPlaceholder {
                                anchors.fill: parent
                                visible: !AppSession.hasDocument
                                iconKey: "stack-simple"
                                iconUrl: root.iconUrl
                                text: qsTr("No document open")
                                hint: qsTr("Open or create one to see its layers.")
                            }

                            LayersPanel {
                                anchors.fill: parent
                                visible: AppSession.hasDocument
                                iconUrl: root.iconUrl
                                maskEditActive: root.maskEditActive
                                onContextMenuRequested: function (stackIndex, origin, localX, localY) {
                                    layerContextMenu.targetIndex = stackIndex
                                    root.openContextMenu(layerContextMenu, origin, localX, localY)
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
                    PanelResizeGrip {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.topMargin: -2
                        readonly property string above: root.panelAboveInStack("panel.history")
                        readonly property real dockHeight: rightDock.height
                        visible: above.length > 0 && root.panelIsResizable(above, dockHeight)
                        panelId: above
                        currentHeight: root.panelHeightRevision >= 0
                                       && root.resolvedPanelHeights[above] !== undefined
                                       ? root.resolvedPanelHeights[above]
                                       : root.panelDefaultHeight(above, dockHeight)
                        onPreviewed: (height) => root.setPanelHeightDraft(above, height)
                        onCommitted: (height) => root.commitPanelHeight(above, height)
                    }
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
                            panelId: "panel.history"
                            stackRow: root.dockStackRow("panel.history")
                            stackLength: root.dockRightStack.length
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
                    Layout.preferredHeight: visible && root.panelHeightRevision >= 0
                                            ? (root.resolvedPanelHeights["panel.history"]
                                               !== undefined
                                               ? root.resolvedPanelHeights["panel.history"]
                                               : root.panelDefaultHeight("panel.history",
                                                                         parent.height))
                                            : 0
                    Layout.minimumHeight: Layout.preferredHeight
                    color: Theme.surfaceSunken

                    // Two empty states, because the guidance differs: with no
                    // document there is nothing to have a history of, and with
                    // one open the list simply has not been written to yet.
                    PanelPlaceholder {
                        anchors.fill: parent
                        visible: historyList.count === 0
                        iconKey: "clock-counter-clockwise"
                        iconUrl: root.iconUrl
                        text: AppSession.hasDocument
                              ? qsTr("No history yet")
                              : qsTr("No document open")
                        hint: AppSession.hasDocument
                              ? qsTr("Edits you make will be listed here, newest last.")
                              : qsTr("Open or create one to start a history.")
                    }

                    ListView {
                        id: historyList
                        anchors.fill: parent
                        clip: true
                        reuseItems: true
                        cacheBuffer: 88
                        model: AppSession.historyModel
                        delegate: Item {
                            id: historyRow
                            // Roles from the model item's field names, which
                            // the derive leaves in snake_case.
                            required property string label
                            required property string kind
                            required property int entry_id
                            required property bool undone

                            width: historyList.width
                            height: 22

                            // An undone step is still on the timeline, and
                            // clicking it walks forward to it. Dimmed rather
                            // than hidden, the way Photoshop greys the steps
                            // ahead of where you are.
                            opacity: historyRow.undone ? 0.45 : 1.0

                            Rectangle {
                                anchors.fill: parent
                                color: historyHover.hovered
                                       ? Theme.surfaceContainerHigh : "transparent"
                            }

                            Label {
                                anchors.left: parent.left
                                anchors.right: historyKind.left
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceSm
                                text: historyRow.label
                                color: Theme.colorOnSurfaceVariant
                                font.pixelSize: Theme.fontBodySm
                                elide: Text.ElideRight
                            }
                            // The kind is taxonomy, not a name: inline it read
                            // "Brush stroke · stroke". Set to one side, in the
                            // same muted trailing column the command palette
                            // uses for an action's menu.
                            Label {
                                id: historyKind
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.rightMargin: Theme.spaceSm
                                text: historyRow.kind
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                            }

                            Accessible.role: Accessible.ListItem
                            Accessible.name: historyRow.undone
                                             ? qsTr("%1 (undone)").arg(historyRow.label)
                                             : historyRow.label

                            HoverHandler { id: historyHover }
                            TapHandler {
                                onTapped: AppSession.jumpHistoryEntry(historyRow.entry_id)
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
        fileMode: FileDialog.OpenFile
        nameFilters: [
            qsTr("All supported (*.ptx *.png *.jpg *.jpeg *.webp *.tif *.tiff *.bmp *.gif *.psd)"),
            qsTr("PhotoTux documents (*.ptx)"),
            qsTr("Image files (*.png *.jpg *.jpeg *.webp *.tif *.tiff *.bmp *.gif)"),
            qsTr("Photoshop (*.psd)")
        ]
        onAccepted: {
            root.lastBrowsedFolder = currentFolder
            AppSession.openRasterFile(selectedFile.toString())
        }
        onRejected: {
            if (!AppSession.hasDocument && !AppSession.ioBusy)
                welcomeDialog.open()
        }
    }

    FileDialog {
        id: embedIccFileDialog
        title: qsTr("Embed ICC Profile")
        fileMode: FileDialog.OpenFile
        nameFilters: [
            qsTr("ICC profiles (*.icc *.icm)"),
            qsTr("All files (*)")
        ]
        onAccepted: {
            root.lastBrowsedFolder = currentFolder
            AppSession.embedIccFromFile(selectedFile.toString())
        }
    }

    FileDialog {
        id: saveFileDialog
        title: qsTr("Save PhotoTux Document")
        fileMode: FileDialog.SaveFile
        nameFilters: [ qsTr("PhotoTux documents (*.ptx)") ]
        defaultSuffix: "ptx"
        onAccepted: {
            root.lastBrowsedFolder = currentFolder
            AppSession.saveDocument(selectedFile.toString())
        }
    }

    FileDialog {
        id: exportFileDialog
        title: qsTr("Export Image")
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
        onAccepted: {
            root.lastBrowsedFolder = currentFolder
            AppSession.exportRasterFile(selectedFile.toString())
        }
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

            // Written out rather than `standardButtons`, which builds its
            // buttons from the Controls style and so cannot be told that
            // saving is the commit and discarding is the destructive one.
            footer: ThemedDialogFooter {
                ThemedButton {
                    text: qsTr("Save")
                    prominence: "primary"
                    DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
                    onClicked: {
                        unsavedDialog.close()
                        if (AppSession.documentPath && AppSession.documentPath.length > 0)
                            AppSession.saveDocument("")
                        else
                            root.browseForFile(saveFileDialog)
                    }
                }
                ThemedButton {
                    text: qsTr("Discard")
                    prominence: "danger"
                    flat: true
                    DialogButtonBox.buttonRole: DialogButtonBox.DestructiveRole
                    onClicked: {
                        unsavedDialog.close()
                        root.discardAndContinue()
                    }
                }
                ThemedButton {
                    text: qsTr("Cancel")
                    DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
                    onClicked: {
                        root.pendingDestructiveAction = ""
                        unsavedDialog.close()
                    }
                }
            }
        }
    }

    // Select > Modify asks how far, because its entries are spelled with an
    // ellipsis and because the radius *is* the operation — a feather is
    // nothing but its radius.
    LazyDialog {
        id: imageSizeLoader
        requested: AppSession.imageSizeOpen

        ImageSizeDialog {
            afterHostSlot: root.afterHostSlot
        }
    }

    LazyDialog {
        id: canvasSizeLoader
        requested: AppSession.canvasSizeOpen

        CanvasSizeDialog {
            afterHostSlot: root.afterHostSlot
        }
    }

    LazyDialog {
        id: selectionModifyLoader
        requested: AppSession.selectionModifyOp.length > 0

        SelectionModifyDialog {
            afterHostSlot: root.afterHostSlot
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
            footer: ThemedDialogFooter {}
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
            footer: ThemedDialogFooter {}
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
                ScrollBar.vertical: ThemedScrollBar {}
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
            footer: ThemedDialogFooter {}
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
        onRequestOpen: root.browseForFile(openFileDialog)
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
            root.syncLayerOpacity()
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
            root.syncLayerOpacity()
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
