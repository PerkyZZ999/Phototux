import QtQuick
import QtQuick.Controls

/// Icon-only tool button with white Phosphor glyphs on dark chrome (WCAG).
ToolButton {
    id: control
    display: AbstractButton.IconOnly
    padding: 4
    icon.width: 18
    icon.height: 18
    icon.color: enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective

    contentItem: Item {
        implicitWidth: control.icon.width
        implicitHeight: control.icon.height
        ThemedIcon {
            anchors.centerIn: parent
            source: control.icon.source
            size: Math.max(control.icon.width, control.icon.height)
            color: control.enabled ? Theme.iconOnSurfaceEffective : Theme.iconDisabledEffective
            opacity: control.enabled ? 1.0 : 0.9
        }
    }

    background: Rectangle {
        implicitWidth: 34
        implicitHeight: 34
        radius: Theme.radiusSm
        color: control.down || control.checked
               ? Theme.toolActiveBg
               : (control.hovered ? Theme.surfaceContainerHigh : "transparent")
        border.color: control.checked ? Theme.primary : "transparent"
        border.width: 1
    }
}
