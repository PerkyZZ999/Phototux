// Properties panel body — the right dock's per-layer editor (handbook 01).
//
// This lived inline in `Main.qml`, where it was the single largest thing in the
// file: about 1,600 lines between the dock's `Flickable` and the Navigator
// header, so that reading either neighbour meant scrolling past the whole
// adjustment stack. Extracting it does not change what the panel does — the
// body below is the same tree — but it makes the panel's dependencies
// enumerable, which they were not while every id in `Main.qml` was in scope.
//
// The seam is deliberately narrow: ten inbound values, one signal out, and
// two functions the shell calls to push host state into controls that hold
// their own editing state. The properties are named exactly as they are on
// `Main.qml`'s root so the body's `root.*` references resolve
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

    /// Editor slots for the active adjustment kind, as the engine declares them.
    readonly property var adjustmentSlots: {
        try {
            var all = JSON.parse(AppSession.adjustmentRangesJson || "{}")
            var slots = all[AppSession.adjustmentKind]
            return slots ? slots : []
        } catch (e) {
            return []
        }
    }

    /// Display name of the active adjustment kind.
    readonly property string adjustmentLabel: {
        try {
            var labels = JSON.parse(AppSession.adjustmentLabelsJson || "{}")
            var label = labels[AppSession.adjustmentKind]
            return label ? label : qsTr("Adjustment")
        } catch (e) {
            return qsTr("Adjustment")
        }
    }

    /// Current values of the active adjustment's editor slots.
    readonly property var adjustmentValues: {
        try {
            return JSON.parse(AppSession.adjustmentSlotsJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Current value of editor slot `index`.
    function adjustmentSlotValue(index) {
        var v = root.adjustmentValues[index]
        return v === undefined ? 0 : v
    }

    /// The active layer's styles, with the descriptors to edit them.
    readonly property var layerStyles: {
        try {
            return JSON.parse(AppSession.layerStylesJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// The active layer's blend ranges, on the 0–255 scale the sliders show.
    readonly property var blendIf: {
        try {
            return JSON.parse(AppSession.blendIfJson || "{}")
        } catch (e) {
            return {}
        }
    }

    readonly property var blendIfChannels: {
        try {
            return JSON.parse(AppSession.blendIfChannelsJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Display name of the gradient shape the tool will sweep.
    readonly property string gradientKindLabel: {
        try {
            var kinds = JSON.parse(AppSession.gradientKindsJson || "[]")
            for (var i = 0; i < kinds.length; ++i) {
                if (kinds[i].id === AppSession.gradientKind)
                    return kinds[i].label
            }
        } catch (e) {
            // fall through
        }
        return qsTr("Linear")
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
                    Accessible.name: qsTr("Brush size, %1 pixels").arg(Math.round(brushSlider.value))
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
                    Accessible.name: qsTr("Brush hardness, %1 percent").arg(Math.round(hardnessSlider.value * 100))
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
                    implicitWidth: 22
                    implicitHeight: 22
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
                    Accessible.name: qsTr("Rotate, %1 degrees").arg(Math.round(AppSession.transformRot))
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
    //
    // Built from the engine's slot table rather than a hand-written slider
    // pair per kind: the panel used to name brightness, levels and exposure
    // explicitly, so the four other adjustment kinds a user could create had
    // no editor at all.
    DisclosureGroup {
        groupId: "inspector.adjustment"
        title: qsTr("Adjustment")
        visible: root.adjustmentSlots.length > 0
                 || AppSession.adjustmentKind.length > 0
        summary: {
            var slots = root.adjustmentSlots
            if (slots.length === 0)
                return root.adjustmentLabel
            var shown = []
            for (var i = 0; i < slots.length && i < 2; i++)
                shown.push(root.adjustmentSlotValue(i).toFixed(2))
            return shown.join(" / ")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: root.adjustmentLabel
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            Label {
                text: qsTr("No parameters")
                visible: root.adjustmentSlots.length === 0
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                font.italic: true
            }
            Repeater {
                model: root.adjustmentSlots
                delegate: ColumnLayout {
                    id: slotEditor
                    required property var modelData
                    required property int index

                    Layout.fillWidth: true
                    spacing: 2

                    RowLayout {
                        Layout.fillWidth: true
                        Label {
                            text: slotEditor.modelData.label
                            color: Theme.colorOnSurface
                            font.pixelSize: Theme.fontBodySm
                            Layout.fillWidth: true
                        }
                        Label {
                            text: root.adjustmentSlotValue(slotEditor.index).toFixed(2)
                            color: Theme.primary
                            font.pixelSize: Theme.fontMono
                            font.family: "Noto Sans Mono"
                        }
                    }
                    Slider {
                        Layout.fillWidth: true
                        from: slotEditor.modelData.min
                        to: slotEditor.modelData.max
                        value: root.adjustmentSlotValue(slotEditor.index)
                        Accessible.name: qsTr("%1 for %2")
                                         .arg(slotEditor.modelData.label)
                                         .arg(root.adjustmentLabel)
                        onMoved: AppSession.setAdjustmentSlot(slotEditor.index, value)
                    }
                }
            }
        }
    }


    // Layer styles
    //
    // Built from the engine's descriptors: each style declares its own scalar
    // slots and colours, so the panel names no style kind and a new one gets
    // an editor on arrival. Before this the styles could be added from the
    // menu and never edited — they rendered at their defaults for good.
    DisclosureGroup {
        groupId: "inspector.styles"
        title: qsTr("Layer Styles")
        visible: root.layerStyles.length > 0
        summary: root.layerStyles.length === 1
                 ? qsTr("1 style")
                 : qsTr("%1 styles").arg(root.layerStyles.length)

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm

            Repeater {
                model: root.layerStyles
                delegate: ColumnLayout {
                    id: styleEditor
                    required property var modelData

                    Layout.fillWidth: true
                    spacing: 2

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spaceXs
                        ThemedCheckBox {
                            checked: styleEditor.modelData.enabled
                            text: styleEditor.modelData.label
                            Layout.fillWidth: true
                            onToggled: AppSession.setLayerStyleEnabled(
                                           styleEditor.modelData.index, checked)
                        }
                        ChromeIconToolButton {
                            icon.source: root.iconUrl("trash")
                            Accessible.name: qsTr("Remove %1")
                                             .arg(styleEditor.modelData.label)
                            onClicked: AppSession.removeLayerStyle(
                                           styleEditor.modelData.index)
                        }
                    }

                    Repeater {
                        model: styleEditor.modelData.editor.slots
                        delegate: RowLayout {
                            id: styleSlot
                            required property var modelData
                            required property int index

                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            Label {
                                text: styleSlot.modelData.label
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                                Layout.preferredWidth: 64
                            }
                            Slider {
                                Layout.fillWidth: true
                                from: styleSlot.modelData.min
                                to: styleSlot.modelData.max
                                value: styleEditor.modelData.slots[styleSlot.index]
                                enabled: styleEditor.modelData.enabled
                                Accessible.name: qsTr("%1 for %2")
                                                 .arg(styleSlot.modelData.label)
                                                 .arg(styleEditor.modelData.label)
                                onMoved: AppSession.setLayerStyleSlot(
                                             styleEditor.modelData.index,
                                             styleSlot.index, value)
                            }
                            Label {
                                text: styleEditor.modelData.slots[styleSlot.index].toFixed(1)
                                color: Theme.primary
                                font.pixelSize: Theme.fontMono
                                font.family: "Noto Sans Mono"
                                Layout.preferredWidth: 34
                                horizontalAlignment: Text.AlignRight
                            }
                        }
                    }

                    Repeater {
                        model: styleEditor.modelData.editor.colors
                        delegate: RowLayout {
                            id: styleColor
                            required property var modelData
                            required property int index

                            readonly property var rgba:
                                styleEditor.modelData.colors[styleColor.index]

                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            Label {
                                text: styleColor.modelData
                                color: Theme.colorOnSurfaceMuted
                                font.pixelSize: Theme.fontLabelSm
                                Layout.preferredWidth: 64
                            }
                            Rectangle {
                                implicitWidth: 22
                                implicitHeight: 22
                                radius: Theme.radiusSm
                                border.color: Theme.border
                                color: Qt.rgba(styleColor.rgba[0],
                                               styleColor.rgba[1],
                                               styleColor.rgba[2], 1)
                                Accessible.role: Accessible.ColorChooser
                                Accessible.name: qsTr("%1 colour for %2")
                                                 .arg(styleColor.modelData)
                                                 .arg(styleEditor.modelData.label)
                            }
                            // Three channels rather than a picker dialog: the
                            // panel is dense and the shell has no colour dialog
                            // of its own, so this matches the Foreground swatch
                            // a few groups down.
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Repeater {
                                    model: 3
                                    delegate: Slider {
                                        required property int index
                                        Layout.fillWidth: true
                                        implicitHeight: 16
                                        from: 0
                                        to: 1
                                        value: styleColor.rgba[index]
                                        enabled: styleEditor.modelData.enabled
                                        Accessible.name: ["Red", "Green", "Blue"][index]
                                        onMoved: {
                                            var c = styleColor.rgba.slice()
                                            c[index] = value
                                            AppSession.setLayerStyleColor(
                                                styleEditor.modelData.index,
                                                styleColor.index, c[0], c[1], c[2])
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Blend If — hide the layer where it, or what is under it, falls outside a
    // channel range. Collapsed by default: eight handles would dominate the
    // panel, and this is not the first control anyone reaches for. The two
    // ranges are built from one component because they differ only in which
    // pixels they read.
    DisclosureGroup {
        id: blendIfGroup
        groupId: "inspector.blend-if"
        title: qsTr("Blend If")
        visible: AppSession.hasDocument
        summary: root.blendIf.active ? qsTr("Active") : qsTr("Off")

        component BlendRangeEditor: ColumnLayout {
            id: rangeEditor
            /// 0 = this layer, 1 = the underlying composite.
            required property int side
            required property string caption
            required property var stops

            Layout.fillWidth: true
            spacing: 2

            Label {
                text: rangeEditor.caption
                color: Theme.colorOnSurface
                font.pixelSize: Theme.fontBodySm
            }
            Repeater {
                model: root.blendIf.labels || []
                delegate: RowLayout {
                    id: stopRow
                    required property string modelData
                    required property int index

                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    Label {
                        text: stopRow.modelData
                        color: Theme.colorOnSurfaceMuted
                        font.pixelSize: Theme.fontLabelSm
                        Layout.preferredWidth: 74
                    }
                    Slider {
                        Layout.fillWidth: true
                        from: 0
                        to: 255
                        stepSize: 1
                        value: rangeEditor.stops[stopRow.index]
                        Accessible.name: qsTr("%1, %2")
                                         .arg(rangeEditor.caption)
                                         .arg(stopRow.modelData)
                        onMoved: AppSession.setBlendIfStop(
                                     rangeEditor.side, stopRow.index, value)
                    }
                    Label {
                        text: Math.round(rangeEditor.stops[stopRow.index])
                        color: Theme.primary
                        font.pixelSize: Theme.fontMono
                        font.family: "Noto Sans Mono"
                        Layout.preferredWidth: 28
                        horizontalAlignment: Text.AlignRight
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Label {
                    text: qsTr("Channel")
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                    Layout.preferredWidth: 74
                }
                ThemedComboBox {
                    Layout.fillWidth: true
                    model: root.blendIfChannels
                    textRole: "label"
                    valueRole: "id"
                    currentIndex: Math.max(0, indexOfValue(root.blendIf.channel))
                    Accessible.name: qsTr("Blend If channel")
                    onActivated: AppSession.setBlendIfChannel(currentValue)
                }
                ChromeIconToolButton {
                    icon.source: root.iconUrl("arrow-counter-clockwise")
                    enabled: root.blendIf.active === true
                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Reset blend ranges")
                    Accessible.name: qsTr("Reset blend ranges")
                    onClicked: AppSession.resetBlendIf()
                }
            }

            BlendRangeEditor {
                side: 0
                caption: qsTr("This Layer")
                stops: root.blendIf.thisLayer || [0, 0, 255, 255]
            }
            BlendRangeEditor {
                side: 1
                caption: qsTr("Underlying Layer")
                stops: root.blendIf.underlying || [0, 0, 255, 255]
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
                Accessible.name: qsTr("Gaussian blur radius, %1 pixels").arg(AppSession.gaussianRadius.toFixed(1))
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
            Accessible.name: qsTr("Mask density, %1 percent").arg(Math.round(AppSession.maskDensity * 100))
            onMoved: AppSession.setMaskAttributesOnActive(
                         value, AppSession.maskFeather,
                         AppSession.maskInverted, AppSession.maskLinked,
                         AppSession.maskContrast, AppSession.maskShift)
        }
        // Feather softens the mask using each texel's neighbours, so unlike
        // density, contrast and shift it cannot be applied as the composite
        // samples: the mask is blurred before it is packed. The control was
        // disabled while the separable blur was RGBA-only and could not filter
        // the R8 mask array.
        Label {
            text: qsTr("Feather %1 px").arg(AppSession.maskFeather.toFixed(1))
            color: Theme.colorOnSurfaceVariant
            font.pixelSize: Theme.fontLabelSm
        }
        Slider {
            Layout.fillWidth: true
            from: 0
            to: 64
            value: AppSession.maskFeather
            Accessible.name: qsTr("Mask feather, %1 pixels").arg(AppSession.maskFeather.toFixed(1))
            onMoved: AppSession.setMaskAttributesOnActive(
                         AppSession.maskDensity, value,
                         AppSession.maskInverted, AppSession.maskLinked,
                         AppSession.maskContrast, AppSession.maskShift)
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
            Accessible.name: qsTr("Mask contrast, %1").arg(AppSession.maskContrast.toFixed(2))
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
            Accessible.name: qsTr("Mask shift, %1").arg(AppSession.maskShift.toFixed(2))
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
            // Names the shape actually selected. It said "Linear"
            // unconditionally, which was true only while linear was the only
            // shape there was.
            text: AppSession.activeTool === "tool.gradient"
                  ? qsTr("Gradient (%1)").arg(root.gradientKindLabel)
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
        Label {
            text: qsTr("Foreground")
            color: Theme.colorOnSurface
            font.pixelSize: Theme.fontBodySm
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            Rectangle {
                implicitWidth: 28
                implicitHeight: 28
                radius: Theme.radiusSm
                color: Qt.rgba(AppSession.brushR, AppSession.brushG, AppSession.brushB, 1)
                border.color: Theme.border
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Slider {
                    id: colorR
                    Accessible.name: qsTr("Foreground red, %1 percent")
                                     .arg(Math.round(colorR.value * 100))
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: AppSession.brushR
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setForegroundRgb(value, colorG.value, colorB.value)
                }
                Slider {
                    id: colorG
                    Accessible.name: qsTr("Foreground green, %1 percent")
                                     .arg(Math.round(colorG.value * 100))
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: AppSession.brushG
                    enabled: AppSession.hasDocument
                    onMoved: AppSession.setForegroundRgb(colorR.value, value, colorB.value)
                }
                Slider {
                    id: colorB
                    Accessible.name: qsTr("Foreground blue, %1 percent")
                                     .arg(Math.round(colorB.value * 100))
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


    Label {
        text: root.gpuStatus
        color: Theme.colorOnSurfaceMuted
        font.pixelSize: Theme.fontLabelSm
        wrapMode: Text.WordWrap
        Layout.fillWidth: true
    }
}
