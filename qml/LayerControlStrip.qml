import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Blend mode, opacity and locks, at the top of the Layers panel.
///
/// This is where Photoshop keeps them, and it is the most-used control cluster
/// in the application — the one a user reaches for without looking. They used
/// to live in Properties, three panels away from the layer they act on, which
/// also meant the panel with the tightest height budget was spending a fifth of
/// it on controls that belong somewhere else.
///
/// The two rows are Photoshop's two rows: blend and opacity, then the locks.
///
/// Nothing here is a per-row binding into a model, so unlike the list below it
/// this file may read `AppSession` freely — the re-entrancy hazard documented
/// in `LayersPanel.qml` is specific to delegate bindings evaluated inside a
/// host slot that is mutating the model.
ColumnLayout {
    id: root

    /// Blend modes as the engine declares them, with their family for banding.
    required property var blendModes
    /// Resolve a Phosphor icon stem to a URL, as the rest of the chrome does.
    required property var iconUrl

    spacing: Theme.spaceXxs

    readonly property bool hasLayer: AppSession.hasDocument
                                     && AppSession.activeLayerIndex >= 0

    /// Point the blend combo at the active layer's blend mode.
    ///
    /// A combo box holds its own selection, so host state has to be pushed into
    /// it rather than bound. Falls back to index 0 for a blend the model does
    /// not list.
    function syncBlendCombo() {
        if (!blendCombo)
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

    /// Push an opacity from the host into the slider.
    ///
    /// The epsilon matters: the slider emits on every assignment, so writing a
    /// value it already holds would round-trip back to the host and fight the
    /// user mid-drag.
    function setLayerOpacity(value) {
        if (!opacitySlider)
            return
        if (Math.abs(opacitySlider.value - value) > 0.001)
            opacitySlider.value = value
    }

    /// One lock toggle. Three buttons that differ only in label and action.
    ///
    /// `checked` matters as much as the click: the three buttons used to look
    /// identical whether the lock was on or off, so the only way to find out
    /// was to try an edit and watch it be refused.
    /// One lock toggle: an icon, as Photoshop has, with the sentence in the
    /// tooltip and in the accessible name.
    ///
    /// Four labelled buttons do not fit this strip. Sharing the row equally
    /// elided "Position" to "Positi…"; letting each size to its own text
    /// pushed "All" off the edge. Photoshop draws these as four small icons
    /// for the same reason.
    component LockButton: ChromeIconToolButton {
        required property string actionId
        /// The full sentence, for the tooltip and for assistive technology.
        required property string explanation
        checkable: true
        Accessible.name: explanation
        ThemedToolTip {
            visible: parent.hovered
            text: parent.explanation
        }
        enabled: AppSession.hasDocument
    }

    // Row 1 — blend mode and opacity, side by side the way Photoshop pairs them.
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs

        ThemedComboBox {
            id: blendCombo
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            model: root.blendModes
            textRole: "label"
            valueRole: "id"
            familyRole: "family"
            // A locked layer refuses `layer.set-blend`, so the combo must not
            // offer it — it would change on screen and then be corrected back.
            enabled: root.hasLayer && !AppSession.activeLayerLocked
            // Dimmed rather than blanked when the object selection disagrees:
            // the combo still has to show *a* mode, and "Mixed" beside it says
            // which layers it would apply to.
            opacity: AppSession.inspectorBlendMixed ? 0.85 : 1.0
            Accessible.name: qsTr("Blend mode")
            Component.onCompleted: root.syncBlendCombo()
            onActivated: AppSession.setActiveBlend(currentValue)
        }

        Label {
            text: qsTr("Opacity")
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabelSm
        }
        ThemedSlider {
            id: opacitySlider
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            from: 0
            to: 1
            value: AppSession.activeOpacity
            enabled: root.hasLayer && !AppSession.activeLayerLocked
            opacity: AppSession.inspectorOpacityMixed ? 0.85 : 1.0
            Accessible.name: AppSession.inspectorOpacityMixed
                             ? qsTr("Opacity mixed across selection")
                             : qsTr("Layer opacity percent %1")
                                   .arg(Math.round(opacitySlider.value * 100))
            onMoved: AppSession.setActiveOpacity(value)
        }
        Label {
            text: AppSession.inspectorOpacityMixed
                  ? qsTr("Mixed")
                  : (Math.round(opacitySlider.value * 100) + "%")
            color: !opacitySlider.enabled
                   ? Theme.colorOnSurfaceDisabled
                   : (AppSession.inspectorOpacityMixed
                      ? Theme.colorOnSurfaceMuted : Theme.primary)
            font.pixelSize: Theme.fontMono
            font.family: "Noto Sans Mono"
            font.italic: AppSession.inspectorOpacityMixed
            Layout.preferredWidth: 34
            horizontalAlignment: Text.AlignRight
        }
    }

    // "Mixed" for blend needs its own line — the combo already fills its half
    // of row one, and a badge squeezed beside it would elide before it read.
    Label {
        visible: AppSession.inspectorBlendMixed
        text: qsTr("Blend mode differs across the selected layers")
        color: Theme.colorOnSurfaceMuted
        font.pixelSize: Theme.fontLabelSm
        font.italic: true
        elide: Text.ElideRight
        Layout.fillWidth: true
    }

    // Row 2 — locks.
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        visible: AppSession.hasDocument
        Label {
            text: qsTr("Lock")
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabelSm
        }
        // Photoshop's order: transparent pixels, image pixels, position, all.
        LockButton {
            icon.source: root.iconUrl("checkerboard")
            explanation: qsTr("Lock transparent pixels")
            actionId: "action.layer.lock-transparency"
            checked: AppSession.activeLockAlpha
            onClicked: AppSession.invokeAction(actionId)
        }
        LockButton {
            icon.source: root.iconUrl("paint-brush")
            explanation: qsTr("Lock image pixels")
            actionId: "action.layer.lock-pixels"
            checked: AppSession.activeLockPixels
            onClicked: AppSession.invokeAction(actionId)
        }
        LockButton {
            icon.source: root.iconUrl("arrows-out-cardinal")
            explanation: qsTr("Lock position")
            actionId: "action.layer.lock-position"
            checked: AppSession.activeLockPosition
            onClicked: AppSession.invokeAction(actionId)
        }
        LockButton {
            icon.source: root.iconUrl("lock")
            explanation: qsTr("Lock all")
            actionId: "action.layer.lock-all"
            checked: AppSession.activeLayerLocked
            onClicked: AppSession.invokeAction(actionId)
        }
        Item { Layout.fillWidth: true }
    }
}
