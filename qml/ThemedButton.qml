import QtQuick
import QtQuick.Controls
import phototux_ui

/// Push button drawn from `Theme` tokens.
///
/// Same reason as `ThemedCheckBox` and `ThemedComboBox`: with no Controls
/// style configured the shell runs the **Basic** style, whose hardcoded light
/// palette put white buttons into dark editor chrome. That was invisible on a
/// developer profile with a Qt style configured system-wide and obvious on a
/// clean one, which is the profile every user has.
///
/// It also collects a treatment that had been written out per call site — the
/// same rounded rectangle, border and label, six times across four files, and
/// missing everywhere else.
///
/// Three prominences, because a surface has at most three kinds of button:
/// the one it commits with, the ordinary ones, and destructive ones. `flat`
/// is the fourth state and is orthogonal — a button with no resting fill, for
/// runs of secondary actions inside a panel.
Button {
    id: control

    /// `normal` | `primary` | `danger`.
    property string prominence: "normal"

    readonly property bool _danger: control.prominence === "danger"
    readonly property bool _primary: control.prominence === "primary"

    readonly property color _accent: control._primary
                                     ? Theme.primary
                                     : (control._danger ? Theme.error : Theme.surfaceRaised)
    /// A flat destructive button stays quiet until the pointer is on it, which
    /// is how the shell already draws "Discard": a run of red text beside every
    /// row reads as an error state rather than as an action.
    readonly property color _label: {
        if (control._danger)
            return control.flat
                   ? ((control.hovered || control.down) ? Theme.error
                                                        : Theme.colorOnSurfaceMuted)
                   : Theme.primaryOn
        return control._primary ? Theme.primaryOn : Theme.colorOnSurface
    }

    implicitHeight: Theme.controlHeight
    padding: Theme.spaceSm
    font.pixelSize: Theme.fontLabel
    Accessible.name: control.text

    background: Rectangle {
        radius: Theme.radiusSm
        color: {
            if (control.flat && !control.down && !control.hovered)
                return "transparent"
            if (control.down)
                return control._primary ? Theme.primaryHover : Theme.surfaceContainerHigh
            if (control.hovered && control.prominence === "normal")
                return Theme.surfaceContainerHigh
            return control._accent
        }
        border.color: control.visualFocus
                      ? Theme.focusRing
                      : (control.prominence === "normal" && !control.flat
                         ? Theme.borderSubtle : "transparent")
        border.width: (control.visualFocus
                       || (control.prominence === "normal" && !control.flat)) ? 1 : 0
        // Dimmed rather than greyed: the label keeps its contrast ratio, and
        // a disabled button still has to be readable to say why it is there.
        opacity: control.enabled ? 1.0 : 0.55
    }

    contentItem: Text {
        text: control.text
        color: control.enabled ? control._label : Theme.colorOnSurfaceDisabled
        font.pixelSize: control.font.pixelSize
        font.family: control.font.family
        font.weight: control._primary ? Font.DemiBold : Font.Normal
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
}
