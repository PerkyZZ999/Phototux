import QtQuick
import QtQuick.Controls
import phototux_ui

/// Numeric stepper drawn from `Theme` tokens.
///
/// Companion to `ThemedCheckBox` / `ThemedComboBox`: the Basic style's white
/// field and black digits are unreadable against dark editor chrome once the
/// surrounding panel is dark. The value field is sunken and the two steppers
/// sit flush on either side, which is how raster editors present a numeric
/// entry that is also draggable by keyboard.
SpinBox {
    id: control

    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontBodySm

    background: Rectangle {
        implicitWidth: 96
        implicitHeight: Theme.controlHeight
        color: control.enabled ? Theme.surfaceSunken : Theme.surfaceContainer
        radius: Theme.radiusSm
        border.width: control.activeFocus ? 2 : 1
        border.color: control.activeFocus ? Theme.focusRing : Theme.borderEffective
    }

    contentItem: TextInput {
        text: control.displayText
        font: control.font
        color: control.enabled ? Theme.colorOnSurfaceEffective : Theme.colorOnSurfaceDisabled
        selectionColor: Theme.primary
        selectedTextColor: Theme.primaryOn
        horizontalAlignment: Qt.AlignHCenter
        verticalAlignment: Qt.AlignVCenter
        readOnly: !control.editable
        validator: control.validator
        inputMethodHints: Qt.ImhFormattedNumbersOnly
    }

    up.indicator: Rectangle {
        x: control.mirrored ? 0 : control.width - width
        height: control.height
        implicitWidth: Theme.controlHeight
        color: control.up.pressed ? Theme.surfaceContainerHigh
                                  : (control.up.hovered ? Theme.surfaceRaised : "transparent")
        radius: Theme.radiusSm

        ThemedIcon {
            anchors.centerIn: parent
            size: Theme.iconMd
            source: Theme.iconUrl(AppSession.iconRoot, "caret-up")
            color: control.enabled && control.value < control.to
                   ? Theme.colorOnSurfaceVariant : Theme.iconDisabledEffective
        }
    }

    down.indicator: Rectangle {
        x: control.mirrored ? control.width - width : 0
        height: control.height
        implicitWidth: Theme.controlHeight
        color: control.down.pressed ? Theme.surfaceContainerHigh
                                    : (control.down.hovered ? Theme.surfaceRaised : "transparent")
        radius: Theme.radiusSm

        ThemedIcon {
            anchors.centerIn: parent
            size: Theme.iconMd
            source: Theme.iconUrl(AppSession.iconRoot, "caret-down")
            color: control.enabled && control.value > control.from
                   ? Theme.colorOnSurfaceVariant : Theme.iconDisabledEffective
        }
    }
}
