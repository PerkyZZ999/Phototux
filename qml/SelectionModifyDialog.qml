import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Radius prompt for Select ▸ Modify.
///
/// The five entries are spelled with an ellipsis, which is a promise that the
/// user gets to say how much, and for feather especially the radius *is* the
/// operation. They used to apply the registry's default the moment they were
/// clicked — a 2px expand, a 4px feather — so the promise was broken and the
/// commands were close to useless.
///
/// Its own file rather than another block in `Main.qml`, and shaped after
/// `FilterGalleryDialog`: bound to host state, explicit size, deferred close.
Dialog {
    id: dialog

    /// `Qt.callLater` from the shell — see `FilterGalleryDialog` for why the
    /// close path cannot call the host synchronously.
    required property var afterHostSlot

    /// The op this prompt is collecting a radius for, taken when it opens.
    ///
    /// Read once rather than bound: closing clears it on the host, and a live
    /// binding would blank the title on the way out.
    property string op: ""

    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    closePolicy: Popup.CloseOnEscape
    title: AppSession.selectionModifyTitle
    header: ThemedDialogHeader { text: dialog.title }
    width: Math.round(420 * Theme.densityScale)
    height: 200
    padding: Theme.spaceMd
    visible: AppSession.selectionModifyOp.length > 0

    onVisibleChanged: if (dialog.visible) {
        dialog.op = AppSession.selectionModifyOp
        radius.value = AppSession.selectionModifyRadius
    }

    // Deferred for the same reason the filter gallery defers: `visible` is
    // bound to host state that this handler clears, so a synchronous call
    // would re-enter the session from inside its own notify.
    onRejected: dialog.afterHostSlot(dialog.closeIfOpen)
    onClosed: dialog.afterHostSlot(dialog.closeIfOpen)

    function closeIfOpen() {
        if (AppSession.selectionModifyOp.length > 0)
            AppSession.closeSelectionModifyPrompt()
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
                text: qsTr("Radius")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBody
            }
            ThemedSpinBox {
                id: radius
                from: 1
                to: 512
                value: 2
                editable: true
                Layout.fillWidth: true
                Accessible.name: qsTr("Radius in pixels")
            }
            Label {
                text: qsTr("px")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBodySm
            }
        }
        Item { Layout.fillHeight: true }
    }

    footer: ThemedDialogFooter {
        ThemedButton {
            text: qsTr("OK")
            prominence: "primary"
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            onClicked: {
                AppSession.modifySelection(dialog.op, radius.value)
                AppSession.closeSelectionModifyPrompt()
            }
        }
        ThemedButton {
            text: qsTr("Cancel")
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
            onClicked: AppSession.closeSelectionModifyPrompt()
        }
    }
}
