import QtQuick
import QtQuick.Controls
import phototux_ui

/// One row of a menu, drawn from `Theme` tokens.
///
/// The last unstyled control in the shell. No Controls style is configured, so
/// menus ran the Basic style: a light popup with dark text, opening out of dark
/// editor chrome every time anyone used the menu bar or a context menu.
///
/// Three columns, which is what a menu row actually is: a marker slot that
/// holds either the item's icon or its check mark, the label, and the shortcut
/// chord pushed to the right. The shell packs the chord into `text` after a
/// tab, which Basic rendered as a literal tab character — a ragged column of
/// labels with their chords wherever the tab stop happened to land.
MenuItem {
    id: control

    /// `[label]` or `[label, chord]`, as the shell packs it.
    readonly property var _parts: control.text.split("\t")
    readonly property string _label: Theme.withoutMnemonic(control._parts[0])
    readonly property string _chord: control._parts.length > 1 ? control._parts[1] : ""
    readonly property color _ink: control.enabled
                                  ? Theme.colorOnSurfaceEffective
                                  : Theme.colorOnSurfaceDisabled

    /// Width of the marker column, reserved whether or not this row uses it so
    /// that labels line up down the menu rather than per-item.
    readonly property int markerWidth: Theme.iconMd

    implicitHeight: Math.max(Theme.controlHeight, implicitContentHeight + topPadding + bottomPadding)
    padding: Theme.spaceSm
    spacing: Theme.spaceSm
    icon.width: Theme.iconMd
    icon.height: Theme.iconMd

    contentItem: Item {
        implicitWidth: markerSlot.width + labelText.implicitWidth + control.spacing
                       + (chordText.visible ? chordText.implicitWidth + Theme.spaceXl : 0)
                       + (control.subMenu ? Theme.iconMd + control.spacing : 0)
        implicitHeight: Math.max(Theme.iconMd, labelText.implicitHeight)

        Item {
            id: markerSlot
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            width: control.markerWidth
            height: control.markerWidth

            ThemedIcon {
                anchors.centerIn: parent
                visible: control.icon.source.toString().length > 0 && !control.checked
                source: control.icon.source
                size: Theme.iconMd
                color: control.enabled ? Theme.iconOnSurfaceEffective
                                       : Theme.iconDisabledEffective
            }
        }

        Text {
            id: labelText
            anchors.left: markerSlot.right
            anchors.leftMargin: control.spacing
            anchors.verticalCenter: parent.verticalCenter
            text: control._label
            font: control.font
            color: control._ink
            elide: Text.ElideRight
        }

        // Muted and right-aligned: a chord is a reminder, not the thing you
        // came to read, and a column of them is only scannable when they end
        // on the same edge.
        Text {
            id: chordText
            anchors.right: parent.right
            anchors.rightMargin: control.subMenu ? Theme.iconMd + control.spacing : 0
            anchors.verticalCenter: parent.verticalCenter
            visible: control._chord.length > 0
            text: control._chord
            font.pixelSize: Theme.fontLabelSm
            font.family: "Noto Sans Mono"
            color: Theme.colorOnSurfaceMuted
        }
    }

    // A check in the marker column, so a checked item and an item with an icon
    // occupy the same slot rather than shifting the label between them.
    indicator: ThemedIcon {
        x: control.leftPadding + (control.markerWidth - width) / 2
        y: control.topPadding + (control.availableHeight - height) / 2
        visible: control.checked
        source: Theme.iconUrl(AppSession.iconRoot, "check")
        size: Theme.iconMd
        color: control.enabled ? Theme.primary : Theme.iconDisabledEffective
    }

    arrow: ThemedIcon {
        x: control.mirrored ? control.leftPadding : control.width - width - control.rightPadding
        y: control.topPadding + (control.availableHeight - height) / 2
        visible: control.subMenu
        source: Theme.iconUrl(AppSession.iconRoot, "caret-right")
        size: Theme.iconMd
        color: control.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
    }

    // Opaque, unlike the accent wash the rest of the chrome uses for a
    // selected row. Something below the item paints a pale focus frame on the
    // current row inside a menu popup — visible with no background at all —
    // and a translucent highlight sat on top of it and read light blue. An
    // opaque fill covers it, and `surfaceRaised` is what a raised row is
    // everywhere else in the shell.
    background: Rectangle {
        implicitWidth: 200
        radius: Theme.radiusXs
        color: control.down
               ? Theme.surfaceContainerHigh
               : (control.highlighted ? Theme.surfaceRaised : "transparent")
    }
}
