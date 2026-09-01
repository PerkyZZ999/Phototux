// Properties panel body — the right dock's contextual editor (handbook 01).
//
// Photoshop's Properties panel is a *contextual* one: it shows the settings
// for what is selected and nothing else. This one showed everything at once —
// a brush size above a text frame above a soft-proof profile — because each
// section carried its own visibility rule, and several of those rules compared
// `LayerKind::as_str` against a string literal written here. The panel was
// also carrying four control clusters that already had a home elsewhere in
// the shell, so a narrow dock spent its height on duplicates.
//
// What that means concretely, and what changed:
//
//   * Presence is decided by *subject* — document, raster, text, shape, fill,
//     adjustment, group — declared once per section in the engine's disclosure
//     descriptors and asked for through `sectionApplies`. Nothing here names a
//     layer kind. `properties_panel_draws_the_sections_the_engine_declares`
//     holds this file and that table in agreement.
//   * A section may add live conditions on top (a styles list with no styles
//     stays away), but never a second subject rule.
//   * Tool settings left. Brush size, hardness, texture and the selection
//     combine modes live in `ToolOptionsBar.qml`, which is where Photoshop
//     keeps them; the foreground colour lives in the Swatches panel, which
//     already has the wells, the hex field and the recents; Fit and 100% live
//     in the options bar's zoom field. Each was a second copy, and the panel
//     with the tightest height budget was the one paying for them.
//
// The seam stays narrow: five inbound values, one signal out, and a header
// component that owns the scope switch and the paint target.
//
// The `Flickable` stays behind in the shell: it carries the dock's `Layout.*`
// attachments and its `contentHeight` binding, which are the dock's business.

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

    // Derived host state the shell already computes once for several panels.
    required property bool activeLayerHasMask
    required property bool activeMaskEnabled

    /// Present GPU status line, owned by the canvas host.
    required property string gpuStatus

    /// Raised when the user asks to embed an ICC profile. The file dialog
    /// belongs to the shell, so the panel asks rather than opens.
    signal embedIccRequested

    /// `InspectorSubject::Document`. The one subject id written out here,
    /// because the document scope is a thing the *panel* offers rather than
    /// something the selection reports; `the_properties_panel_pins_a_subject_
    /// the_engine_declares` checks it against the engine's vocabulary.
    readonly property string documentSubject: "document"

    /// Subject the user has pinned, or empty to follow the selection.
    ///
    /// Presentation state, so it lives here rather than in the session: which
    /// half of the panel someone is reading is not a fact about the document,
    /// and pinning it in the engine would make it undoable.
    property string pinnedSubject: ""

    /// The subject every section below is resolved against.
    readonly property string subject: root.pinnedSubject.length > 0
                                      ? root.pinnedSubject
                                      : AppSession.inspectorSubject

    /// Section id → the subject ids that section describes, from the engine.
    readonly property var sectionSubjects: {
        var map = ({})
        try {
            var groups = JSON.parse(AppSession.disclosureGroupsJson || "[]")
            for (var i = 0; i < groups.length; ++i)
                map[groups[i].id] = groups[i].subjects
        } catch (e) {
            // Left empty on purpose. With no table, no section claims the
            // subject and the panel shows its empty state — which is visibly
            // wrong, where defaulting to "show everything" would look like
            // the old panel and hide the fault.
        }
        return map
    }

    /// Whether `groupId` describes what is currently selected.
    function sectionApplies(groupId) {
        var subjects = root.sectionSubjects[groupId]
        return subjects !== undefined && subjects.indexOf(root.subject) >= 0
    }

    /// Guides on the current document, as the host publishes them.
    readonly property var guides: {
        try {
            return JSON.parse(AppSession.guidesJson || "[]")
        } catch (e) {
            return []
        }
    }

    /// Align and distribute operations, as the engine declares them.
    readonly property var alignOps: {
        try {
            return JSON.parse(AppSession.alignOpsJson || "[]")
        } catch (e) {
            return []
        }
    }

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

    /// An icon-over-label button for the document's quick actions.
    ///
    /// Photoshop's document properties lead with a row of these — crop, trim,
    /// rotate, image size — because they are the handful of things anyone does
    /// to a whole document. PhotoTux offers the ones it actually has; nothing
    /// here is a button for a command that does not exist.
    component QuickAction: AbstractButton {
        id: quick
        required property string stem
        Layout.fillWidth: true
        implicitHeight: Theme.toolHit
        focusPolicy: Qt.StrongFocus
        Accessible.role: Accessible.Button
        Accessible.name: quick.text
        ToolTip.visible: quick.hovered
        ToolTip.text: quick.text
        background: Rectangle {
            radius: Theme.radiusSm
            color: quick.down
                   ? Theme.surfaceRaised
                   : (quick.hovered && quick.enabled ? Theme.surfaceContainerHigh
                                                     : Theme.surfaceContainer)
            border.color: quick.activeFocus ? Theme.focusRing : Theme.borderSubtle
            border.width: 1
            opacity: quick.enabled ? 1.0 : 0.55
        }
        contentItem: ColumnLayout {
            spacing: 1
            ThemedIcon {
                Layout.alignment: Qt.AlignHCenter
                source: root.iconUrl(quick.stem)
                size: Theme.iconMd
                color: quick.enabled ? Theme.iconOnSurfaceEffective
                                     : Theme.iconDisabledEffective
            }
            Label {
                Layout.fillWidth: true
                text: quick.text
                color: quick.enabled ? Theme.colorOnSurface
                                     : Theme.colorOnSurfaceDisabled
                font.pixelSize: Theme.fontLabelSm
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
            }
        }
    }

    InspectorContextHeader {
        Layout.fillWidth: true
        // A nested layout defaults to `fillHeight: true`, so in a dock taller
        // than the panel's content this absorbed the slack and floated the
        // scope tabs down the panel. Everything here is intrinsically sized;
        // the spacer at the foot is what takes the leftovers.
        Layout.fillHeight: false
        iconUrl: root.iconUrl
        subject: root.subject
        documentSubject: root.documentSubject
        hasMask: root.activeLayerHasMask
        onScopeRequested: function (pinned) { root.pinnedSubject = pinned }
    }

    // Picking a layer returns the panel to the layer scope, which is what
    // Photoshop does: selecting something means you want to see it. Without
    // this, pinning the document once would leave every later selection
    // showing canvas facts, and the panel would look broken rather than
    // pinned. Assigning a property is safe inside a host signal handler; only
    // calling back into a slot is not (handbook 32).
    Connections {
        target: AppSession
        function onActiveLayerIndexChanged() { root.pinnedSubject = "" }
        function onInspectorSubjectChanged() { root.pinnedSubject = "" }
    }

    // Nothing open: the panel says so rather than showing a stack of empty
    // groups, matching the Layers and History panels.
    PanelPlaceholder {
        Layout.fillWidth: true
        Layout.preferredHeight: 120
        visible: !AppSession.hasDocument
        iconKey: "frame-corners"
        iconUrl: root.iconUrl
        text: qsTr("No document open")
        hint: qsTr("Properties describe the layer you select.")
    }

    // ── Document scope ──────────────────────────────────────────────────

    DisclosureGroup {
        groupId: "inspector.document"
        title: qsTr("Document")
        visible: root.sectionApplies("inspector.document")
        summary: qsTr("%1 × %2").arg(AppSession.docWidth).arg(AppSession.docHeight)

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: Theme.spaceSm
                rowSpacing: 2

                component FactName: Label {
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                }
                component FactValue: Label {
                    Layout.fillWidth: true
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    font.family: "Noto Sans Mono"
                    elide: Text.ElideMiddle
                }

                FactName { text: qsTr("Canvas") }
                FactValue {
                    text: qsTr("%1 × %2 px").arg(AppSession.docWidth)
                                            .arg(AppSession.docHeight)
                }
                FactName { text: qsTr("Layers") }
                FactValue { text: AppSession.layerCount.toString() }
                FactName { text: qsTr("Selection") }
                FactValue {
                    text: AppSession.selectionActive
                          ? qsTr("%1 × %2 at %3, %4")
                            .arg(AppSession.selectionW).arg(AppSession.selectionH)
                            .arg(AppSession.selectionX).arg(AppSession.selectionY)
                          : qsTr("none")
                    color: AppSession.selectionActive
                           ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
                }
                FactName { text: qsTr("File") }
                FactValue {
                    text: AppSession.documentPath.length > 0
                          ? AppSession.documentPath : qsTr("not saved yet")
                    color: AppSession.documentPath.length > 0
                           ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
                    font.family: "Noto Sans Mono"
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 3
                columnSpacing: Theme.spaceXs
                rowSpacing: Theme.spaceXs

                QuickAction {
                    text: qsTr("Crop")
                    stem: "crop"
                    enabled: AppSession.hasDocument
                    onClicked: AppSession.setActiveTool("tool.crop")
                }
                QuickAction {
                    text: qsTr("Rotate")
                    stem: "arrows-clockwise"
                    enabled: AppSession.hasDocument
                    onClicked: root.runAction("action.image.rotate-90")
                }
                QuickAction {
                    text: qsTr("Fit")
                    stem: "arrows-out"
                    enabled: AppSession.hasDocument
                    onClicked: AppSession.zoomToFit()
                }
                QuickAction {
                    text: qsTr("Flip H")
                    stem: "flip-horizontal"
                    enabled: AppSession.hasDocument
                    onClicked: root.runAction("action.image.flip-h")
                }
                QuickAction {
                    text: qsTr("Flip V")
                    stem: "flip-vertical"
                    enabled: AppSession.hasDocument
                    onClicked: root.runAction("action.image.flip-v")
                }
                QuickAction {
                    text: qsTr("100%")
                    stem: "magnifying-glass"
                    enabled: AppSession.hasDocument
                    onClicked: AppSession.setZoom(1.0)
                }
            }
        }
    }

    DisclosureGroup {
        groupId: "inspector.guides"
        title: qsTr("Guides and Grid")
        visible: root.sectionApplies("inspector.guides")
        summary: root.guides.length === 1
                 ? qsTr("1 guide")
                 : qsTr("%1 guides").arg(root.guides.length)

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs

            ThemedCheckBox {
                text: qsTr("Rulers")
                checked: AppSession.prefShowRulers
                onToggled: root.runAction("action.view.toggle-rulers")
            }
            ThemedCheckBox {
                text: qsTr("Grid")
                checked: AppSession.prefShowGrid
                onToggled: root.runAction("action.view.toggle-grid")
            }
            ThemedCheckBox {
                text: qsTr("Guides")
                checked: AppSession.prefShowGuides
                onToggled: root.runAction("action.view.toggle-guides")
            }
            ThemedCheckBox {
                text: qsTr("Snap to guides and grid")
                checked: AppSession.prefSnap
                onToggled: root.runAction("action.view.toggle-snap")
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                Label {
                    text: qsTr("Grid spacing")
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    Layout.fillWidth: true
                }
                ThemedSpinBox {
                    id: gridSpacingSpin
                    from: 2
                    to: 512
                    value: Math.round(AppSession.gridSpacing)
                    enabled: AppSession.hasDocument
                    onValueModified: AppSession.setGridSpacing(value)
                    Connections {
                        target: AppSession
                        function onGridSpacingChanged() {
                            gridSpacingSpin.value = Math.round(AppSession.gridSpacing)
                        }
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Button {
                    text: qsTr("Add H")
                    Layout.fillWidth: true
                    enabled: AppSession.hasDocument
                    Accessible.name: qsTr("Add horizontal guide")
                    onClicked: root.runAction("action.view.guide-h")
                }
                Button {
                    text: qsTr("Add V")
                    Layout.fillWidth: true
                    enabled: AppSession.hasDocument
                    Accessible.name: qsTr("Add vertical guide")
                    onClicked: root.runAction("action.view.guide-v")
                }
                Button {
                    text: qsTr("Clear")
                    Layout.fillWidth: true
                    enabled: root.guides.length > 0
                    Accessible.name: qsTr("Clear all guides")
                    onClicked: root.runAction("action.view.clear-guides")
                }
            }
        }
    }

    DisclosureGroup {
        groupId: "inspector.color"
        title: qsTr("Color Management")
        visible: root.sectionApplies("inspector.color")
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
        visible: root.sectionApplies("inspector.diagnostics")
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
            // Was a bare label at the very bottom of the panel, below
            // every group, where it read as a caption for whatever happened
            // to be above it. It is a diagnostic, and this is where the
            // diagnostics are.
            Label {
                Layout.fillWidth: true
                text: root.gpuStatus
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                wrapMode: Text.WordWrap
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

    // ── Layer scope ─────────────────────────────────────────────────────

    // Character / text layer chrome
    DisclosureGroup {
        groupId: "inspector.text"
        title: qsTr("Character")
        visible: root.sectionApplies("inspector.text")
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

    // Fill layer chrome
    DisclosureGroup {
        groupId: "inspector.fill"
        title: qsTr("Fill")
        visible: root.sectionApplies("inspector.fill")
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

    // Adjustment layer params
    //
    // Built from the engine's slot table rather than a hand-written slider
    // pair per kind: the panel used to name brightness, levels and exposure
    // explicitly, so the four other adjustment kinds a user could create had
    // no editor at all.
    DisclosureGroup {
        groupId: "inspector.adjustment"
        title: qsTr("Adjustment")
        visible: root.sectionApplies("inspector.adjustment")
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

    // Path edit chrome
    DisclosureGroup {
        groupId: "inspector.path"
        title: qsTr("Path")
        visible: root.sectionApplies("inspector.path")
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
        visible: root.sectionApplies("inspector.transform")
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

            // Photoshop's Properties offers the transform as something to
            // *start*, not only as a mode you must already be in. The
            // coordinates below do not exist until a session does, and the
            // group used to appear only once the tool was picked — so the
            // panel described the transform to the one user who had already
            // found it.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                visible: !AppSession.transformActive
                         && !AppSession.cropPreviewActive
                Button {
                    text: qsTr("Free Transform")
                    Layout.fillWidth: true
                    enabled: AppSession.hasDocument
                    Accessible.name: text
                    onClicked: AppSession.setActiveTool("tool.transform")
                }
                Button {
                    text: qsTr("Crop")
                    Layout.fillWidth: true
                    enabled: AppSession.hasDocument
                    Accessible.name: text
                    onClicked: AppSession.setActiveTool("tool.crop")
                }
            }
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

    // Align and distribute.
    //
    // Also in the Move tool's options bar, which is where Photoshop puts them
    // *and* where it repeats them: the options bar reaches them while moving
    // things, and Properties reaches them whatever tool is in hand. The two
    // read the same descriptor table, so neither can drift from the engine.
    DisclosureGroup {
        groupId: "inspector.align"
        title: qsTr("Align and Distribute")
        visible: root.sectionApplies("inspector.align")
        summary: AppSession.selectedLayerCount > 1
                 ? qsTr("%1 selected").arg(AppSession.selectedLayerCount)
                 : qsTr("to canvas")

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs

            component AlignRun: ColumnLayout {
                id: run
                required property bool distribute
                required property string caption
                Layout.fillWidth: true
                spacing: 2
                Label {
                    text: run.caption
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    Repeater {
                        model: root.alignOps.filter(op => op.distribute === run.distribute)
                        delegate: ToolButton {
                            id: opButton
                            required property var modelData
                            // Distribution needs a third object to have
                            // anything to space out; aligning one layer works
                            // because it aligns to the canvas.
                            readonly property bool available:
                                AppSession.hasDocument
                                && AppSession.layerCount >= opButton.modelData.minTargets
                            implicitWidth: Theme.controlHeight
                            implicitHeight: Theme.controlHeight
                            padding: 0
                            enabled: opButton.available
                            onClicked: AppSession.alignLayers(opButton.modelData.id)
                            ToolTip.visible: opButton.hovered
                            ToolTip.text: opButton.available
                                          ? opButton.modelData.label
                                          : qsTr("%1 — needs %2 layers")
                                            .arg(opButton.modelData.label)
                                            .arg(opButton.modelData.minTargets)
                            Accessible.name: opButton.modelData.label
                            contentItem: ThemedIcon {
                                anchors.centerIn: parent
                                source: root.iconUrl(opButton.modelData.icon)
                                size: Theme.iconMd
                                color: opButton.enabled ? Theme.iconOnSurfaceEffective
                                                        : Theme.iconDisabledEffective
                            }
                            background: Rectangle {
                                radius: Theme.radiusSm
                                color: opButton.hovered && opButton.enabled
                                       ? Theme.surfaceContainerHigh : "transparent"
                                border.color: Theme.borderSubtle
                                border.width: 1
                            }
                        }
                    }
                }
            }

            AlignRun { distribute: false; caption: qsTr("Align") }
            AlignRun { distribute: true; caption: qsTr("Distribute") }

            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                text: AppSession.selectedLayerCount > 1
                      ? qsTr("Aligns the %1 selected layers to each other.")
                        .arg(AppSession.selectedLayerCount)
                      : qsTr("Aligns the active layer to the canvas. Select more layers to align them to each other.")
            }
        }
    }

    // Masks.
    //
    // Was a bare column at the foot of the panel, below every group and with
    // no header, so the four attribute sliders read as loose controls
    // belonging to whatever was above them. Photoshop gives the mask its own
    // Properties section, and so does this — with the empty state saying how
    // to get one, rather than the section simply not being there.
    DisclosureGroup {
        groupId: "inspector.mask"
        title: qsTr("Masks")
        visible: root.sectionApplies("inspector.mask")
        summary: root.activeLayerHasMask
                 ? qsTr("%1%").arg(Math.round(AppSession.maskDensity * 100))
                 : qsTr("none")

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                visible: !root.activeLayerHasMask
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: qsTr("This layer has no mask. A mask hides parts of the layer without deleting them.")
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    Button {
                        text: qsTr("Add Mask")
                        Layout.fillWidth: true
                        enabled: AppSession.hasDocument
                        Accessible.name: qsTr("Add a layer mask")
                        onClicked: root.runAction("action.layer.add-mask")
                    }
                    Button {
                        text: qsTr("Add Vector Mask")
                        Layout.fillWidth: true
                        enabled: AppSession.hasDocument
                        Accessible.name: qsTr("Add a vector mask")
                        onClicked: root.runAction("action.layer.add-vector-mask")
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                visible: root.activeLayerHasMask
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
        visible: root.sectionApplies("inspector.styles")
                 && root.layerStyles.length > 0
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
        visible: root.sectionApplies("inspector.blend-if")
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
        visible: root.sectionApplies("inspector.effects")
                 && (AppSession.hasGaussianBlur
                     || AppSession.effectsJoined.length > 0)
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


    // Takes whatever height the dock has over and above the sections, so the
    // sections stay packed at the top instead of sharing it out between them.
    Item {
        Layout.fillWidth: true
        Layout.fillHeight: true
    }
}
