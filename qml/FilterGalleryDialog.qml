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
    width: 420
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
            property var kinds: ["gaussian", "motion", "emboss", "sharpen", "noise"]
            model: [qsTr("Gaussian Blur"), qsTr("Motion Blur"), qsTr("Emboss"), qsTr("Sharpen"), qsTr("Noise")]
            function currentKind() {
                return kinds[currentIndex] || "gaussian"
            }
            onActivated: {
                AppSession.filterGalleryPreview(currentKind())
                filterP0Slider.value = AppSession.filterPreviewP0
                filterP1Slider.value = AppSession.filterPreviewP1
            }
        }
        Label {
            text: filterKindCombo.currentIndex === 0
                  ? qsTr("Radius")
                  : (filterKindCombo.currentIndex === 1
                     ? qsTr("Distance")
                     : (filterKindCombo.currentIndex === 2
                        ? qsTr("Strength")
                        : (filterKindCombo.currentIndex === 4
                           ? qsTr("Amount") : qsTr("Amount"))))
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
        }
        Slider {
            id: filterP0Slider
            Layout.fillWidth: true
            from: 0
            to: filterKindCombo.currentIndex === 0
                ? 64
                : (filterKindCombo.currentIndex === 4 ? 1 : 32)
            value: AppSession.filterPreviewP0
            onMoved: AppSession.filterGallerySetParams(
                         value, filterP1Slider.value, 0)
        }
        Label {
            visible: filterKindCombo.currentIndex === 1
                     || filterKindCombo.currentIndex === 2
            text: qsTr("Angle")
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
        }
        Slider {
            id: filterP1Slider
            Layout.fillWidth: true
            visible: filterKindCombo.currentIndex === 1
                     || filterKindCombo.currentIndex === 2
            from: 0
            to: 360
            value: AppSession.filterPreviewP1
            onMoved: AppSession.filterGallerySetParams(
                         filterP0Slider.value, value, 0)
        }
        Item { Layout.fillHeight: true }
    }

    footer: DialogButtonBox {
        Button {
            text: qsTr("Preview")
            DialogButtonBox.buttonRole: DialogButtonBox.ActionRole
            onClicked: AppSession.filterGalleryPreview(filterKindCombo.currentKind())
        }
        Button {
            text: qsTr("Apply")
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            enabled: AppSession.filterPreviewActive
            onClicked: AppSession.filterGalleryApply()
        }
        Button {
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
        AppSession.filterGalleryPreview("gaussian")
        filterP0Slider.value = AppSession.filterPreviewP0
        filterP1Slider.value = AppSession.filterPreviewP1
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
