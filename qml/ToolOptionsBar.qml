import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Contextual parameters for the active tool, on a bar under the main toolbar.
///
/// This is disclosure **level 1** (handbook 01/28): the two or three parameters
/// a tool is adjusted by constantly, always visible, never collapsible. It is
/// deliberately not a second Properties panel — everything that is not reached
/// mid-gesture belongs in the inspector's disclosure groups at level 2 and
/// below. Handbook 06 sanctions the overlap where it exists: both surfaces edit
/// through the same session slots, so neither can drift from the other.
///
/// Content is chosen by active tool, which is presence, not disclosure: an
/// absent control means the parameter does not apply to this tool, not that it
/// was collapsed.
Rectangle {
    id: root

    implicitHeight: Theme.toolbarHeight
    color: Theme.surfaceContainer

    Accessible.role: Accessible.ToolBar
    Accessible.name: qsTr("%1 options").arg(root.toolTitle)

    readonly property string tool: AppSession.activeTool
    readonly property bool isBrushLike: tool === "tool.brush" || tool === "tool.eraser"
    readonly property bool isMarquee: tool === "tool.select.rect" || tool === "tool.select.ellipse"
    readonly property bool isLasso: tool === "tool.select.lasso" || tool === "tool.select.polygon"
    readonly property bool isSelectLike: isMarquee || isLasso

    readonly property string toolTitle: {
        var all = AppSession.toolDescriptorsJson
        try {
            var list = JSON.parse(all || "[]")
            for (var i = 0; i < list.length; ++i) {
                if (list[i].id === root.tool)
                    return list[i].title
            }
        } catch (e) {
            // fall through
        }
        return qsTr("Tool")
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.borderEffective
    }

    /// Label + control pair, so every option reads the same way across tools.
    component Field: RowLayout {
        property alias label: fieldLabel.text
        spacing: Theme.spaceXs
        Label {
            id: fieldLabel
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabel
        }
    }

    component Divider: Rectangle {
        Layout.preferredWidth: 1
        Layout.preferredHeight: Math.round(Theme.toolbarHeight * 0.5)
        Layout.alignment: Qt.AlignVCenter
        color: Theme.borderSubtle
    }

    /// Compact numeric readout shared by the sliders.
    component ValueText: Label {
        color: Theme.primary
        font.pixelSize: Theme.fontMono
        font.family: "Noto Sans Mono"
        horizontalAlignment: Text.AlignRight
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceMd
        anchors.rightMargin: Theme.spaceMd
        spacing: Theme.spaceSm

        // Active tool identity, so the bar's contents are always attributable.
        ThemedIcon {
            source: Theme.iconUrl(AppSession.iconRoot, root.toolIconStem)
            size: Theme.iconMd
            color: Theme.iconOnSurfaceEffective
            Layout.alignment: Qt.AlignVCenter
        }
        Label {
            text: root.toolTitle
            color: Theme.colorOnSurfaceEffective
            font.pixelSize: Theme.fontLabel
            font.weight: Font.DemiBold
            Layout.alignment: Qt.AlignVCenter
        }
        Divider {}

        // Options overflow by scrolling rather than disappearing: handbook 06
        // forbids a narrow window silently dropping parameters.
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: optionsRow.implicitWidth
            flickableDirection: Flickable.HorizontalFlick
            boundsBehavior: Flickable.StopAtBounds
            clip: true

            RowLayout {
                id: optionsRow
                height: parent.height
                spacing: Theme.spaceMd

                // ── Brush and eraser ──────────────────────────────────────
                Field {
                    visible: root.isBrushLike
                    label: qsTr("Size")
                    Slider {
                        Layout.preferredWidth: 110
                        from: 1
                        to: 500
                        value: AppSession.brushSize
                        onMoved: AppSession.setBrushSize(value)
                    }
                    ValueText {
                        Layout.preferredWidth: 34
                        text: qsTr("%1 px").arg(Math.round(AppSession.brushSize))
                    }
                }
                Field {
                    visible: root.isBrushLike
                    label: qsTr("Hardness")
                    Slider {
                        Layout.preferredWidth: 90
                        from: 0
                        to: 1
                        value: AppSession.brushHardness
                        onMoved: AppSession.setBrushHardness(value)
                    }
                    ValueText {
                        Layout.preferredWidth: 30
                        text: qsTr("%1%").arg(Math.round(AppSession.brushHardness * 100))
                    }
                }
                Field {
                    visible: root.isBrushLike
                    label: qsTr("Texture")
                    Slider {
                        Layout.preferredWidth: 90
                        from: 0
                        to: 1
                        value: AppSession.brushTextureStrength
                        onMoved: AppSession.setBrushTextureStrength(value)
                    }
                }

                // ── Selection tools ───────────────────────────────────────
                Field {
                    visible: root.isSelectLike
                    label: qsTr("Mode")
                    Repeater {
                        model: [
                            { id: "replace", stem: "selection", tip: qsTr("Replace selection") },
                            { id: "add", stem: "selection-plus", tip: qsTr("Add to selection") },
                            { id: "subtract", stem: "minus-circle", tip: qsTr("Subtract from selection") },
                            { id: "intersect", stem: "intersect", tip: qsTr("Intersect with selection") }
                        ]
                        delegate: ToolButton {
                            id: modeButton
                            required property var modelData
                            implicitWidth: Theme.controlHeight
                            implicitHeight: Theme.controlHeight
                            padding: 0
                            checkable: true
                            checked: AppSession.selectionCombine === modelData.id
                            enabled: AppSession.hasDocument
                            onClicked: AppSession.setSelectionCombine(modelData.id)
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.tip
                            Accessible.name: modelData.tip
                            // Address the button by id, not by `parent`: inside
                            // contentItem/background `parent` types as a plain
                            // Item, so control state reads are unchecked.
                            contentItem: ThemedIcon {
                                anchors.centerIn: parent
                                source: Theme.iconUrl(AppSession.iconRoot, modeButton.modelData.stem)
                                size: Theme.iconMd
                                color: modeButton.enabled ? Theme.iconOnSurfaceEffective
                                                          : Theme.iconDisabledEffective
                            }
                            background: Rectangle {
                                radius: Theme.radiusSm
                                color: modeButton.checked
                                       ? Theme.toolActiveBg
                                       : (modeButton.hovered ? Theme.surfaceContainerHigh : "transparent")
                                border.color: modeButton.checked ? Theme.primary : "transparent"
                                border.width: 1
                            }
                        }
                    }
                }
                Label {
                    visible: root.isSelectLike
                    text: qsTr("Shift add · Alt subtract")
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                }

                // ── Paint bucket ──────────────────────────────────────────
                Field {
                    visible: root.tool === "tool.fill"
                    label: qsTr("Fill")
                    TextField {
                        Layout.preferredWidth: 90
                        text: AppSession.fillColorHex
                        onEditingFinished: AppSession.setActiveFillHex(text)
                        Accessible.name: qsTr("Fill colour hex")
                    }
                }

                // ── Text ──────────────────────────────────────────────────
                Field {
                    visible: root.tool === "tool.text"
                    label: qsTr("Font")
                    Label {
                        text: AppSession.textFontFamily.length > 0
                              ? AppSession.textFontFamily : qsTr("—")
                        color: Theme.colorOnSurfaceEffective
                        font.pixelSize: Theme.fontLabel
                        elide: Text.ElideRight
                        Layout.maximumWidth: 160
                    }
                    ValueText {
                        text: qsTr("%1 pt").arg(Math.round(AppSession.textFontSize))
                    }
                }

                // ── Crop and transform: commit is the whole point ─────────
                Field {
                    visible: root.tool === "tool.crop" || AppSession.cropPreviewActive
                    label: qsTr("Crop")
                    ValueText {
                        text: AppSession.cropPreviewActive
                              ? qsTr("%1 × %2").arg(AppSession.cropPreviewW).arg(AppSession.cropPreviewH)
                              : qsTr("drag on canvas")
                    }
                }
                Field {
                    visible: root.tool === "tool.transform" || AppSession.transformActive
                    label: qsTr("Rotate")
                    ValueText {
                        text: qsTr("%1°").arg(Math.round(AppSession.transformRot))
                    }
                    ThemedCheckBox {
                        text: qsTr("Constrain")
                        checked: AppSession.transformConstrain
                        enabled: AppSession.transformActive
                        onToggled: AppSession.updateTransformDraft(
                                       AppSession.transformTx, AppSession.transformTy,
                                       AppSession.transformSx, AppSession.transformSy,
                                       AppSession.transformRot, checked)
                    }
                }

                // ── Navigation ────────────────────────────────────────────
                Field {
                    visible: root.tool === "tool.zoom" || root.tool === "tool.pan"
                    label: qsTr("Zoom")
                    ValueText {
                        Layout.preferredWidth: 46
                        text: qsTr("%1%").arg(Math.round(AppSession.zoom * 100))
                    }
                    Button {
                        text: qsTr("Fit")
                        flat: true
                        enabled: AppSession.hasDocument
                        onClicked: AppSession.zoomToFit()
                    }
                    Button {
                        text: qsTr("100%")
                        flat: true
                        enabled: AppSession.hasDocument
                        onClicked: AppSession.setZoom(1.0)
                    }
                }

                Item { Layout.fillWidth: true }
            }
        }

        // Commit controls sit outside the scrolling region: an uncommitted
        // crop or transform must never scroll out of reach.
        Divider { visible: AppSession.transformActive || AppSession.cropPreviewActive }
        Button {
            visible: AppSession.transformActive || AppSession.cropPreviewActive
            text: qsTr("Apply")
            Layout.alignment: Qt.AlignVCenter
            onClicked: {
                if (AppSession.transformActive)
                    AppSession.commitTransform()
                else
                    AppSession.commitCrop(AppSession.cropPreviewX, AppSession.cropPreviewY,
                                          AppSession.cropPreviewW, AppSession.cropPreviewH)
            }
        }
        Button {
            visible: AppSession.transformActive || AppSession.cropPreviewActive
            text: qsTr("Cancel")
            Layout.alignment: Qt.AlignVCenter
            onClicked: {
                if (AppSession.transformActive)
                    AppSession.cancelTransform()
                else
                    AppSession.cancelCrop()
            }
        }
    }

    /// Phosphor stem for the active tool, from the shelf descriptors.
    readonly property string toolIconStem: {
        try {
            var list = JSON.parse(AppSession.toolDescriptorsJson || "[]")
            for (var i = 0; i < list.length; ++i) {
                if (list[i].id === root.tool)
                    return list[i].icon_key
            }
        } catch (e) {
            // fall through
        }
        return "paint-brush"
    }
}
