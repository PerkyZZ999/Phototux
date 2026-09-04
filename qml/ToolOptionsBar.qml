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
    // The wand and colour range differ only in whether the flood is
    // contiguous, so they share every option including the combine mode.
    readonly property bool isColorSelect: tool === "tool.select.wand"
                                          || tool === "tool.select.color-range"
    readonly property bool isGradient: tool === "tool.gradient"
    // Photoshop puts the align and distribute buttons in the Move tool's
    // options bar, which is where someone arriving from it will look first.
    readonly property bool isMove: tool === "tool.move"

    /// Align and distribute operations, as the engine declares them.
    readonly property var alignOps: {
        try {
            return JSON.parse(AppSession.alignOpsJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Gradient shapes, as the engine declares them.
    readonly property var gradientKinds: {
        try {
            return JSON.parse(AppSession.gradientKindsJson || "[]")
        } catch (e) {
            return []
        }
    }
    readonly property bool isSelectLike: isMarquee || isLasso || isColorSelect

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
                //
                // The preset picker leads, the way Photoshop's options bar
                // opens with the brush preset well. It used to be a wrapped
                // row of flat buttons in Properties — the only part of that
                // panel's brush section with no home out here — so the whole
                // section had to stay for it.
                Field {
                    visible: root.isBrushLike
                    label: qsTr("Preset")
                    ThemedComboBox {
                        id: presetCombo
                        Layout.preferredWidth: 120
                        model: AppSession.brushPresetNames.length > 0
                               ? AppSession.brushPresetNames.split("|") : []
                        enabled: presetCombo.model.length > 0
                        Accessible.name: qsTr("Brush preset")
                        // A preset is applied, not held: picking one pushes
                        // size, hardness and texture into the brush, and the
                        // user then edits those directly. The combo showing
                        // the last pick would go stale on the first drag.
                        displayText: qsTr("Presets")
                        onActivated: AppSession.applyBrushPreset(currentIndex)
                    }
                    ToolButton {
                        implicitWidth: Theme.controlHeight
                        implicitHeight: Theme.controlHeight
                        padding: 0
                        enabled: AppSession.hasDocument
                        onClicked: AppSession.saveCurrentBrushPreset("Custom")
                        Accessible.name: qsTr("Save the current brush as a preset")
                        ThemedToolTip {
                            visible: parent.hovered
                            text: parent.Accessible.name
                        }
                        contentItem: ThemedIcon {
                            anchors.centerIn: parent
                            source: Theme.iconUrl(AppSession.iconRoot, "plus")
                            size: Theme.iconMd
                            color: parent.enabled ? Theme.iconOnSurfaceEffective
                                                  : Theme.iconDisabledEffective
                        }
                        background: Rectangle {
                            radius: Theme.radiusSm
                            color: parent.hovered && parent.enabled
                                   ? Theme.surfaceContainerHigh : "transparent"
                            border.color: parent.visualFocus ? Theme.focusRing : "transparent"
                            border.width: 1
                        }
                    }
                }
                Field {
                    visible: root.isBrushLike
                    label: qsTr("Size")
                    ThemedSlider {
                        Layout.preferredWidth: 110
                        from: 1
                        to: 500
                        value: AppSession.brushSize
                        Accessible.name: qsTr("Brush size, %1 pixels").arg(Math.round(AppSession.brushSize))
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
                    ThemedSlider {
                        Layout.preferredWidth: 90
                        from: 0
                        to: 1
                        value: AppSession.brushHardness
                        Accessible.name: qsTr("Brush hardness, %1 percent").arg(Math.round(AppSession.brushHardness * 100))
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
                    ThemedSlider {
                        Layout.preferredWidth: 90
                        from: 0
                        to: 1
                        value: AppSession.brushTextureStrength
                        Accessible.name: qsTr("Brush texture, %1 percent").arg(Math.round(AppSession.brushTextureStrength * 100))
                        onMoved: AppSession.setBrushTextureStrength(value)
                    }
                }

                // ── Gradient ──────────────────────────────────────────────
                Field {
                    visible: root.isGradient
                    label: qsTr("Shape")
                    Repeater {
                        model: root.gradientKinds
                        delegate: ToolButton {
                            id: gradButton
                            required property var modelData
                            implicitWidth: Theme.controlHeight
                            implicitHeight: Theme.controlHeight
                            padding: 0
                            checkable: true
                            checked: AppSession.gradientKind === gradButton.modelData.id
                            enabled: AppSession.hasDocument
                            onClicked: AppSession.setGradientKind(gradButton.modelData.id)
                            ThemedToolTip {
                                visible: parent.hovered
                                text: gradButton.modelData.label
                            }
                            Accessible.name: gradButton.modelData.label
                            contentItem: ThemedIcon {
                                anchors.centerIn: parent
                                source: Theme.iconUrl(AppSession.iconRoot,
                                                      gradButton.modelData.icon)
                                size: Theme.iconMd
                                color: gradButton.enabled ? Theme.iconOnSurfaceEffective
                                                          : Theme.iconDisabledEffective
                            }
                            background: Rectangle {
                                radius: Theme.radiusSm
                                color: gradButton.checked
                                       ? Theme.toolActiveBg
                                       : (gradButton.hovered ? Theme.surfaceContainerHigh
                                                             : "transparent")
                                border.color: gradButton.visualFocus
                                              ? Theme.focusRing
                                              : (gradButton.checked ? Theme.primary : "transparent")
                                border.width: 1
                            }
                        }
                    }
                }

                // ── Move: align and distribute ────────────────────────────
                Field {
                    visible: root.isMove
                    label: qsTr("Align")
                    Repeater {
                        model: root.alignOps.filter(op => !op.distribute)
                        delegate: ToolButton {
                            id: alignButton
                            required property var modelData
                            // Distribution needs a third layer to have anything
                            // to space out, so its buttons stay dim until there
                            // is one rather than accepting a click that does
                            // nothing.
                            readonly property bool available:
                                AppSession.hasDocument
                                && AppSession.layerCount >= alignButton.modelData.minTargets
                            implicitWidth: Theme.controlHeight
                            implicitHeight: Theme.controlHeight
                            padding: 0
                            enabled: alignButton.available
                            onClicked: AppSession.alignLayers(alignButton.modelData.id)
                            ThemedToolTip {
                                visible: parent.hovered
                                text: alignButton.modelData.label
                            }
                            Accessible.name: alignButton.modelData.label
                            contentItem: ThemedIcon {
                                anchors.centerIn: parent
                                source: Theme.iconUrl(AppSession.iconRoot,
                                                      alignButton.modelData.icon)
                                size: Theme.iconMd
                                color: alignButton.enabled ? Theme.iconOnSurfaceEffective
                                                           : Theme.iconDisabledEffective
                            }
                            background: Rectangle {
                                radius: Theme.radiusSm
                                color: alignButton.hovered && alignButton.enabled
                                       ? Theme.surfaceContainerHigh
                                       : "transparent"
                                border.color: alignButton.visualFocus ? Theme.focusRing : "transparent"
                                border.width: 1
                            }
                        }
                    }
                }
                // Distribution is its own labelled run rather than two more
                // buttons on the end of Align: they answer a different question
                // ("space these evenly", not "line these up"), they need a
                // third layer where aligning needs one, and Photoshop separates
                // them the same way. Eight unlabelled icons in one row is also
                // more than a glance can parse.
                Divider { visible: root.isMove }
                Field {
                    visible: root.isMove
                    label: qsTr("Distribute")
                    Repeater {
                        model: root.alignOps.filter(op => op.distribute)
                        delegate: ToolButton {
                            id: distributeButton
                            required property var modelData
                            readonly property bool available:
                                AppSession.hasDocument
                                && AppSession.layerCount >= distributeButton.modelData.minTargets
                            implicitWidth: Theme.controlHeight
                            implicitHeight: Theme.controlHeight
                            padding: 0
                            enabled: distributeButton.available
                            onClicked: AppSession.alignLayers(distributeButton.modelData.id)
                            ThemedToolTip {
                                visible: parent.hovered
                                text: distributeButton.available ? distributeButton.modelData.label : qsTr("%1 — needs %2 layers").arg(distributeButton.modelData.label).arg(distributeButton.modelData.minTargets)
                            }
                            Accessible.name: distributeButton.modelData.label
                            contentItem: ThemedIcon {
                                anchors.centerIn: parent
                                source: Theme.iconUrl(AppSession.iconRoot,
                                                      distributeButton.modelData.icon)
                                size: Theme.iconMd
                                color: distributeButton.enabled ? Theme.iconOnSurfaceEffective
                                                                : Theme.iconDisabledEffective
                            }
                            background: Rectangle {
                                radius: Theme.radiusSm
                                color: distributeButton.hovered && distributeButton.enabled
                                       ? Theme.surfaceContainerHigh
                                       : "transparent"
                                border.color: distributeButton.visualFocus ? Theme.focusRing : "transparent"
                                border.width: 1
                            }
                        }
                    }
                }

                // ── Selection tools ───────────────────────────────────────
                Field {
                    visible: root.isColorSelect
                    label: qsTr("Tolerance")
                    ThemedSlider {
                        implicitWidth: 120
                        from: 0
                        to: 1
                        value: AppSession.selectionTolerance
                        enabled: AppSession.hasDocument
                        Accessible.name: qsTr("Colour tolerance")
                        onMoved: AppSession.setSelectionTolerance(value)
                    }
                    Label {
                        text: Math.round(AppSession.selectionTolerance * 100) + "%"
                        color: Theme.primary
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                    }
                }
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
                            ThemedToolTip {
                                visible: parent.hovered
                                text: modelData.tip
                            }
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
                                border.color: modeButton.visualFocus
                                              ? Theme.focusRing
                                              : (modeButton.checked ? Theme.primary : "transparent")
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
                //
                // The foreground, because that is what the bucket pours:
                // `fillActiveLayer` reads `engine.colors.foreground`. It was
                // bound to `fillColorHex` / `setActiveFillHex`, which are the
                // *fill layer* inspector's pair — so the field showed a colour
                // the tool did not use (the #738CBF default whenever the active
                // layer carried no fill content), and editing it tried to
                // recolour a fill layer, which a raster layer refuses.
                Field {
                    visible: root.tool === "tool.fill"
                    label: qsTr("Fill")
                    ThemedTextField {
                        Layout.preferredWidth: 90
                        source: AppSession.foregroundHex
                        onEditingFinished: AppSession.setForegroundHex(text)
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
                    ThemedButton {
                        text: qsTr("Fit")
                        flat: true
                        enabled: AppSession.hasDocument
                        onClicked: AppSession.zoomToFit()
                    }
                    ThemedButton {
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
        ThemedButton {
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
        ThemedButton {
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
