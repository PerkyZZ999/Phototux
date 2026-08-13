import QtQuick
import QtQuick.Controls

/// Checkbox drawn from `Theme` tokens instead of the Basic style's defaults.
///
/// No Qt Quick Controls style is configured, so the shell runs the **Basic**
/// style, which hardcodes a light palette and ignores `palette` overrides. A
/// plain `CheckBox` therefore drew its label in near-black on our dark surfaces
/// — around 1.3:1, far under the AA floor handbook 28 requires — while the
/// indicator kept a white plate that read as a light-theme widget dropped into
/// dark chrome. Every checkbox in the shell uses this type so that stays fixed
/// in one place rather than per call site.
///
/// Checked state is carried by fill, glyph **and** border, never colour alone.
CheckBox {
    id: control

    implicitHeight: Math.max(Theme.controlHeight, indicator.implicitHeight)
    font.pixelSize: Theme.fontBodySm

    indicator: Rectangle {
        implicitWidth: Theme.iconMd
        implicitHeight: Theme.iconMd
        x: control.leftPadding
        y: control.topPadding + (control.availableHeight - height) / 2
        radius: Theme.radiusXs
        color: control.checked ? Theme.primary : Theme.surfaceSunken
        border.width: control.visualFocus ? 2 : 1
        border.color: control.visualFocus
                      ? Theme.focusRing
                      : (control.checked ? Theme.primary : Theme.borderEffective)
        opacity: control.enabled ? 1.0 : 0.5

        // Non-colour marker: the tick is what distinguishes the states in the
        // high-contrast pack and for anyone who cannot separate the two fills.
        Text {
            anchors.centerIn: parent
            visible: control.checked
            text: "✓"
            color: Theme.primaryOn
            font.pixelSize: Math.round(Theme.iconMd * 0.8)
            font.weight: Font.Bold
        }
    }

    contentItem: Text {
        text: control.text
        font: control.font
        color: control.enabled ? Theme.colorOnSurfaceEffective : Theme.colorOnSurfaceDisabled
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        leftPadding: control.indicator.width + Theme.spaceSm
    }
}
