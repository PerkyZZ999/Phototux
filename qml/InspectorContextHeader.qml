import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// The Properties panel's context strip: what is being edited, and what a
/// paint stroke would land on.
///
/// This replaces a block headed "Edit target" that carried a four-part summary
/// line ("raster · Layer pixels · object: Layer 1 · no pixel selection") above
/// two buttons labelled "Layer pixels" and "Layer mask". Three problems, all
/// the same problem: it named a concept — the edit target — that exists
/// nowhere else in the application, it spent a whole line restating facts the
/// layers panel and status bar already show, and the two buttons were present
/// even for a layer with no mask, so the commonest case offered a choice with
/// one legal answer.
///
/// Photoshop has no such control because it does not need one: the layer and
/// mask *thumbnails* in the layers panel are the selector, and the ringed one
/// is the target. The chips below are that, brought next to the properties
/// they govern — and the mask chip appears only when there is a mask, so the
/// choice exists exactly when it is a choice (handbook 28).
///
/// The scope tabs above them are how the document is reached. Photoshop shows
/// document properties when nothing is selected in the layers panel; PhotoTux
/// always has an active layer, so asking is the honest equivalent, and it is
/// reachable rather than a state the user has to discover by accident.
ColumnLayout {
    id: header

    /// Resolve an icon stem to a URL.
    required property var iconUrl
    /// `InspectorSubject` id the panel is currently showing.
    required property string subject
    /// `InspectorSubject::Document`, the id the document tab selects.
    required property string documentSubject
    /// Whether the active layer has a mask, which is what makes the target a
    /// choice at all.
    required property bool hasMask

    /// Raised when the user picks a scope. Empty string means "follow the
    /// selection again"; otherwise a subject id to pin.
    signal scopeRequested(string pinned)

    readonly property bool showingDocument: header.subject === header.documentSubject
    readonly property bool maskTargeted: AppSession.maskEditActive

    /// Title and glyph for the subject on screen, from the engine's table.
    ///
    /// Resolved from `header.subject` rather than from the live selection: in
    /// the document scope a raster layer is still active, and reading the
    /// selection's own title and icon put the layer's glyph on the document.
    readonly property var subjectRow: {
        try {
            var rows = JSON.parse(AppSession.inspectorSubjectsJson || "[]")
            for (var i = 0; i < rows.length; ++i) {
                if (rows[i].id === header.subject)
                    return rows[i]
            }
        } catch (e) {
            // fall through
        }
        return ({ id: header.subject, title: "", icon: "" })
    }

    spacing: Theme.spaceXs

    /// One half of the scope switch.
    component ScopeTab: AbstractButton {
        id: tab
        required property bool current
        Layout.fillWidth: true
        implicitHeight: Theme.controlHeight
        focusPolicy: Qt.StrongFocus
        Accessible.role: Accessible.PageTab
        Accessible.name: tab.text
        Accessible.description: tab.current ? qsTr("Selected") : qsTr("Not selected")
        background: Rectangle {
            radius: Theme.radiusSm
            color: tab.current
                   ? Theme.toolActiveBg
                   : (tab.hovered ? Theme.surfaceContainerHigh : "transparent")
            border.color: tab.visualFocus
                          ? Theme.focusRing
                          : (tab.current ? Theme.primary : "transparent")
            border.width: (tab.current || tab.visualFocus) ? 1 : 0
        }
        contentItem: Text {
            text: tab.text
            color: tab.current ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
            font.pixelSize: Theme.fontLabel
            font.weight: tab.current ? Font.DemiBold : Font.Normal
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    /// A layer/mask thumbnail chip, sized and ringed like the layers panel's.
    ///
    /// Square and 28px because that is what reads as a thumbnail rather than a
    /// button; the ring is the same `Theme.primary` two-pixel border the mask
    /// chip in the layers panel uses, so "this one is the target" looks the
    /// same in both places.
    component TargetChip: AbstractButton {
        id: chip
        required property bool current
        required property string glyph
        property string stem: ""
        implicitWidth: 28
        implicitHeight: 28
        focusPolicy: Qt.StrongFocus
        Accessible.role: Accessible.RadioButton
        Accessible.name: chip.text
        Accessible.checkable: true
        Accessible.checked: chip.current
        ThemedToolTip {
            visible: chip.hovered
            text: chip.text
        }
        background: Rectangle {
            radius: Theme.radiusXs
            color: chip.current
                   ? Theme.surfaceRaised
                   : (chip.hovered ? Theme.surfaceContainerHigh : Theme.surfaceContainer)
            border.color: chip.visualFocus
                          ? Theme.focusRing
                          : (chip.current ? Theme.primary : Theme.border)
            border.width: (chip.current || chip.visualFocus) ? 2 : 1
        }
        contentItem: Item {
            ThemedIcon {
                anchors.centerIn: parent
                visible: chip.stem.length > 0
                source: chip.stem.length > 0 ? header.iconUrl(chip.stem) : ""
                size: Theme.iconMd
                color: chip.current ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
            }
            Label {
                anchors.centerIn: parent
                visible: chip.stem.length === 0
                text: chip.glyph
                color: chip.current ? Theme.colorOnSurface : Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                font.weight: Font.DemiBold
            }
        }
    }

    // Scope switch. Two tabs rather than a combo: there are two of them, they
    // are the panel's most-used control after the disclosure headers, and a
    // segmented pair says "these are the alternatives" without being opened.
    RowLayout {
        Layout.fillWidth: true
        spacing: 2
        visible: AppSession.hasDocument

        ScopeTab {
            text: qsTr("Layer")
            current: !header.showingDocument
            onClicked: header.scopeRequested("")
        }
        ScopeTab {
            text: qsTr("Document")
            current: header.showingDocument
            onClicked: header.scopeRequested(header.documentSubject)
        }
    }

    // Identity card — chips, name, kind.
    Rectangle {
        Layout.fillWidth: true
        visible: AppSession.hasDocument
        radius: Theme.radiusSm
        color: Theme.surfaceContainerHigh
        border.color: Theme.borderSubtle
        border.width: 1
        implicitHeight: identity.implicitHeight + Theme.spaceSm * 2

        RowLayout {
            id: identity
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceSm

            // Pixels. Not clickable while the document is showing — there is
            // no target to choose — and not clickable without a mask either,
            // because it is already the only answer.
            TargetChip {
                Layout.alignment: Qt.AlignVCenter
                text: header.showingDocument
                      ? qsTr("Document")
                      : qsTr("Edit layer pixels")
                current: header.showingDocument || !header.maskTargeted
                glyph: ""
                stem: header.subjectRow.icon
                enabled: !header.showingDocument && header.hasMask
                onClicked: AppSession.setMaskEditTarget(false)
            }
            TargetChip {
                Layout.alignment: Qt.AlignVCenter
                visible: !header.showingDocument && header.hasMask
                text: qsTr("Edit layer mask")
                current: header.maskTargeted
                glyph: qsTr("M")
                onClicked: AppSession.setMaskEditTarget(true)
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                Label {
                    Layout.fillWidth: true
                    text: header.showingDocument
                          ? AppSession.documentName
                          : (AppSession.activeLayerName.length > 0
                             ? AppSession.activeLayerName : qsTr("Untitled layer"))
                    color: Theme.colorOnSurface
                    font.pixelSize: Theme.fontBodySm
                    font.weight: Font.DemiBold
                    elide: Text.ElideMiddle
                }
                // One subordinate line, and only facts the header is the right
                // place for: what kind of thing this is, and — when it is a
                // choice — which half of it a stroke would land on.
                Label {
                    Layout.fillWidth: true
                    text: {
                        var title = header.subjectRow.title
                        if (header.showingDocument)
                            return qsTr("%1 · %2 × %3").arg(title)
                                   .arg(AppSession.docWidth).arg(AppSession.docHeight)
                        if (!header.hasMask)
                            return title
                        return header.maskTargeted
                               ? qsTr("%1 · painting on the mask").arg(title)
                               : qsTr("%1 · painting on the pixels").arg(title)
                    }
                    color: Theme.colorOnSurfaceMuted
                    font.pixelSize: Theme.fontLabelSm
                    elide: Text.ElideRight
                }
            }
        }
    }
}
