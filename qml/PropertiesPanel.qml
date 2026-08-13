// Properties panel body — the right dock's per-layer editor (handbook 01).
//
// This lived inline in `Main.qml`, where it was the single largest thing in the
// file: about 1,600 lines between the dock's `Flickable` and the Navigator
// header, so that reading either neighbour meant scrolling past the whole
// adjustment stack. Extracting it does not change what the panel does — the
// body below is the same tree — but it makes the panel's dependencies
// enumerable, which they were not while every id in `Main.qml` was in scope.
//
// The seam is deliberately narrow: eleven inbound values, one signal out, and
// two functions the shell calls to push host state into controls that hold
// their own editing state. The properties are named exactly as they are on
// `Main.qml`'s root so the body's `root.adjRange(...)` and friends resolve
// against this component instead — the extraction is a move, not a rewrite.
//
// The `Flickable` stays behind: it carries the dock's `Layout.*` attachments
// and its `contentHeight` binding, which are the dock's business, not the
// panel's.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

ColumnLayout {
    id: root

    // Shell helpers, passed rather than reached for. `var` because these are
    // functions on the shell root; QML has no narrower type for them.
    required property var adjRange
    required property var iconUrl
    required property var runAction
    required property var isTransformTool
    required property var isCropTool
    required property var isSelectTool
    required property var selectionCombineLabel

    // Derived host state the shell already computes once for several panels.
    required property string activeLayerKind
    required property bool activeLayerHasMask
    required property bool activeMaskEnabled

    /// Present GPU status line, owned by the canvas host.
    required property string gpuStatus

    /// Raised when the user asks to embed an ICC profile. The file dialog
    /// belongs to the shell, so the panel asks rather than opens.
    signal embedIccRequested

    /// Point the blend combo at the active layer's blend mode.
    ///
    /// A combo box holds its own selection, so host state has to be pushed into
    /// it rather than bound — and the combo is in here now, so the push lives
    /// here too. Falls back to index 0 for a blend the model does not list.
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
        if (!layerOpacitySlider)
            return
        if (Math.abs(layerOpacitySlider.value - value) > 0.001)
            layerOpacitySlider.value = value
    }

    // Edit target + selection context (distinct chrome)
    Rectangle {
        Layout.fillWidth: true
        visible: AppSession.hasDocument
        radius: Theme.radiusSm
        color: Theme.surfaceContainerHigh
        border.color: Theme.borderSubtle
        border.width: 1
        implicitHeight: editTargetCol.implicitHeight + Theme.spaceSm * 2

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 2
            color: Theme.primary
            radius: 1
        }

        ColumnLayout {
            id: editTargetCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Theme.spaceSm + 2
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceXs

            Label {
                text: qsTr("Edit target")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
                font.weight: Font.DemiBold
            }
            Label {
                text: {
                    var kind = AppSession.activeLayerKind.length > 0
                               ? AppSession.activeLayerKind
                               : qsTr("layer")
                    var obj = AppSession.objectSelectionLabel.length > 0
                              ? qsTr("object: %1").arg(AppSession.objectSelectionLabel)
                              : qsTr("no object selection")
                    var sel = AppSession.pixelSelectionActive
                              ? qsTr("pixel selection active")
                              : qsTr("no pixel selection")
                    return qsTr("%1 · %2 · %3 · %4")
                           .arg(kind)
                           .arg(AppSession.editTargetLabel)
                           .arg(obj)
                           .arg(sel)
                }
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Button {
                    text: qsTr("Layer pixels")
                    checkable: true
                    checked: AppSession.editTarget === "layer"
                    Layout.fillWidth: true
                    Accessible.name: qsTr("Edit layer pixels")
                    onClicked: AppSession.setMaskEditTarget(false)
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: parent.checked
                               ? Theme.toolActiveBg
                               : (parent.hovered ? Theme.surfaceRaised : Theme.surfaceContainer)
                        border.color: parent.checked ? Theme.primary : Theme.borderSubtle
                        border.width: 1
                    }
                    contentItem: Text {
                        text: parent.text
                        color: parent.checked ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
                        font.pixelSize: Theme.fontLabel
                        font.weight: parent.checked ? Font.DemiBold : Font.Normal
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
                Button {
                    text: qsTr("Layer mask")
                    checkable: true
                    checked: AppSession.editTarget === "mask"
                    enabled: root.activeLayerHasMask
                    Layout.fillWidth: true
                    Accessible.name: qsTr("Edit layer mask")
                    onClicked: AppSession.setMaskEditTarget(true)
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: parent.enabled
                               ? (parent.checked
                                  ? Theme.toolActiveBg
                                  : (parent.hovered ? Theme.surfaceRaised : Theme.surfaceContainer))
                               : Theme.surfaceSunken
                        border.color: parent.checked ? Theme.primary : Theme.borderSubtle
                        border.width: 1
                        opacity: parent.enabled ? 1.0 : 0.55
                    }
                    contentItem: Text {
                        text: parent.text
                        color: parent.checked ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
                        font.pixelSize: Theme.fontLabel
                        font.weight: parent.checked ? Font.DemiBold : Font.Normal
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }
    }

    Label {
        visible: AppSession.hasDocument && AppSession.lastAnnounce.length > 0
        text: AppSession.lastAnnounce
        color: Theme.colorOnSurfaceVariant
        font.pixelSize: Theme.fontLabelSm
        wrapMode: Text.WordWrap
        Layout.fillWidth: true
        Accessible.name: AppSession.lastAnnounce
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        visible: AppSession.hasDocument
        Button {
            text: qsTr("Lock px")
            Layout.fillWidth: true
            onClicked: root.runAction("action.layer.lock-pixels")
            background: Rectangle {
                radius: Theme.radiusSm
                color: parent.down || parent.hovered
                       ? Theme.surfaceRaised : Theme.surfaceContainer
                border.color: Theme.borderSubtle
                border.width: 1
            }
            contentItem: Text {
                text: parent.text
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontLabel
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
        Button {
            text: qsTr("Lock pos")
            Layout.fillWidth: true
            onClicked: root.runAction("action.layer.lock-position")
            background: Rectangle {
                radius: Theme.radiusSm
                color: parent.down || parent.hovered
                       ? Theme.surfaceRaised : Theme.surfaceContainer
                border.color: Theme.borderSubtle
                border.width: 1
            }
            contentItem: Text {
                text: parent.text
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontLabel
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
        Button {
            text: qsTr("Lock all")
            Layout.fillWidth: true
            onClicked: root.runAction("action.layer.lock-all")
            background: Rectangle {
                radius: Theme.radiusSm
                color: parent.down || parent.hovered
                       ? Theme.surfaceRaised : Theme.surfaceContainer
                border.color: Theme.borderSubtle
                border.width: 1
            }
            contentItem: Text {
                text: parent.text
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontLabel
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }

    // Selection combine modes
    DisclosureGroup {
        groupId: "inspector.selection"
        title: qsTr("Selection")
        visible: root.isSelectTool()
        // The combine mode silently changes what the next
        // drag does, so it is what the header must confirm.
        summary: root.selectionCombineLabel(AppSession.selectionCombine)

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Pixel selection")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Repeater {
                    model: [
                        { id: "replace", stem: "selection", tip: qsTr("Replace") },
                        { id: "add", stem: "selection-plus", tip: qsTr("Add") },
                        { id: "subtract", stem: "minus-circle", tip: qsTr("Subtract") },
                        { id: "intersect", stem: "intersect", tip: qsTr("Intersect") }
                    ]
                    delegate: ToolButton {
                        implicitWidth: 32
                        implicitHeight: 28
                        checkable: true
                        checked: AppSession.selectionCombine === modelData.id
                        icon.source: root.iconUrl(modelData.stem)
                        icon.color: enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                        icon.width: 16
                        icon.height: 16
                        contentItem: ThemedIcon {
                            anchors.centerIn: parent
                            source: parent.icon.source
                            size: parent.icon.height
                            color: parent.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                        }
                        enabled: AppSession.hasDocument
                        onClicked: AppSession.setSelectionCombine(modelData.id)
                        ToolTip.visible: hovered
                        ToolTip.text: modelData.tip
                        background: Rectangle {
                            radius: Theme.radiusSm
                            color: parent.checked
                                   ? Theme.toolActiveBg
                                   : (parent.hovered ? Theme.surfaceContainerHigh : "transparent")
                            border.color: parent.checked ? Theme.primary : "transparent"
                            border.width: 1
                        }
                    }
                }
            }
            Label {
                text: qsTr("Shift add · Alt subtract · Shift+Alt intersect")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }

    DisclosureGroup {
        groupId: "inspector.brush"
        title: qsTr("Brush")
        visible: AppSession.activeTool === "tool.brush"
                 || AppSession.activeTool === "tool.eraser"
        summary: qsTr("%1 px").arg(Math.round(AppSession.brushSize))

        ColumnLayout {
            spacing: Theme.spaceMd

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                RowLayout {
                    Layout.fillWidth: true
                    Label {
                        text: qsTr("Brush Size")
                        color: Theme.colorOnSurface
                        font.pixelSize: Theme.fontBodySm
                        Layout.fillWidth: true
                    }
                    Label {
                        text: Math.round(brushSlider.value) + " px"
                        color: Theme.primary
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                    }
                }
                Slider {
                    id: brushSlider
                    Layout.fillWidth: true
                    from: 1
                    to: 200
                    value: AppSession.brushSize
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setBrushSize(value)
                    // Dragging a Slider breaks its value binding, so host-side
                    // changes (presets, new document) need an explicit re-push.
                    Connections {
                        target: AppSession
                        function onBrushSizeChanged() {
                            brushSlider.value = AppSession.brushSize
                        }
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                RowLayout {
                    Layout.fillWidth: true
                    Label {
                        text: qsTr("Hardness")
                        color: Theme.colorOnSurface
                        font.pixelSize: Theme.fontBodySm
                        Layout.fillWidth: true
                    }
                    Label {
                        text: Math.round(hardnessSlider.value * 100) + " %"
                        color: Theme.primary
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                    }
                }
                Slider {
                    id: hardnessSlider
                    Layout.fillWidth: true
                    from: 0
                    to: 1
                    value: AppSession.brushHardness
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setBrushHardness(value)
                    Connections {
                        target: AppSession
                        function onBrushHardnessChanged() {
                            hardnessSlider.value = AppSession.brushHardness
                        }
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                RowLayout {
                    Layout.fillWidth: true
                    Label {
                        text: qsTr("Texture")
                        color: Theme.colorOnSurface
                        font.pixelSize: Theme.fontBodySm
                        Layout.fillWidth: true
                    }
                    Label {
                        text: Math.round(textureSlider.value * 100) + " %"
                        color: Theme.primary
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                    }
                }
                Slider {
                    id: textureSlider
                    Layout.fillWidth: true
                    from: 0
                    to: 1
                    value: AppSession.brushTextureStrength
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setBrushTextureStrength(value)
                    Accessible.name: qsTr("Brush tip texture strength")
                    Connections {
                        target: AppSession
                        function onBrushTextureStrengthChanged() {
                            textureSlider.value = AppSession.brushTextureStrength
                        }
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Label {
                    text: qsTr("Brush presets")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    Repeater {
                        model: AppSession.brushPresetNames.length > 0
                               ? AppSession.brushPresetNames.split("|") : []
                        Button {
                            text: modelData
                            flat: true
                            onClicked: AppSession.applyBrushPreset(index)
                            Accessible.name: qsTr("Apply brush preset %1").arg(modelData)
                        }
                    }
                }
                Button {
                    text: qsTr("Save current as preset")
                    flat: true
                    enabled: AppSession.hasDocument
                    onClicked: AppSession.saveCurrentBrushPreset("Custom")
                }
            }
        }
    }

    // Fill layer chrome
    DisclosureGroup {
        groupId: "inspector.fill"
        title: qsTr("Fill")
        visible: root.activeLayerKind === "fill"
        summary: AppSession.fillColorHex

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Fill")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                Label {
                    text: qsTr("Color")
                    color: Theme.colorOnSurfaceVariant
                    font.pixelSize: Theme.fontLabelSm
                }
                TextField {
                    Layout.fillWidth: true
                    text: AppSession.fillColorHex
                    onEditingFinished: AppSession.setActiveFillHex(text)
                }
                Rectangle {
                    width: 22
                    height: 22
                    radius: Theme.radiusXs
                    color: AppSession.fillColorHex
                    border.color: Theme.border
                }
            }
        }
    }

    // Character / text layer chrome
    DisclosureGroup {
        groupId: "inspector.text"
        title: qsTr("Character")
        visible: AppSession.textLayerActive
                 || AppSession.activeTool === "tool.text"
                 || root.activeLayerKind === "text"
        summary: AppSession.textFontFamily

        ColumnLayout {
            id: characterProps
            Layout.fillWidth: true
            spacing: Theme.spaceXs

            function pushText() {
                AppSession.updateActiveText(
                            textBodyField.text,
                            fontFamilyCombo.currentText,
                            fontSizeSpin.value,
                            trackingSpin.value,
                            lineSpacingSpin.value / 100.0,
                            alignCombo.currentIndex,
                            textColorField.text)
                AppSession.updateActiveTextFrame(
                            frameWSpin.value,
                            frameHSpin.value,
                            wrapCheck.checked)
            }

            Label {
                text: qsTr("Character")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            Label {
                visible: !AppSession.textLayerActive
                text: qsTr("Click the canvas with the Text tool to create a text layer.")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
            TextField {
                id: textBodyField
                Layout.fillWidth: true
                enabled: AppSession.textLayerActive
                text: AppSession.textBody
                placeholderText: qsTr("Text")
                onEditingFinished: characterProps.pushText()
            }
            ThemedComboBox {
                id: fontFamilyCombo
                Layout.fillWidth: true
                enabled: AppSession.textLayerActive
                model: {
                    try {
                        return JSON.parse(AppSession.availableFontsJson)
                    } catch (e) {
                        return ["Noto Sans", "DejaVu Sans"]
                    }
                }
                // Fontconfig discovery is deferred out of cold boot; ask for the
                // real family list the first time this chrome is reachable.
                Component.onCompleted: {
                    AppSession.ensureFontsDiscovered()
                    var i = model.indexOf(AppSession.textFontFamily)
                    currentIndex = i >= 0 ? i : 0
                }
                onPressedChanged: if (pressed) AppSession.ensureFontsDiscovered()
                Connections {
                    target: AppSession
                    function onTextFontFamilyChanged() {
                        var i = fontFamilyCombo.model.indexOf(
                                    AppSession.textFontFamily)
                        if (i >= 0)
                            fontFamilyCombo.currentIndex = i
                    }
                }
                onActivated: characterProps.pushText()
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("Size")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                ThemedSpinBox {
                    id: fontSizeSpin
                    from: 4
                    to: 512
                    value: Math.round(AppSession.textFontSize)
                    enabled: AppSession.textLayerActive
                    onValueModified: characterProps.pushText()
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("Tracking")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                ThemedSpinBox {
                    id: trackingSpin
                    from: -20
                    to: 40
                    value: Math.round(AppSession.textTracking)
                    enabled: AppSession.textLayerActive
                    onValueModified: characterProps.pushText()
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("Leading")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                ThemedSpinBox {
                    id: lineSpacingSpin
                    from: 50
                    to: 400
                    value: Math.round(AppSession.textLineSpacing * 100)
                    enabled: AppSession.textLayerActive
                    textFromValue: function (v) { return (v / 100).toFixed(2) }
                    valueFromText: function (t) { return Math.round(parseFloat(t) * 100) }
                    onValueModified: characterProps.pushText()
                }
            }
            ThemedComboBox {
                id: alignCombo
                Layout.fillWidth: true
                enabled: AppSession.textLayerActive
                model: [qsTr("Left"), qsTr("Center"), qsTr("Right")]
                currentIndex: AppSession.textAlignment
                onActivated: characterProps.pushText()
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("Color")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                TextField {
                    id: textColorField
                    Layout.fillWidth: true
                    enabled: AppSession.textLayerActive
                    text: AppSession.textColorHex
                    onEditingFinished: characterProps.pushText()
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("Frame W")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                ThemedSpinBox {
                    id: frameWSpin
                    from: 0
                    to: 16384
                    value: Math.round(AppSession.textFrameW)
                    enabled: AppSession.textLayerActive
                    onValueModified: characterProps.pushText()
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("Frame H")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                ThemedSpinBox {
                    id: frameHSpin
                    from: 0
                    to: 16384
                    value: Math.round(AppSession.textFrameH)
                    enabled: AppSession.textLayerActive
                    onValueModified: characterProps.pushText()
                }
            }
            ThemedCheckBox {
                id: wrapCheck
                text: qsTr("Wrap within frame")
                checked: AppSession.textWrap
                enabled: AppSession.textLayerActive
                onToggled: characterProps.pushText()
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                text: qsTr("Keep editable until you bake. Bake Text converts to pixels and discards the editable text layer.")
            }
            Button {
                text: qsTr("Bake Text")
                enabled: AppSession.textLayerActive && !AppSession.ioBusy
                onClicked: AppSession.bakeTextLayer()
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                text: qsTr("Keep editable: leave this panel; do not bake.")
            }
            Connections {
                target: AppSession
                function onTextBodyChanged() {
                    if (!textBodyField.activeFocus)
                        textBodyField.text = AppSession.textBody
                }
                function onTextFontSizeChanged() {
                    fontSizeSpin.value = Math.round(AppSession.textFontSize)
                }
                function onTextColorHexChanged() {
                    if (!textColorField.activeFocus)
                        textColorField.text = AppSession.textColorHex
                }
                function onTextAlignmentChanged() {
                    alignCombo.currentIndex = AppSession.textAlignment
                }
                function onTextFrameWChanged() {
                    frameWSpin.value = Math.round(AppSession.textFrameW)
                }
                function onTextFrameHChanged() {
                    frameHSpin.value = Math.round(AppSession.textFrameH)
                }
                function onTextWrapChanged() {
                    wrapCheck.checked = AppSession.textWrap
                }
            }
        }
    }

    // Path edit chrome
    DisclosureGroup {
        groupId: "inspector.path"
        title: qsTr("Path")
        visible: AppSession.activeTool === "tool.path-edit"
                 || root.activeLayerKind === "shape"
        summary: AppSession.pathClosed
                 ? qsTr("%1 anchors · closed").arg(AppSession.pathAnchorCount)
                 : qsTr("%1 anchors · open").arg(AppSession.pathAnchorCount)

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Path Edit")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                text: qsTr("Drag anchors to move. Click empty to add. Delete removes selected. Close toggles the path.")
            }
            ThemedCheckBox {
                text: qsTr("Closed")
                checked: AppSession.pathClosed
                enabled: AppSession.pathAnchorCount >= 2
                onToggled: AppSession.pathSetClosed(checked)
            }
            Label {
                text: qsTr("Anchors: %1").arg(AppSession.pathAnchorCount)
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            Label {
                visible: AppSession.pathEditSelected >= 0
                text: qsTr("Selected: %1").arg(AppSession.pathEditSelected)
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
            }
            Button {
                text: qsTr("Delete Anchor")
                enabled: AppSession.pathEditSelected >= 0
                         && AppSession.pathAnchorCount > 2
                onClicked: AppSession.pathDeleteSelectedAnchor()
            }
        }
    }

    // Crop / Transform commit chrome
    DisclosureGroup {
        groupId: "inspector.transform"
        title: qsTr("Transform and Crop")
        visible: root.isCropTool() || root.isTransformTool()
                 || AppSession.transformActive
                 || AppSession.cropPreviewActive
        // An uncommitted crop or transform is pending work;
        // its extent is what decides whether to expand.
        summary: {
            if (AppSession.cropPreviewActive)
                return qsTr("%1 × %2").arg(AppSession.cropPreviewW)
                                     .arg(AppSession.cropPreviewH)
            if (AppSession.transformActive)
                return qsTr("%1°").arg(Math.round(AppSession.transformRot))
            return qsTr("idle")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: root.isCropTool() || AppSession.cropPreviewActive
                      ? qsTr("Crop") : qsTr("Free Transform")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Button {
                    text: qsTr("Apply")
                    enabled: AppSession.hasDocument
                             && (AppSession.transformActive
                                 || AppSession.cropPreviewActive)
                    onClicked: {
                        if (AppSession.transformActive)
                            AppSession.commitTransform()
                        else if (AppSession.cropPreviewActive)
                            AppSession.commitCrop(
                                        AppSession.cropPreviewX,
                                        AppSession.cropPreviewY,
                                        AppSession.cropPreviewW,
                                        AppSession.cropPreviewH)
                    }
                }
                Button {
                    text: qsTr("Cancel")
                    Accessible.name: qsTr("Cancel")
                    enabled: AppSession.transformActive
                             || AppSession.cropPreviewActive
                    onClicked: {
                        if (AppSession.transformActive)
                            AppSession.cancelTransform()
                        else
                            AppSession.cancelCrop()
                    }
                }
            }
            ThemedCheckBox {
                visible: AppSession.transformActive || root.isTransformTool()
                text: qsTr("Constrain proportions")
                checked: AppSession.transformConstrain
                onToggled: AppSession.updateTransformDraft(
                               AppSession.transformTx,
                               AppSession.transformTy,
                               AppSession.transformSx,
                               AppSession.transformSy,
                               AppSession.transformRot,
                               checked)
            }
            Label {
                visible: AppSession.transformActive
                text: qsTr("Drag to move · handles scale · Enter apply · Esc cancel")
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
            RowLayout {
                visible: AppSession.transformActive
                Layout.fillWidth: true
                Label {
                    text: qsTr("Rotate")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                }
                Slider {
                    Layout.fillWidth: true
                    from: -180
                    to: 180
                    value: AppSession.transformRot
                    onMoved: AppSession.updateTransformDraft(
                                 AppSession.transformTx,
                                 AppSession.transformTy,
                                 AppSession.transformSx,
                                 AppSession.transformSy,
                                 value,
                                 AppSession.transformConstrain)
                }
            }
        }
    }

    // Adjustment layer params
    DisclosureGroup {
        groupId: "inspector.adjustment"
        title: qsTr("Adjustment")
        visible: AppSession.adjustmentKind === "brightness"
                 || AppSession.adjustmentKind === "levels"
                 || AppSession.adjustmentKind === "exposure"
        summary: {
            if (AppSession.adjustmentKind === "levels")
                return qsTr("%1–%2 γ%3")
                       .arg(Math.round(AppSession.adjustmentP0 * 255))
                       .arg(Math.round(AppSession.adjustmentP1 * 255))
                       .arg(AppSession.adjustmentP2.toFixed(2))
            if (AppSession.adjustmentKind === "exposure")
                return qsTr("%1 EV").arg(AppSession.adjustmentP0.toFixed(2))
            return qsTr("%1 / %2")
                   .arg(Math.round(AppSession.adjustmentP0 * 100))
                   .arg(Math.round(AppSession.adjustmentP1 * 100))
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: AppSession.adjustmentKind === "levels"
                      ? qsTr("Levels")
                      : (AppSession.adjustmentKind === "exposure"
                         ? qsTr("Exposure") : qsTr("Brightness/Contrast"))
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            RowLayout {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "brightness"
                Label {
                    text: qsTr("Brightness")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: Math.round(AppSession.adjustmentP0 * 100)
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "brightness"
                from: root.adjRange("brightness", "p0", 0)
                to: root.adjRange("brightness", "p0", 1)
                value: AppSession.adjustmentP0
                onMoved: AppSession.setAdjustmentParams(
                             value, AppSession.adjustmentP1, 0)
            }
            RowLayout {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "brightness"
                Label {
                    text: qsTr("Contrast")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: Math.round(AppSession.adjustmentP1 * 100)
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "brightness"
                from: root.adjRange("brightness", "p1", 0)
                to: root.adjRange("brightness", "p1", 1)
                value: AppSession.adjustmentP1
                onMoved: AppSession.setAdjustmentParams(
                             AppSession.adjustmentP0, value, 0)
            }
            RowLayout {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "levels"
                Label {
                    text: qsTr("Black")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: Math.round(AppSession.adjustmentP0 * 255)
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "levels"
                from: root.adjRange("levels", "p0", 0)
                to: root.adjRange("levels", "p0", 1)
                value: AppSession.adjustmentP0
                onMoved: AppSession.setAdjustmentParams(
                             value, AppSession.adjustmentP1, AppSession.adjustmentP2)
            }
            RowLayout {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "levels"
                Label {
                    text: qsTr("White")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: Math.round(AppSession.adjustmentP1 * 255)
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "levels"
                from: root.adjRange("levels", "p1", 0)
                to: root.adjRange("levels", "p1", 1)
                value: AppSession.adjustmentP1
                onMoved: AppSession.setAdjustmentParams(
                             AppSession.adjustmentP0, value, AppSession.adjustmentP2)
            }
            RowLayout {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "levels"
                Label {
                    text: qsTr("Gamma")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: AppSession.adjustmentP2.toFixed(2)
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "levels"
                from: root.adjRange("levels", "p2", 0)
                to: root.adjRange("levels", "p2", 1)
                value: AppSession.adjustmentP2
                onMoved: AppSession.setAdjustmentParams(
                             AppSession.adjustmentP0, AppSession.adjustmentP1, value)
            }
            RowLayout {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "exposure"
                Label {
                    text: qsTr("Stops")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: AppSession.adjustmentP0.toFixed(2)
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "exposure"
                from: root.adjRange("exposure", "p0", 0)
                to: root.adjRange("exposure", "p0", 1)
                value: AppSession.adjustmentP0
                onMoved: AppSession.setAdjustmentParams(
                             value, AppSession.adjustmentP1, 0)
            }
            RowLayout {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "exposure"
                Label {
                    text: qsTr("Gamma")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: AppSession.adjustmentP1.toFixed(2)
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                Layout.fillWidth: true
                visible: AppSession.adjustmentKind === "exposure"
                from: root.adjRange("exposure", "p1", 0)
                to: root.adjRange("exposure", "p1", 1)
                value: AppSession.adjustmentP1
                onMoved: AppSession.setAdjustmentParams(
                             AppSession.adjustmentP0, value, 0)
            }
        }
    }

    // Gaussian blur effect
    DisclosureGroup {
        groupId: "inspector.effects"
        title: qsTr("Effects")
        visible: AppSession.hasGaussianBlur
                 || AppSession.effectsJoined.length > 0
        summary: {
            var n = AppSession.effectsJoined.length > 0
                    ? AppSession.effectsJoined.split("|").length : 0
            return n === 1 ? qsTr("1 effect") : qsTr("%1 effects").arg(n)
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Effects")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            Repeater {
                id: effectsRepeater
                model: AppSession.effectsJoined.length > 0
                       ? AppSession.effectsJoined.split("|") : []
                delegate: RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    readonly property var parts: modelData.split(":")
                    readonly property string effectId: parts.length > 0 ? parts[0] : ""
                    readonly property string effectName: parts.length > 1 ? parts[1] : ""
                    readonly property bool effectOn: parts.length > 2 && parts[2] === "1"
                    ThemedCheckBox {
                        checked: effectOn
                        text: effectName
                        Layout.fillWidth: true
                        onToggled: AppSession.setActiveEffectEnabled(
                                       Number(effectId), checked)
                    }
                    ToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        icon.source: root.iconUrl("caret-up")
                        icon.color: enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                        icon.width: 12
                        icon.height: 12
                        contentItem: ThemedIcon {
                            anchors.centerIn: parent
                            source: parent.icon.source
                            size: parent.icon.height
                            color: parent.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                        }
                        enabled: index > 0
                        onClicked: AppSession.reorderActiveEffect(
                                       Number(effectId), index - 1)
                        Accessible.name: qsTr("Move effect up")
                        ToolTip.visible: hovered
                        ToolTip.text: Accessible.name
                    }
                    ToolButton {
                        implicitWidth: 22
                        implicitHeight: 22
                        padding: 0
                        icon.source: root.iconUrl("caret-down")
                        icon.color: enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                        icon.width: 12
                        icon.height: 12
                        contentItem: ThemedIcon {
                            anchors.centerIn: parent
                            source: parent.icon.source
                            size: parent.icon.height
                            color: parent.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                        }
                        enabled: index < effectsRepeater.count - 1
                        onClicked: AppSession.reorderActiveEffect(
                                       Number(effectId), index + 1)
                        Accessible.name: qsTr("Move effect down")
                        ToolTip.visible: hovered
                        ToolTip.text: Accessible.name
                    }
                }
            }
            Label {
                visible: AppSession.hasGaussianBlur
                text: qsTr("Gaussian Blur radius")
                color: Theme.colorOnSurfaceVariant
                font.pixelSize: Theme.fontLabelSm
            }
            RowLayout {
                visible: AppSession.hasGaussianBlur
                Layout.fillWidth: true
                Label {
                    text: qsTr("Radius")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                Label {
                    text: AppSession.gaussianRadius.toFixed(1) + " px"
                    color: Theme.primary
                    font.pixelSize: Theme.fontMono
                    font.family: "Noto Sans Mono"
                }
            }
            Slider {
                visible: AppSession.hasGaussianBlur
                Layout.fillWidth: true
                from: 0
                to: 64
                value: AppSession.gaussianRadius
                onMoved: AppSession.setGaussianRadius(value)
            }
        }
    }


    DisclosureGroup {
        groupId: "inspector.color"
        title: qsTr("Color Management")
        visible: AppSession.hasDocument
        summary: AppSession.softProofActive
                 ? AppSession.softProofProfile : qsTr("Soft-proof off")

        ColumnLayout {
            spacing: Theme.spaceXs
                Label {
                    text: AppSession.softProofActive
                          ? qsTr("Soft-proof: %1").arg(AppSession.softProofProfile)
                          : qsTr("Soft-proof: Off")
                    color: Theme.colorOnSurfaceVariant
                    font.pixelSize: Theme.fontBodySm
                }
                Label {
                    text: AppSession.hasEmbeddedIcc
                          ? qsTr("ICC: embedded")
                          : qsTr("ICC: tag-only")
                    color: Theme.colorOnSurfaceVariant
                    font.pixelSize: Theme.fontBodySm
                }
                Label {
                    text: qsTr("Display: %1").arg(AppSession.displayProfileName)
                    color: Theme.colorOnSurfaceVariant
                    font.pixelSize: Theme.fontBodySm
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    Button {
                        text: qsTr("Use display profile")
                        flat: true
                        enabled: AppSession.hasDocument && !AppSession.ioBusy
                        onClicked: AppSession.useDisplaySoftProof()
                        Accessible.name: qsTr("Soft-proof with display ICC")
                    }
                    Button {
                        text: qsTr("Embed ICC…")
                        flat: true
                        enabled: !AppSession.ioBusy
                        onClicked: root.embedIccRequested()
                    }
                    Button {
                        text: qsTr("Clear ICC")
                        flat: true
                        enabled: AppSession.hasEmbeddedIcc && !AppSession.ioBusy
                        onClicked: AppSession.clearEmbeddedIcc()
                    }
                }
        }
    }

    // Level 4 (specialized): the status bar carries the live
    // readout; this group is the inspectable detail.
    DisclosureGroup {
        groupId: "inspector.diagnostics"
        title: qsTr("Diagnostics")
        visible: AppSession.hasDocument
        // GPU-lost reaches the header via inspectorBadgesJson.
        summary: AppSession.compositeMs > 0
                 ? qsTr("%1 ms").arg(AppSession.compositeMs.toFixed(2))
                 : qsTr("no GPU timing")

        ColumnLayout {
            spacing: Theme.spaceXs

            // GPU-timestamped, one composite behind. Zero means
            // no sample yet, or an adapter without timestamp queries.
            Label {
                Layout.fillWidth: true
                text: AppSession.compositeMs > 0
                      ? qsTr("Composite: %1 ms (GPU)")
                        .arg(AppSession.compositeMs.toFixed(2))
                      : qsTr("Composite: no GPU timing")
                color: AppSession.compositeMs > 0 && AppSession.compositeMs < 2.0
                       ? Theme.success : Theme.colorOnSurfaceVariant
                font.pixelSize: Theme.fontBodySm
                font.family: "Noto Sans Mono"
            }
            Label {
                Layout.fillWidth: true
                text: qsTr("Frame rate: %1 FPS").arg(Math.round(AppSession.fps))
                color: AppSession.fps >= 60
                       ? Theme.success : Theme.colorOnSurfaceVariant
                font.pixelSize: Theme.fontBodySm
                font.family: "Noto Sans Mono"
            }
            Label {
                Layout.fillWidth: true
                text: qsTr("Stroke input→submit: %1 ms")
                      .arg(AppSession.strokeLatencyMs.toFixed(2))
                color: AppSession.strokeLatencyMs > 0
                       && AppSession.strokeLatencyMs < 8.0
                       ? Theme.success : Theme.colorOnSurfaceVariant
                font.pixelSize: Theme.fontBodySm
                font.family: "Noto Sans Mono"
            }
            Label {
                Layout.fillWidth: true
                text: qsTr("Cold boot: %1 ms").arg(AppSession.startupMs.toFixed(0))
                color: Theme.colorOnSurfaceVariant
                font.pixelSize: Theme.fontBodySm
                font.family: "Noto Sans Mono"
            }
            Label {
                Layout.fillWidth: true
                text: qsTr("Document: %1 × %2 px, %3 layers")
                      .arg(AppSession.docWidth)
                      .arg(AppSession.docHeight)
                      .arg(AppSession.layerCount)
                color: Theme.colorOnSurfaceVariant
                font.pixelSize: Theme.fontBodySm
                wrapMode: Text.Wrap
            }
            Label {
                Layout.fillWidth: true
                visible: AppSession.gpuLost
                text: qsTr("Graphics device lost — document authority preserved")
                color: Theme.error
                font.pixelSize: Theme.fontBodySm
                wrapMode: Text.Wrap
            }
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        visible: root.activeLayerHasMask
        RowLayout {
            Layout.fillWidth: true
            Label {
                text: AppSession.maskEditActive
                      ? qsTr("Mask · Editing") : qsTr("Mask")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
                Layout.fillWidth: true
            }
            ToolButton {
                implicitWidth: 28
                implicitHeight: 28
                icon.source: root.iconUrl("rectangle-dashed")
                icon.color: enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                icon.width: 16
                icon.height: 16
                contentItem: ThemedIcon {
                    anchors.centerIn: parent
                    source: parent.icon.source
                    size: parent.icon.height
                    color: parent.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
                }
                checkable: true
                checked: AppSession.maskEditActive
                onClicked: AppSession.setMaskEditTarget(checked)
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Edit layer mask")
            }
        }
        ThemedCheckBox {
            text: qsTr("Enable mask")
            checked: root.activeMaskEnabled
            onToggled: AppSession.setMaskEnabledOnActive(checked)
        }
        ThemedCheckBox {
            text: qsTr("Invert")
            checked: AppSession.maskInverted
            onToggled: AppSession.setMaskAttributesOnActive(
                           AppSession.maskDensity, AppSession.maskFeather,
                           checked, AppSession.maskLinked,
                           AppSession.maskContrast, AppSession.maskShift)
        }
        ThemedCheckBox {
            text: qsTr("Link mask")
            checked: AppSession.maskLinked
            onToggled: AppSession.setMaskAttributesOnActive(
                           AppSession.maskDensity, AppSession.maskFeather,
                           AppSession.maskInverted, checked,
                           AppSession.maskContrast, AppSession.maskShift)
        }
        Label {
            text: qsTr("Density %1%").arg(Math.round(AppSession.maskDensity * 100))
            color: Theme.colorOnSurfaceVariant
            font.pixelSize: Theme.fontLabelSm
        }
        Slider {
            Layout.fillWidth: true
            from: 0
            to: 1
            value: AppSession.maskDensity
            onMoved: AppSession.setMaskAttributesOnActive(
                         value, AppSession.maskFeather,
                         AppSession.maskInverted, AppSession.maskLinked,
                         AppSession.maskContrast, AppSession.maskShift)
        }
        // Feather is stored, clamped and undone, but no
        // renderer consumes it: unlike density, contrast and
        // shift it is a neighbourhood operation, and the
        // composite samples masks from an R8 array that the
        // RGBA-only separable blur cannot filter. Shown
        // disabled rather than removed — the value persists
        // in existing documents, and a control that moves
        // without changing a pixel is worse than one that
        // says it is unavailable.
        Label {
            text: qsTr("Feather %1 px — not yet applied")
                      .arg(AppSession.maskFeather.toFixed(1))
            color: Theme.colorOnSurfaceDisabled
            font.pixelSize: Theme.fontLabelSm
            ToolTip.visible: featherSlider.hovered
            ToolTip.text: qsTr("Mask feather is recorded but not yet rendered")
        }
        Slider {
            id: featherSlider
            Layout.fillWidth: true
            enabled: false
            from: 0
            to: 64
            value: AppSession.maskFeather
        }
        Label {
            text: qsTr("Contrast %1").arg(AppSession.maskContrast.toFixed(2))
            color: Theme.colorOnSurfaceVariant
            font.pixelSize: Theme.fontLabelSm
        }
        Slider {
            Layout.fillWidth: true
            from: -1
            to: 1
            value: AppSession.maskContrast
            onMoved: AppSession.setMaskAttributesOnActive(
                         AppSession.maskDensity, AppSession.maskFeather,
                         AppSession.maskInverted, AppSession.maskLinked,
                         value, AppSession.maskShift)
        }
        Label {
            text: qsTr("Shift %1").arg(AppSession.maskShift.toFixed(2))
            color: Theme.colorOnSurfaceVariant
            font.pixelSize: Theme.fontLabelSm
        }
        Slider {
            Layout.fillWidth: true
            from: -1
            to: 1
            value: AppSession.maskShift
            onMoved: AppSession.setMaskAttributesOnActive(
                         AppSession.maskDensity, AppSession.maskFeather,
                         AppSession.maskInverted, AppSession.maskLinked,
                         AppSession.maskContrast, value)
        }
        Button {
            text: qsTr("Apply Mask")
            onClicked: AppSession.invokeAction("action.layer.apply-mask")
        }
        Button {
            text: qsTr("Delete Mask")
            onClicked: AppSession.deleteMaskOnActive()
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        visible: AppSession.activeTool === "tool.fill"
                 || AppSession.activeTool === "tool.gradient"
                 || AppSession.activeTool === "tool.eyedropper"
        Label {
            text: AppSession.activeTool === "tool.gradient"
                  ? qsTr("Gradient (Linear)")
                  : (AppSession.activeTool === "tool.eyedropper"
                     ? qsTr("Eyedropper")
                     : qsTr("Paint Bucket"))
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
        }
        Label {
            text: AppSession.activeTool === "tool.gradient"
                  ? qsTr("Drag FG→BG. Respects selection.")
                  : (AppSession.activeTool === "tool.eyedropper"
                     ? qsTr("Click canvas to sample foreground.")
                     : qsTr("Click to fill with FG. Respects selection."))
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabelSm
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        visible: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
        Label {
            text: qsTr("Blend Mode")
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
        }
        Label {
            visible: AppSession.inspectorBlendMixed
            text: qsTr("Mixed")
            color: Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabelSm
            font.italic: true
            Accessible.name: qsTr("Blend mode mixed across selection")
        }
        ThemedComboBox {
            id: blendCombo
            Layout.fillWidth: true
            model: [
                { label: qsTr("Normal"), id: "normal" },
                { label: qsTr("Multiply"), id: "multiply" },
                { label: qsTr("Screen"), id: "screen" },
                { label: qsTr("Overlay"), id: "overlay" },
                { label: qsTr("Soft Light"), id: "soft_light" },
                { label: qsTr("Hard Light"), id: "hard_light" },
                { label: qsTr("Darken"), id: "darken" },
                { label: qsTr("Lighten"), id: "lighten" }
            ]
            textRole: "label"
            valueRole: "id"
            enabled: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
            opacity: AppSession.inspectorBlendMixed ? 0.85 : 1.0
            Component.onCompleted: root.syncBlendCombo()
            onActivated: AppSession.setActiveBlend(currentValue)
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        Label {
            text: qsTr("Foreground")
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            Rectangle {
                width: 28
                height: 28
                radius: Theme.radiusSm
                color: Qt.rgba(AppSession.brushR, AppSession.brushG, AppSession.brushB, 1)
                border.color: Theme.border
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Slider {
                    id: colorR
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: AppSession.brushR
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setForegroundRgb(value, colorG.value, colorB.value)
                }
                Slider {
                    id: colorG
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: AppSession.brushG
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setForegroundRgb(colorR.value, value, colorB.value)
                }
                Slider {
                    id: colorB
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: AppSession.brushB
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setForegroundRgb(colorR.value, colorG.value, value)
                }
            }
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        Label {
            text: qsTr("View")
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            Button {
                text: qsTr("Fit")
                Layout.fillWidth: true
                enabled: AppSession.hasDocument
                onClicked: AppSession.zoomToFit()
            }
            Button {
                text: qsTr("100%")
                Layout.fillWidth: true
                enabled: AppSession.hasDocument
                onClicked: AppSession.setZoom(1.0)
            }
        }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        RowLayout {
            Layout.fillWidth: true
            Label {
                text: qsTr("Layer Opacity")
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
                Layout.fillWidth: true
            }
            Label {
                text: AppSession.inspectorOpacityMixed
                      ? qsTr("Mixed")
                      : (Math.round(layerOpacitySlider.value * 100) + " %")
                color: AppSession.inspectorOpacityMixed
                       ? Theme.colorOnSurfaceMuted
                       : Theme.primary
                font.pixelSize: Theme.fontMono
                font.family: "Noto Sans Mono"
                font.italic: AppSession.inspectorOpacityMixed
                Accessible.name: AppSession.inspectorOpacityMixed
                                 ? qsTr("Opacity mixed across selection")
                                 : qsTr("Layer opacity percent %1")
                                       .arg(Math.round(layerOpacitySlider.value * 100))
            }
        }
        Slider {
            id: layerOpacitySlider
            Layout.fillWidth: true
            from: 0
            to: 1
            value: AppSession.activeOpacity
            enabled: AppSession.hasDocument && AppSession.activeLayerIndex >= 0
            opacity: AppSession.inspectorOpacityMixed ? 0.85 : 1.0
            onMoved: AppSession.setActiveOpacity(value)
        }
    }

    Label {
        text: root.gpuStatus
        color: Theme.colorOnSurfaceMuted
        font.pixelSize: Theme.fontLabelSm
        wrapMode: Text.WordWrap
        Layout.fillWidth: true
    }
}
