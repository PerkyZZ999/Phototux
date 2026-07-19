import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

Popup {
    id: dialog
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape
    width: 760
    height: 480
    padding: 0

    signal requestNew()
    signal requestOpen()

    background: Rectangle {
        color: Theme.surface
        border.color: Theme.border
        radius: Theme.radiusLg
    }

    Overlay.modal: Rectangle {
        color: Theme.scrimModal
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Left branding pane
        Rectangle {
            Layout.preferredWidth: 280
            Layout.fillHeight: true
            color: Theme.surfaceContainer

            Rectangle {
                anchors.right: parent.right
                width: 1
                height: parent.height
                color: Theme.border
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Theme.spaceXl
                spacing: Theme.spaceXl

                Item { Layout.fillHeight: true }

                ColumnLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: Theme.spaceMd

                    Rectangle {
                        Layout.alignment: Qt.AlignHCenter
                        width: 128
                        height: 128
                        radius: Theme.radiusLg
                        color: Theme.surfaceSunken
                        border.color: Theme.border
                        clip: true

                        Rectangle {
                            anchors.fill: parent
                            gradient: Gradient {
                                GradientStop { position: 0.0; color: Theme.primarySubtle }
                                GradientStop { position: 1.0; color: "transparent" }
                            }
                        }

                        Image {
                            anchors.centerIn: parent
                            width: 112
                            height: 112
                            source: Theme.logoUrl
                            sourceSize: Qt.size(256, 256)
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            mipmap: true
                        }
                    }

                    Label {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("PhotoTux")
                        color: Theme.colorOnSurface
                        font.pixelSize: Theme.fontHeadline
                        font.weight: Font.DemiBold
                    }

                    Label {
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("Professional Image Environment")
                        color: Theme.colorOnSurfaceMuted
                        font.pixelSize: Theme.fontBodySm
                    }
                }

                Item { Layout.fillHeight: true }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXxs

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: Theme.borderSubtle
                    }

                    Label {
                        Layout.fillWidth: true
                        Layout.topMargin: Theme.spaceMd
                        horizontalAlignment: Text.AlignHCenter
                        text: qsTr("Version 0.1.0")
                        color: Theme.colorOnSurfaceMuted
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                    }

                    Label {
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                        text: qsTr("GPU ACCELERATED")
                        color: Theme.success
                        opacity: 0.85
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                        font.weight: Font.Medium
                    }
                }
            }
        }

        // Right actions
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            anchors.margins: 0
            spacing: 0

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spaceXl
                    spacing: Theme.spaceXl

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spaceMd

                        // New File
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 100
                            radius: Theme.radiusSm
                            color: newHover.hovered ? Theme.primaryHover : Theme.primary

                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: Theme.spaceSm

                                Image {
                                    Layout.alignment: Qt.AlignHCenter
                                    source: Theme.iconUrl(AppSession.iconRoot, "file-plus")
                                    width: 28
                                    height: 28
                                    sourceSize: Qt.size(28, 28)
                                }

                                Label {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: qsTr("New File")
                                    color: Theme.primaryOn
                                    font.pixelSize: Theme.fontLabel
                                    font.weight: Font.DemiBold
                                }
                            }

                            HoverHandler { id: newHover }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                Accessible.role: Accessible.Button
                                Accessible.name: qsTr("New File")
                                onClicked: {
                                    dialog.close()
                                    dialog.requestNew()
                                }
                            }
                        }

                        // Open File
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 100
                            radius: Theme.radiusSm
                            color: openHover.hovered ? Theme.surfaceContainerHigh : Theme.surfaceRaised
                            border.color: Theme.border
                            border.width: 1

                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: Theme.spaceSm

                                Image {
                                    Layout.alignment: Qt.AlignHCenter
                                    source: Theme.iconUrl(AppSession.iconRoot, "folder-open")
                                    width: 28
                                    height: 28
                                    sourceSize: Qt.size(28, 28)
                                }

                                Label {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: qsTr("Open File")
                                    color: Theme.colorOnSurface
                                    font.pixelSize: Theme.fontLabel
                                    font.weight: Font.DemiBold
                                }
                            }

                            HoverHandler { id: openHover }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                Accessible.role: Accessible.Button
                                Accessible.name: qsTr("Open File")
                                onClicked: {
                                    dialog.close()
                                    dialog.requestOpen()
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: Theme.spaceSm

                        RowLayout {
                            Layout.fillWidth: true
                            Label {
                                text: qsTr("RECENT FILES")
                                color: Theme.colorOnSurfaceVariant
                                font.pixelSize: Theme.fontLabelSm
                                font.letterSpacing: 1.2
                                font.weight: Font.Medium
                            }
                            Item { Layout.fillWidth: true }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: Theme.borderSubtle
                        }

                        Item {
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: Theme.spaceSm

                                Image {
                                    Layout.alignment: Qt.AlignHCenter
                                    source: Theme.iconUrl(AppSession.iconRoot, "image-square")
                                    width: 32
                                    height: 32
                                    sourceSize: Qt.size(32, 32)
                                    opacity: 0.45
                                }

                                Label {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: qsTr("No recent files yet")
                                    color: Theme.colorOnSurfaceMuted
                                    font.pixelSize: Theme.fontBodySm
                                }

                                Label {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: qsTr("Create or open an image to get started")
                                    color: Theme.colorOnSurfaceDisabled
                                    font.pixelSize: Theme.fontLabelSm
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
