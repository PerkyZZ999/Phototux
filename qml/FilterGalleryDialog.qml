import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Filter Gallery — browse a shipped effect, preview it on canvas, then apply.
///
/// Extracted from `Main.qml`, where every dialog lived inline. Its only
/// coupling to the shell was the deferral helper, and that is injected rather
/// than reached for: a component that reads `root.*` can only ever live in the
/// file that defines `root`.

Dialog {
    id: dialog

    /// Runs `fn` after the current host slot returns. Handbook 32: a handler
    /// reacting to an AppSession signal must not call back into one
    /// synchronously. Passing the helper in keeps that one definition rather
    /// than re-deriving it here.
    required property var afterHostSlot

    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    title: qsTr("Filter Gallery")
    header: ThemedDialogHeader { text: dialog.title }
    width: Math.round(420 * Theme.densityScale)
    height: 360
    visible: AppSession.filterGalleryOpen
    // Both are deferred. `visible` is bound to host state that flips
    // inside `openFilterGallery` / `filterGalleryApply`, so a close that
    // lands during one of those slots would cancel the gallery from
    // inside the borrow that opened it. Qt.callLater also collapses the
    // rejected+closed pair into one cancel.
    onRejected: dialog.afterHostSlot(dialog.cancelIfOpen)
    onClosed: dialog.afterHostSlot(dialog.cancelIfOpen)

    function cancelIfOpen() {
        if (AppSession.filterGalleryOpen)
            AppSession.filterGalleryCancel()
    }

    background: Rectangle {
        color: Theme.surface
        border.color: Theme.border
        radius: Theme.radiusMd
    }

    padding: Theme.spaceMd

    // No anchors: a Dialog positions its own contentItem between the
    // header and the footer. Filling `parent` instead spans the whole
    // popup, which drew this content straight over the title bar.
    contentItem: ColumnLayout {
        spacing: Theme.spaceSm

        Label {
            text: qsTr("Browse a shipped effect, preview on the canvas, then Apply.")
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabelSm
        }
        ThemedComboBox {
            id: filterKindCombo
            Layout.fillWidth: true
            // A combo's only label is the `Label` beside it, which nothing
            // connects to it — assistive technology reaches it as an anonymous
            // combo box.
            Accessible.name: qsTr("Effect")
            model: dialog.catalog
            textRole: "label"
            valueRole: "id"
            function currentKind() {
                var row = dialog.catalog[currentIndex]
                return row ? row.id : "gaussian"
            }
            // No slider push-back needed: each slider binds to the host's
            // preview parameter and never assigns its own `value`, so starting
            // a new preview re-evaluates them.
            onActivated: AppSession.filterGalleryPreview(currentKind())
        }

        // One slider per slot the kind declares. The dialog used to name five
        // kinds explicitly, with the parameter labels and ranges written as
        // nested conditionals on the combo index — so the eight other kinds in
        // the vocabulary had no gallery entry, and adding one meant editing a
        // chain of ternaries.
        Repeater {
            model: dialog.currentSlots
            delegate: ColumnLayout {
                id: slotRow
                required property var modelData
                required property int index

                Layout.fillWidth: true
                spacing: Theme.spaceXxs

                Label {
                    text: slotRow.modelData.label
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                ThemedSlider {
                    Layout.fillWidth: true
                    from: slotRow.modelData.min
                    to: slotRow.modelData.max
                    value: slotRow.index === 1 ? AppSession.filterPreviewP1
                                               : AppSession.filterPreviewP0
                    Accessible.name: qsTr("%1 for %2")
                                     .arg(slotRow.modelData.label)
                                     .arg(filterKindCombo.currentText)
                    onMoved: {
                        if (slotRow.index === 1)
                            AppSession.filterGallerySetParams(
                                AppSession.filterPreviewP0, value, 0)
                        else
                            AppSession.filterGallerySetParams(
                                value, AppSession.filterPreviewP1, 0)
                    }
                }
            }
        }

        Item { Layout.fillHeight: true }
    }

    /// Filter kinds and their editor slots, as the engine declares them.
    readonly property var catalog: {
        try {
            return JSON.parse(AppSession.filterCatalogJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Editor slots of the kind currently selected in the combo.
    readonly property var currentSlots: {
        var row = dialog.catalog[filterKindCombo ? filterKindCombo.currentIndex : 0]
        return row ? row.slots : []
    }

    footer: ThemedDialogFooter {
        ThemedButton {
            text: qsTr("Preview")
            DialogButtonBox.buttonRole: DialogButtonBox.ActionRole
            onClicked: AppSession.filterGalleryPreview(filterKindCombo.currentKind())
        }
        ThemedButton {
            text: qsTr("Apply")
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            enabled: AppSession.filterPreviewActive
            onClicked: AppSession.filterGalleryApply()
        }
        ThemedButton {
            text: qsTr("Cancel")
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
            onClicked: AppSession.filterGalleryCancel()
        }
    }

    /// Seed the dialog with a default preview. Deferred, because
    /// `filterGalleryOpen` flips inside `openFilterGallery`: previewing
    /// from the notify handler would re-enter AppSession while it is
    /// still mutably borrowed and abort the process.
    function primeDefaultPreview() {
        if (!AppSession.filterGalleryOpen)
            return
        AppSession.filterGalleryPreview(filterKindCombo.currentKind())
    }

    Connections {
        target: AppSession
        function onFilterGalleryOpenChanged() {
            if (!AppSession.filterGalleryOpen)
                return
            filterKindCombo.currentIndex = 0
            dialog.afterHostSlot(dialog.primeDefaultPreview)
        }
    }
}
