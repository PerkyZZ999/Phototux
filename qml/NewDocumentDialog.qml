import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: dialog
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape
    width: 420
    height: 340
    padding: 16

    signal accepted(string presetLabel, int width, int height)

    property string selectedPreset: "1080p"
    property int customW: 1920
    property int customH: 1080

    background: Rectangle {
        color: "#2B2B30"
        border.color: "#3D3D45"
        radius: 6
    }

    Overlay.modal: Rectangle {
        color: "#00000099"
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12

        Label {
            text: qsTr("New Document")
            color: "#EFF0F1"
            font.bold: true
            font.pixelSize: 14
        }

        Label {
            text: qsTr("Choose a size preset (ADR-013). 1080p is recommended.")
            color: "#A0A0A8"
            font.pixelSize: 11
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        GridLayout {
            columns: 2
            Layout.fillWidth: true
            columnSpacing: 8
            rowSpacing: 8

            Repeater {
                model: [
                    { label: "720p", sub: "1280×720" },
                    { label: "1080p", sub: "1920×1080" },
                    { label: "2K", sub: "2560×1440" },
                    { label: "4K", sub: "3840×2160" }
                ]
                delegate: Button {
                    Layout.fillWidth: true
                    checkable: true
                    checked: dialog.selectedPreset === modelData.label
                    text: modelData.label + "\n" + modelData.sub
                    onClicked: {
                        dialog.selectedPreset = modelData.label
                        // clear custom mode
                    }
                    background: Rectangle {
                        radius: 4
                        color: parent.checked ? "#3DAEE940" : "#323238"
                        border.color: parent.checked ? "#3DAEE9" : "#3D3D45"
                        border.width: 1
                    }
                    contentItem: Text {
                        text: parent.text
                        color: "#EFF0F1"
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: 12
                    }
                }
            }
        }

        Label {
            text: qsTr("Or custom size (px)")
            color: "#A0A0A8"
            font.pixelSize: 11
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            SpinBox {
                id: spinW
                from: 1
                to: 32768
                value: dialog.customW
                editable: true
                onValueModified: {
                    dialog.customW = value
                    dialog.selectedPreset = ""
                }
            }
            Label { text: "×"; color: "#A0A0A8" }
            SpinBox {
                id: spinH
                from: 1
                to: 32768
                value: dialog.customH
                editable: true
                onValueModified: {
                    dialog.customH = value
                    dialog.selectedPreset = ""
                }
            }
        }

        Item { Layout.fillHeight: true }

        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            Button {
                text: qsTr("Cancel")
                onClicked: dialog.close()
            }
            Button {
                text: qsTr("Create")
                highlighted: true
                onClicked: {
                    if (dialog.selectedPreset && dialog.selectedPreset.length > 0)
                        dialog.accepted(dialog.selectedPreset, 0, 0)
                    else
                        dialog.accepted("", spinW.value, spinH.value)
                    dialog.close()
                }
                background: Rectangle {
                    radius: 4
                    color: parent.down ? "#5CB8ED" : "#3DAEE9"
                }
                contentItem: Text {
                    text: parent.text
                    color: "#0A1620"
                    font.bold: true
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }
}
