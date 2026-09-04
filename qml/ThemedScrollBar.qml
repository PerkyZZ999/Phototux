import QtQuick
import QtQuick.Controls
import phototux_ui

/// Scroll bar drawn from `Theme` tokens.
///
/// The Basic style draws its handle in `palette.mid` and, under
/// `policy: AlwaysOn`, keeps a track visible behind it — a pale grey blob on a
/// hairline, the brightest thing in a dark panel, and a rule that ran straight
/// through the controls it overlapped in the Preferences dialog.
///
/// The bar sits *over* content rather than beside it, so it stays narrow and
/// quiet at rest and thickens under the pointer, which is what Plasma does.
/// A caller that cannot afford the overlap gives its content a right margin;
/// `implicitWidth` is the number to reserve.
ScrollBar {
    id: control

    implicitWidth: Theme.spaceMd
    padding: Theme.spaceXxs

    contentItem: Rectangle {
        implicitWidth: control.hovered || control.pressed
                       ? Theme.spaceSm : Theme.spaceXs
        radius: width / 2
        color: {
            if (control.pressed)
                return Theme.primary
            return control.hovered ? Theme.colorOnSurfaceMuted : Theme.border
        }
        // Fades out with the bar itself on an AsNeeded policy, and stays put
        // on AlwaysOn — `ScrollBar.active` covers both.
        opacity: control.policy === ScrollBar.AlwaysOn || control.active ? 0.9 : 0.0
        visible: control.size < 1.0

        // Both gated on the accessibility preference, like the slider's scale
        // and the toast fade. These two were the ones it did not reach, so
        // "Reduced motion" left the scroll bar still growing and fading —
        // motion in the corner of the eye is exactly what the preference is
        // asked for.
        Behavior on implicitWidth {
            enabled: !Theme.reducedMotion
            NumberAnimation { duration: 90; easing.type: Easing.OutQuad }
        }
        Behavior on opacity {
            enabled: !Theme.reducedMotion
            NumberAnimation { duration: 120 }
        }
    }

    background: null
}
