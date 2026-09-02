import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Fuzzy command palette over the action registry.
///
/// Extracted from `Main.qml`. Everything it needs from the shell is a declared
/// property rather than a reach into `root`, so the seam is visible and the
/// component is movable.

Popup {
    id: dialog

    /// Action registry rows, as `Main` projects them.
    required property var actionDescriptors

    /// Predicate: is this action id currently invocable.
    required property var actionIsEnabled

    /// Recomputes whether single-key shortcuts yield.
    required property var refreshShortcutYield

    /// Invokes an action id through the host.
    required property var runAction

    /// Chord currently bound to an action id.
    required property var shortcutForAction

    /// Shell window width, for sizing against the viewport.
    required property int hostWidth

    /// Shell window height, for sizing against the viewport.
    required property int hostHeight
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(520, dialog.hostWidth - 48)
    height: Math.min(420, dialog.hostHeight - 48)
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    padding: Theme.spaceMd

    property int selectedIndex: 0
    property string query: ""

    /// Subsequence fuzzy score; higher is better. -1 = no match.
    function fuzzyScore(hay, needle) {
        if (!needle)
            return 0
        if (!hay)
            return -1
        var hi = 0
        var score = 0
        var streak = 0
        for (var ni = 0; ni < needle.length; ++ni) {
            var ch = needle.charAt(ni)
            var found = -1
            for (var j = hi; j < hay.length; ++j) {
                if (hay.charAt(j) === ch) {
                    found = j
                    break
                }
            }
            if (found < 0)
                return -1
            if (found === hi)
                streak++
            else
                streak = 0
            score += 10 + streak * 5 - (found - hi)
            hi = found + 1
        }
        // Prefer shorter labels and early matches.
        score += Math.max(0, 40 - hay.length)
        return score
    }

    function filteredActions() {
        var q = dialog.query.trim().toLowerCase()
        var out = []
        var scored = []
        var all = dialog.actionDescriptors
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
            var best = -1
            var sLabel = dialog.fuzzyScore(label, q)
            var sId = dialog.fuzzyScore(id, q)
            var sMenu = dialog.fuzzyScore(menu, q)
            if (sLabel > best)
                best = sLabel
            if (sId > best)
                best = sId
            if (sMenu > best)
                best = sMenu
            // Substring boost for exact contiguous hits.
            if (label.indexOf(q) >= 0)
                best = Math.max(best, 1000 - label.indexOf(q))
            if (id.indexOf(q) >= 0)
                best = Math.max(best, 900 - id.indexOf(q))
            if (best >= 0)
                scored.push({ action: a, score: best })
        }
        if (!q)
            return out
        scored.sort(function (x, y) {
            return y.score - x.score
        })
        for (var k = 0; k < scored.length; ++k)
            out.push(scored[k].action)
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
        dialog.refreshShortcutYield()
    }

    function runSelected() {
        var list = dialog.filteredActions()
        if (list.length === 0)
            return
        var idx = Math.max(0, Math.min(dialog.selectedIndex, list.length - 1))
        var action = list[idx]
        if (!dialog.actionIsEnabled(action.id))
            return
        dialog.closePalette()
        dialog.runAction(action.id)
    }

    // refreshShortcutYield already computes `false` once the palette
    // is gone, and unlike a direct call it cannot land inside a host
    // slot that closed the palette by changing host state.
    onClosed: dialog.refreshShortcutYield()

    background: Rectangle {
        color: Theme.surface
        border.color: Theme.border
        radius: Theme.radiusMd
    }

    // Unanchored so the Popup's own `padding` above actually applies;
    // filling `parent` pushed the content flush against the border.
    contentItem: ColumnLayout {
        spacing: Theme.spaceSm

        Label {
            text: qsTr("Command palette")
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontLabel
            font.weight: Font.DemiBold
        }

        ThemedTextField {
            id: paletteField
            Layout.fillWidth: true
            placeholderText: qsTr("Filter commands…")
            text: dialog.query
            onTextChanged: {
                dialog.query = text
                dialog.selectedIndex = 0
            }
            Keys.onPressed: function (event) {
                var list = dialog.filteredActions()
                if (event.key === Qt.Key_Down) {
                    if (list.length > 0)
                        dialog.selectedIndex =
                                Math.min(dialog.selectedIndex + 1, list.length - 1)
                    event.accepted = true
                } else if (event.key === Qt.Key_Up) {
                    dialog.selectedIndex = Math.max(0, dialog.selectedIndex - 1)
                    event.accepted = true
                } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                    dialog.runSelected()
                    event.accepted = true
                } else if (event.key === Qt.Key_Escape) {
                    dialog.closePalette()
                    event.accepted = true
                }
            }
        }

        ListView {
            id: paletteList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: dialog.filteredActions()
            currentIndex: dialog.selectedIndex
            delegate: ItemDelegate {
                id: row
                width: paletteList.width
                height: Theme.toolHit
                highlighted: index === dialog.selectedIndex
                opacity: dialog.actionIsEnabled(modelData.id) ? 1.0 : 0.45

                // Opaque on purpose. The Basic style paints its own light
                // delegate background, and leaving this transparent let it show
                // through under text coloured for dark chrome — near-white on
                // near-white, which is why every unhighlighted row looked empty
                // while its grey menu and blue chord stayed readable.
                background: Rectangle {
                    color: row.highlighted ? Theme.primary : Theme.surfaceOverlay
                }

                contentItem: RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spaceSm
                    anchors.rightMargin: Theme.spaceSm
                    spacing: Theme.spaceSm
                    Label {
                        Layout.fillWidth: true
                        text: (modelData.label || "").replace(/&/g, "")
                        color: row.highlighted ? Theme.primaryOn : Theme.colorOnSurfaceEffective
                        font.pixelSize: Theme.fontBodySm
                        elide: Text.ElideRight
                    }
                    Label {
                        text: modelData.menu || ""
                        color: row.highlighted ? Theme.primaryOn : Theme.colorOnSurfaceMuted
                        font.pixelSize: Theme.fontLabelSm
                    }
                    Label {
                        text: dialog.shortcutForAction(modelData.id)
                        color: row.highlighted ? Theme.primaryOn : Theme.primary
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                    }
                }
                onClicked: {
                    dialog.selectedIndex = index
                    dialog.runSelected()
                }
            }
            ScrollBar.vertical: ThemedScrollBar { }
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
