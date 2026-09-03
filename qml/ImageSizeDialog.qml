import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Image ▸ Image Size — resample the document to new pixel dimensions.
///
/// Shaped after `FilterGalleryDialog`: bound to host state, explicit size,
/// close deferred through `afterHostSlot`.
Dialog {
    id: dialog

    /// `Qt.callLater` from the shell — the close path clears the host flag
    /// that `visible` is bound to, so it cannot call the session synchronously.
    required property var afterHostSlot

    /// Aspect ratio the dialog opened on, so the link survives rounding: a
    /// chain of integer round-trips through width and height drifts.
    property real sourceAspect: 1.0
    property bool linked: true
    /// Guards the two spin boxes against answering each other forever.
    property bool syncing: false

    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    closePolicy: Popup.CloseOnEscape
    title: qsTr("Image Size")
    header: ThemedDialogHeader { text: dialog.title }
    width: Math.round(420 * Theme.densityScale)
    height: 260
    padding: Theme.spaceMd
    visible: AppSession.imageSizeOpen

    onVisibleChanged: if (dialog.visible) {
        dialog.syncing = true
        widthSpin.value = AppSession.docWidth
        heightSpin.value = AppSession.docHeight
        dialog.sourceAspect = AppSession.docHeight > 0
                ? AppSession.docWidth / AppSession.docHeight : 1.0
        dialog.syncing = false
    }

    onRejected: dialog.afterHostSlot(dialog.closeIfOpen)
    onClosed: dialog.afterHostSlot(dialog.closeIfOpen)

    function closeIfOpen() {
        if (AppSession.imageSizeOpen)
            AppSession.closeImageSize()
    }

    function matchHeight() {
        if (dialog.syncing || !dialog.linked)
            return
        dialog.syncing = true
        heightSpin.value = Math.max(1, Math.round(widthSpin.value / dialog.sourceAspect))
        dialog.syncing = false
    }

    function matchWidth() {
        if (dialog.syncing || !dialog.linked)
            return
        dialog.syncing = true
        widthSpin.value = Math.max(1, Math.round(heightSpin.value * dialog.sourceAspect))
        dialog.syncing = false
    }

    background: Rectangle {
        color: Theme.surface
        border.color: Theme.border
        radius: Theme.radiusMd
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceSm

        RowLayout {
            spacing: Theme.spaceMd
            Layout.fillWidth: true
            Label {
                text: qsTr("Width")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBody
                Layout.preferredWidth: 56
            }
            ThemedSpinBox {
                id: widthSpin
                from: 1
                to: AppSession.maxDocumentDimension
                editable: true
                Layout.fillWidth: true
                Accessible.name: qsTr("Width in pixels")
                onValueChanged: dialog.matchHeight()
            }
            Label {
                text: qsTr("px")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBodySm
            }
        }

        RowLayout {
            spacing: Theme.spaceMd
            Layout.fillWidth: true
            Label {
                text: qsTr("Height")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBody
                Layout.preferredWidth: 56
            }
            ThemedSpinBox {
                id: heightSpin
                from: 1
                to: AppSession.maxDocumentDimension
                editable: true
                Layout.fillWidth: true
                Accessible.name: qsTr("Height in pixels")
                onValueChanged: dialog.matchWidth()
            }
            Label {
                text: qsTr("px")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBodySm
            }
        }

        ThemedCheckBox {
            text: qsTr("Constrain proportions")
            checked: dialog.linked
            onToggled: dialog.linked = checked
        }

        Label {
            text: qsTr("Every layer and mask is resampled. Undo restores the original pixels.")
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontBodySm
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        Item { Layout.fillHeight: true }
    }

    footer: ThemedDialogFooter {
        ThemedButton {
            text: qsTr("Resize")
            prominence: "primary"
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            onClicked: {
                AppSession.resizeImage(widthSpin.value, heightSpin.value)
                AppSession.closeImageSize()
            }
        }
        ThemedButton {
            text: qsTr("Cancel")
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
            onClicked: AppSession.closeImageSize()
        }
    }
}
