import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Preferences — general, appearance, workspace and keymap.
///
/// Extracted from `Main.qml`. Everything it needs from the shell is a declared
/// property rather than a reach into `root`, so the seam is visible and the
/// component is movable.

Dialog {
    id: dialog

    /// Action registry rows, as `Main` projects them.
    required property var actionDescriptors

    /// Defers `fn` past the running host slot (handbook 32).
    required property var afterHostSlot

    /// Ordered right-dock panel ids.
    required property var dockRightStack

    /// Resolves a Phosphor stem to a themed icon URL.
    required property var iconUrl

    /// Panel registry rows.
    required property var panelDescriptors

    /// Predicate: is this panel id currently shown.
    required property var panelIsVisible

    /// Chord currently bound to an action id.
    required property var shortcutForAction

    /// Shell window height, for sizing against the viewport.
    required property int hostHeight
    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    title: qsTr("Preferences")
    header: ThemedDialogHeader { text: dialog.title }
    footer: ThemedDialogFooter {}
    standardButtons: Dialog.Close
    width: 480
    // Grow into the window when there is room: the themed header and
    // footer take real height now that content no longer draws over
    // them, and a fixed 560 clipped the last row mid-line.
    height: Math.min(640, Math.max(480, dialog.hostHeight - 160))
    visible: AppSession.preferencesOpen
    onRejected: dialog.afterHostSlot(dialog.closeIfOpen)
    onAccepted: dialog.afterHostSlot(dialog.closeIfOpen)
    onClosed: {
        dialog.capturingActionId = ""
        dialog.shortcutConflictHint = ""
        dialog.afterHostSlot(dialog.closeIfOpen)
    }

    function closeIfOpen() {
        if (AppSession.preferencesOpen)
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
        boundsBehavior: Flickable.StopAtBounds
        width: dialog.availableWidth
        height: dialog.availableHeight
        contentWidth: width
        contentHeight: prefsCol.implicitHeight
        focus: dialog.capturingActionId.length > 0
        ScrollBar.vertical: ThemedScrollBar {
            id: prefsScroll
            policy: ScrollBar.AlwaysOn
            Accessible.name: qsTr("Preferences scroll")
        }
        Keys.onPressed: function (event) {
            if (dialog.capturingActionId.length === 0)
                return
            if (event.key === Qt.Key_Escape) {
                dialog.capturingActionId = ""
                dialog.shortcutConflictHint = ""
                event.accepted = true
                return
            }
            if (event.key === Qt.Key_Backspace || event.key === Qt.Key_Delete) {
                AppSession.setActionShortcut(dialog.capturingActionId, "")
                dialog.capturingActionId = ""
                dialog.shortcutConflictHint = ""
                event.accepted = true
                return
            }
            var chord = dialog.chordFromKeyEvent(event)
            if (!chord)
                return
            var conflict = AppSession.shortcutConflictFor(
                        dialog.capturingActionId, chord)
            if (conflict && conflict.length > 0)
                dialog.shortcutConflictHint =
                        qsTr("Replaces binding on %1").arg(conflict)
            else
                dialog.shortcutConflictHint = ""
            AppSession.setActionShortcut(dialog.capturingActionId, chord)
            dialog.capturingActionId = ""
            event.accepted = true
        }

        ColumnLayout {
            id: prefsCol
            spacing: Theme.spaceMd
            // The scroll bar floats over the content rather than beside it, so
            // the column keeps its width clear or the handle sits on top of
            // the combo box and spin box on the right-hand edge.
            width: prefsFlick.width - prefsScroll.implicitWidth
            Accessible.name: qsTr("Preferences content")

            Label {
                text: qsTr("General")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontLabel
                font.weight: Font.DemiBold
            }
            ThemedCheckBox {
                text: qsTr("Show guides")
                checked: AppSession.prefShowGuides
                onToggled: AppSession.setPrefShowGuides(checked)
            }
            ThemedCheckBox {
                text: qsTr("Show grid")
                checked: AppSession.prefShowGrid
                onToggled: AppSession.setGridVisible(checked)
            }
            ThemedCheckBox {
                text: qsTr("Show rulers")
                checked: AppSession.prefShowRulers
                onToggled: AppSession.setRulersVisible(checked)
            }
            ThemedCheckBox {
                text: qsTr("Snap to grid / guides")
                checked: AppSession.prefSnap
                onToggled: AppSession.setSnapEnabled(checked)
            }
            ThemedCheckBox {
                text: qsTr("Restore last tool on launch")
                checked: AppSession.prefRestoreLastTool
                onToggled: AppSession.setPrefRestoreLastTool(checked)
            }

            Label {
                Layout.topMargin: Theme.spaceSm
                text: qsTr("Appearance & accessibility")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontLabel
                font.weight: Font.DemiBold
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("UI density")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                ThemedComboBox {
                    model: [qsTr("Dense"), qsTr("Comfortable")]
                    currentIndex: AppSession.prefUiDensity === "comfortable" ? 1 : 0
                    onActivated: AppSession.setPrefUiDensity(
                                     index === 1 ? "comfortable" : "dense")
                }
            }
            ThemedCheckBox {
                text: qsTr("High contrast chrome")
                checked: AppSession.prefHighContrast
                onToggled: AppSession.setPrefHighContrast(checked)
            }
            ThemedCheckBox {
                text: qsTr("Reduced motion")
                checked: AppSession.prefReducedMotion
                onToggled: AppSession.setPrefReducedMotion(checked)
            }
            ThemedCheckBox {
                text: qsTr("Safe start next launch")
                checked: AppSession.prefSafeStartNext
                onToggled: AppSession.setPrefSafeStartNext(checked)
                ThemedToolTip {
                    visible: parent.hovered
                    text: qsTr("Next launch uses essentials layout and ignores custom shortcuts (PHOTOTUX_SAFE_START=1 also works)")
                }
                Accessible.name: qsTr("Safe start next launch")
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("History retention")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                ThemedSpinBox {
                    id: historyRetentionSpin
                    from: 8
                    to: 512
                    stepSize: 8
                    editable: true
                    value: AppSession.prefHistoryRetention
                    onValueModified: AppSession.setPrefHistoryRetention(value)
                    // Editable commits do not always emit valueModified until focus leaves.
                    onValueChanged: {
                        if (value !== AppSession.prefHistoryRetention)
                            AppSession.setPrefHistoryRetention(value)
                    }
                    Accessible.name: qsTr("History retention steps")
                    ThemedToolTip {
                        visible: parent.hovered
                        text: qsTr("Max undo steps retained (oldest dropped when over budget)")
                    }
                }
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
                    var stack = dialog.dockRightStack
                    var all = dialog.panelDescriptors
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
                delegate: ThemedCheckBox {
                    required property var modelData
                    text: qsTr(modelData.title || modelData.id)
                    checked: dialog.panelIsVisible(modelData.id)
                    onToggled: AppSession.setPanelVisible(modelData.id, checked)
                }
            }

            Label {
                Layout.topMargin: Theme.spaceXs
                text: qsTr("Workspace presets")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontLabel
                font.weight: Font.DemiBold
            }
            Label {
                Layout.fillWidth: true
                text: qsTr("Built-ins plus your saved layouts. Saving stores the current panel visibility and dock layout (not the document).")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                wrapMode: Text.WordWrap
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                ThemedTextField {
                    id: userWorkspacePresetName
                    Layout.fillWidth: true
                    placeholderText: qsTr("Name for current layout")
                    Accessible.name: qsTr("User workspace preset name")
                    onAccepted: {
                        AppSession.saveUserWorkspacePreset(text)
                        text = ""
                    }
                }
                ThemedButton {
                    text: qsTr("Save")
                    enabled: userWorkspacePresetName.text.trim().length > 0
                    Accessible.name: qsTr("Save user workspace preset")
                    onClicked: {
                        AppSession.saveUserWorkspacePreset(userWorkspacePresetName.text)
                        userWorkspacePresetName.text = ""
                    }
                }
            }
            Repeater {
                model: {
                    try {
                        return JSON.parse(AppSession.workspacePresetsJson || "[]")
                    } catch (e) {
                        return []
                    }
                }
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    ThemedButton {
                        Layout.fillWidth: true
                        text: qsTr(modelData.title || modelData.id)
                        highlighted: AppSession.activeWorkspacePresetId === modelData.id
                        Accessible.name: qsTr("Apply workspace %1").arg(modelData.title || modelData.id)
                        onClicked: AppSession.applyWorkspacePreset(modelData.id)
                    }
                    ToolButton {
                        id: deletePresetBtn
                        visible: {
                            var id = modelData.id || ""
                            return id.indexOf("workspace.preset.user.") === 0
                        }
                        implicitWidth: Theme.panelHeaderBtn
                        implicitHeight: Theme.panelHeaderBtn
                        padding: 0
                        display: AbstractButton.IconOnly
                        icon.source: dialog.iconUrl("trash")
                        icon.width: Theme.iconMd
                        icon.height: Theme.iconMd
                        Accessible.name: qsTr("Delete workspace preset %1").arg(modelData.title || modelData.id)
                        ThemedToolTip {
                            visible: parent.hovered
                            text: qsTr("Delete user preset")
                        }
                        contentItem: Item {
                            implicitWidth: Theme.iconMd
                            implicitHeight: Theme.iconMd
                            ThemedIcon {
    parent: Overlay.overlay
    anchors.centerIn: parent
                                source: deletePresetBtn.icon.source
                                size: Theme.iconMd
                                color: deletePresetBtn.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                            }
                        }
                        background: Rectangle {
                            radius: Theme.radiusXs
                            color: deletePresetBtn.hovered ? Theme.surfaceContainerHigh : "transparent"
                        }
                        onClicked: AppSession.deleteUserWorkspacePreset(modelData.id)
                    }
                }
            }
            ThemedButton {
                text: qsTr("Restore last saved layout")
                onClicked: AppSession.restoreLastSavedWorkspace()
            }
            ThemedButton {
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
                visible: dialog.shortcutConflictHint.length > 0
                         || dialog.capturingActionId.length > 0
                Layout.fillWidth: true
                text: dialog.capturingActionId.length > 0
                      ? qsTr("Waiting for shortcut…")
                      : dialog.shortcutConflictHint
                color: Theme.warning
                font.pixelSize: Theme.fontLabelSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                model: {
                    var _ = AppSession.actionShortcutsJson
                    var out = []
                    var all = dialog.actionDescriptors
                    for (var i = 0; i < all.length; ++i) {
                        if (all[i].shortcut || dialog.shortcutForAction(all[i].id))
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
                    ThemedButton {
                        implicitWidth: 140
                        text: dialog.capturingActionId === modelData.id
                              ? qsTr("Press keys…")
                              : (dialog.shortcutForAction(modelData.id) || qsTr("None"))
                        onClicked: {
                            dialog.capturingActionId = modelData.id
                            dialog.shortcutConflictHint = ""
                            prefsFlick.forceActiveFocus()
                        }
                    }
                }
            }

            ThemedButton {
                text: qsTr("Reset shortcuts to defaults")
                onClicked: {
                    dialog.capturingActionId = ""
                    dialog.shortcutConflictHint = ""
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
