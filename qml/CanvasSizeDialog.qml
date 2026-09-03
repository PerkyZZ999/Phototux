import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Image ▸ Canvas Size — more or less room around the image, no resampling.
///
/// The anchor grid is Photoshop's nine cells. The wire ids come from the
/// engine's `CanvasAnchor`, and so does the offset the resize uses: the two
/// agreeing about where an odd-numbered growth lands is not something to
/// arrange twice.
Dialog {
    id: dialog

    required property var afterHostSlot

    property string anchor: "center"

    readonly property var anchors_: [
        { id: "top-left",     row: 0, col: 0 },
        { id: "top",          row: 0, col: 1 },
        { id: "top-right",    row: 0, col: 2 },
        { id: "left",         row: 1, col: 0 },
        { id: "center",       row: 1, col: 1 },
        { id: "right",        row: 1, col: 2 },
        { id: "bottom-left",  row: 2, col: 0 },
        { id: "bottom",       row: 2, col: 1 },
        { id: "bottom-right", row: 2, col: 2 }
    ]

    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    closePolicy: Popup.CloseOnEscape
    title: qsTr("Canvas Size")
    header: ThemedDialogHeader { text: dialog.title }
    width: Math.round(420 * Theme.densityScale)
    height: 300
    padding: Theme.spaceMd
    visible: AppSession.canvasSizeOpen

    onVisibleChanged: if (dialog.visible) {
        widthSpin.value = AppSession.docWidth
        heightSpin.value = AppSession.docHeight
        dialog.anchor = "center"
    }

    onRejected: dialog.afterHostSlot(dialog.closeIfOpen)
    onClosed: dialog.afterHostSlot(dialog.closeIfOpen)

    function closeIfOpen() {
        if (AppSession.canvasSizeOpen)
            AppSession.closeCanvasSize()
    }

    background: Rectangle {
        color: Theme.surface
        border.color: Theme.border
        radius: Theme.radiusMd
    }

    contentItem: RowLayout {
        spacing: Theme.spaceLg

        ColumnLayout {
            spacing: Theme.spaceSm
            Layout.fillWidth: true

            RowLayout {
                spacing: Theme.spaceMd
                Layout.fillWidth: true
                Label {
                    text: qsTr("Width")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBody
                    Layout.preferredWidth: 52
                }
                ThemedSpinBox {
                    id: widthSpin
                    from: 1
                    to: AppSession.maxDocumentDimension
                    editable: true
                    Layout.fillWidth: true
                    Accessible.name: qsTr("Canvas width in pixels")
                }
            }
            RowLayout {
                spacing: Theme.spaceMd
                Layout.fillWidth: true
                Label {
                    text: qsTr("Height")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBody
                    Layout.preferredWidth: 52
                }
                ThemedSpinBox {
                    id: heightSpin
                    from: 1
                    to: AppSession.maxDocumentDimension
                    editable: true
                    Layout.fillWidth: true
                    Accessible.name: qsTr("Canvas height in pixels")
                }
            }
            Label {
                text: qsTr("Nothing is resampled. What falls outside a smaller canvas is cut.")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontBodySm
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
            Item { Layout.fillHeight: true }
        }

        ColumnLayout {
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Anchor")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabel
            }
            Grid {
                columns: 3
                spacing: Theme.spaceXxs
                Repeater {
                    model: dialog.anchors_
                    delegate: Rectangle {
                        required property var modelData
                        width: 26
                        height: 26
                        radius: Theme.radiusXs
                        color: dialog.anchor === modelData.id
                               ? Theme.primary : Theme.surfaceRaised
                        border.color: Theme.borderSubtle
                        border.width: 1
                        Accessible.role: Accessible.RadioButton
                        Accessible.name: modelData.id
                        Accessible.checked: dialog.anchor === modelData.id
                        TapHandler {
                            onTapped: dialog.anchor = modelData.id
                        }
                    }
                }
            }
            Item { Layout.fillHeight: true }
        }
    }

    footer: ThemedDialogFooter {
        ThemedButton {
            text: qsTr("Resize")
            prominence: "primary"
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            onClicked: {
                AppSession.resizeCanvas(widthSpin.value, heightSpin.value, dialog.anchor)
                AppSession.closeCanvasSize()
            }
        }
        ThemedButton {
            text: qsTr("Cancel")
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
            onClicked: AppSession.closeCanvasSize()
        }
    }
}
