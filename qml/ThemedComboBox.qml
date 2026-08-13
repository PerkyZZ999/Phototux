import QtQuick
import QtQuick.Controls
import phototux_ui

/// Combo box drawn from `Theme` tokens, including its drop-down list.
///
/// Same reason as `ThemedCheckBox`: with no Controls style configured the shell
/// runs the Basic style, whose hardcoded light palette put white fields and
/// white popup lists into dark editor chrome. Raster editors keep field chrome
/// darker than the panel it sits on, so the closed control reads as sunken.
ComboBox {
    id: control

    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontBodySm

    background: Rectangle {
        implicitWidth: 96
        implicitHeight: Theme.controlHeight
        color: control.enabled
               ? (control.hovered ? Theme.surfaceContainerHigh : Theme.surfaceSunken)
               : Theme.surfaceContainer
        radius: Theme.radiusSm
        border.width: control.visualFocus ? 2 : 1
        border.color: control.visualFocus ? Theme.focusRing : Theme.borderEffective
    }

    contentItem: Text {
        leftPadding: Theme.spaceSm
        rightPadding: Theme.spaceXs
        text: control.displayText
        font: control.font
        color: control.enabled ? Theme.colorOnSurfaceEffective : Theme.colorOnSurfaceDisabled
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    indicator: ThemedIcon {
        x: control.width - width - Theme.spaceXs
        y: control.topPadding + (control.availableHeight - height) / 2
        size: Theme.iconMd
        source: Theme.iconUrl(AppSession.iconRoot, "caret-down")
        color: control.enabled ? Theme.colorOnSurfaceVariant : Theme.iconDisabledEffective
    }

    // The drop-down list is deliberately left to the style.
    //
    // Overriding `popup` + `delegate` to theme it left the row at `currentIndex`
    // blank in every combo — the list reserved its slot but painted neither the
    // label nor the highlight, through both DelegateModel access and direct
    // array indexing. A light-on-dark list is a cosmetic mismatch; a list that
    // hides one of its options is a functional defect, so the popup keeps the
    // style's own rendering until that is understood. Tracked in the gap
    // analysis as the dark combo popup item.
}
