import QtQuick
import QtQuick.Controls
import phototux_ui

/// Value slider drawn from `Theme` tokens.
///
/// Companion to `ThemedCheckBox` / `ThemedComboBox` / `ThemedSpinBox`, and the
/// last of the four the shell instantiates in quantity. The Basic style's
/// slider is a bright blue groove under a large white disc: taller than the
/// rows it sits in and the loudest thing in a panel of muted chrome, which is
/// exactly backwards for a control that appears eighteen times.
///
/// Thin groove, small handle, accent on the filled side only — the Plasma
/// idiom, and the one that lets a column of sliders read as a column rather
/// than as a stack of separate widgets.
Slider {
    id: control

    implicitHeight: Theme.controlHeight

    background: Rectangle {
        x: control.leftPadding
        y: control.topPadding + control.availableHeight / 2 - height / 2
        implicitWidth: 120
        width: control.availableWidth
        height: 4
        radius: 2
        color: Theme.surfaceSunken
        border.color: Theme.borderSubtle
        border.width: 1

        // The travelled part, so the reading is legible without the number
        // beside it — which is what a glance at a column of sliders is for.
        Rectangle {
            width: control.visualPosition * parent.width
            height: parent.height
            radius: parent.radius
            color: control.enabled ? Theme.primary : Theme.colorOnSurfaceDisabled
        }
    }

    handle: Rectangle {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + control.availableHeight / 2 - height / 2
        implicitWidth: 14
        implicitHeight: 14
        radius: width / 2
        color: control.enabled
               ? (control.pressed ? Theme.primaryHover : Theme.primary)
               : Theme.surfaceContainer
        // A ring rather than a shadow: the handle has to stay findable where
        // it sits on top of the filled groove, which is the same colour.
        border.color: control.activeFocus ? Theme.focusRing : Theme.surface
        border.width: 2

        // Grows under the pointer. The handle is small by design, and the
        // hit area is the whole control, so this is feedback rather than a
        // change in what can be grabbed.
        scale: control.hovered || control.pressed ? 1.15 : 1.0
        Behavior on scale {
            enabled: !Theme.reducedMotion
            NumberAnimation { duration: 90; easing.type: Easing.OutCubic }
        }
    }
}
