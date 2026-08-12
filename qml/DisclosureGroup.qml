import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Collapsible inspector section implementing handbook 01/28 progressive disclosure.
///
/// Context decides whether a group is present (`visible`); disclosure decides how
/// much of it is shown (`expanded`). Expansion is presentation state persisted via
/// `AppSession.setDisclosureOpen`, never document state.
///
/// The body is a `Component` (default property), so it is built on first expansion
/// and then retained — collapsing preserves in-progress control values, which the
/// handbook requires of disclosure.
ColumnLayout {
    id: control

    /// Stable descriptor id from `AppSession.disclosureGroupsJson`.
    required property string groupId
    /// Concept name shown in the header — never "More" or "Advanced" alone.
    required property string title
    /// Summary shown beside the title when collapsed (information scent).
    property string summary: ""
    /// Non-empty when the collapsed group hides a warning or invalid value.
    ///
    /// Resolved from host state by group id, so a badge does not depend on the
    /// body existing. Call sites may override for group-local conditions.
    property string badgeText: control.hostBadge ? control.hostBadge.text : ""
    /// `warning` | `error`. Drives badge color and the accessible description.
    property string badgeSeverity: control.hostBadge ? control.hostBadge.severity : "warning"
    /// Keep the body instantiated after first expansion (value preservation).
    property bool retain: true

    default property Component contentComponent

    readonly property var disclosureMap: {
        try {
            return JSON.parse(AppSession.disclosureOpenJson || "{}")
        } catch (e) {
            return ({})
        }
    }
    readonly property var badgeMap: {
        try {
            return JSON.parse(AppSession.inspectorBadgesJson || "{}")
        } catch (e) {
            return ({})
        }
    }
    readonly property var hostBadge: control.badgeMap[control.groupId] || null
    /// Resolved by the host: descriptor default merged with the user override.
    readonly property bool expanded: disclosureMap[control.groupId] === true
    readonly property bool hasBadge: control.badgeText.length > 0

    property bool everExpanded: false

    spacing: 0
    Layout.fillWidth: true

    Component.onCompleted: if (expanded) everExpanded = true
    onExpandedChanged: if (expanded) everExpanded = true

    function toggle() {
        AppSession.setDisclosureOpen(control.groupId, !control.expanded)
    }

    // Header — the whole row is the hit target and the single focus stop.
    AbstractButton {
        id: header
        Layout.fillWidth: true
        implicitHeight: Theme.panelHeaderHeight
        focusPolicy: Qt.StrongFocus
        // AbstractButton already maps Space/Enter to clicked(); do not re-handle.
        onClicked: control.toggle()

        Accessible.role: Accessible.Button
        Accessible.name: control.title
        // Non-color state: expansion and any hidden badge reach assistive tech.
        Accessible.description: (control.expanded ? qsTr("Expanded") : qsTr("Collapsed"))
                                + (control.hasBadge ? ". " + control.badgeText : "")

        // Arrow keys are the conventional disclosure grammar and are not
        // consumed by AbstractButton.
        Keys.onRightPressed: if (!control.expanded) control.toggle()
        Keys.onLeftPressed: if (control.expanded) control.toggle()

        background: Rectangle {
            color: header.down
                   ? Theme.surfaceContainerHigh
                   : (header.hovered ? Theme.surfaceContainer : "transparent")
            radius: Theme.radiusSm
            border.width: header.activeFocus ? 1 : 0
            border.color: Theme.focusRing
        }

        contentItem: RowLayout {
            spacing: Theme.spaceXs

            // Caret direction matches the keyboard grammar below: Right expands
            // a collapsed group, so a collapsed group points right.
            ThemedIcon {
                source: Theme.iconUrl(AppSession.iconRoot,
                                      control.expanded ? "caret-down" : "caret-right")
                size: Theme.iconMd
                color: Theme.iconOnSurfaceEffective
            }

            Label {
                text: control.title
                color: Theme.colorOnSurfaceEffective
                font.pixelSize: Theme.fontLabel
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            // Collapsed summary keeps information scent without expanding.
            Label {
                visible: !control.expanded && control.summary.length > 0
                text: control.summary
                color: Theme.colorOnSurfaceMuted
                font.pixelSize: Theme.fontLabelSm
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignRight
                Layout.fillWidth: true
            }

            Item {
                Layout.fillWidth: true
                visible: control.expanded || control.summary.length === 0
            }

            // Handbook: hidden invalid values MUST surface at the collapsed group.
            // The badge outranks the summary for space; it elides rather than
            // pushing the row wider than a narrow dock, and the header's
            // accessible description still carries the full text.
            Rectangle {
                visible: control.hasBadge
                radius: Theme.radiusXs
                color: control.badgeSeverity === "error" ? Theme.error : Theme.warning
                implicitWidth: badgeLabel.implicitWidth + Theme.spaceXs * 2
                implicitHeight: badgeLabel.implicitHeight + Theme.spaceXxs * 2
                Layout.maximumWidth: header.width * 0.6

                Label {
                    id: badgeLabel
                    anchors.centerIn: parent
                    width: Math.min(implicitWidth, parent.width - Theme.spaceXs * 2)
                    text: control.badgeText
                    color: Theme.primaryOn
                    font.pixelSize: Theme.fontLabelSm
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }
        }
    }

    // Body — lazy on first expansion, retained afterwards.
    Loader {
        id: bodyLoader
        Layout.fillWidth: true
        Layout.topMargin: control.expanded ? Theme.spaceXxs : 0
        // Gated on `visible` as well: a group whose context is absent (wrong
        // tool, wrong layer kind) must not build its body just because the user
        // last left it expanded.
        active: control.visible
                && (control.expanded || (control.retain && control.everExpanded))
        visible: control.expanded
        sourceComponent: control.contentComponent
    }
}
