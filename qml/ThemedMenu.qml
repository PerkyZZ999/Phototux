import QtQuick
import QtQuick.Controls
import phototux_ui

/// Menu popup drawn from `Theme` tokens.
///
/// Companion to `ThemedMenuItem`, and the pair that finishes the set: with no
/// Controls style configured the shell ran Basic, whose menus are a light panel
/// with dark text — opening out of dark editor chrome every time anyone touched
/// the menu bar or right-clicked.
///
/// `delegate` matters as much as `background`: a nested menu is presented in
/// its parent as an item built from that delegate, so a submenu row is themed
/// by this one line rather than at each of the nine places one is declared.
Menu {
    id: control

    delegate: ThemedMenuItem {}

    // Room for the popup's own edge, so the first and last rows do not sit on
    // the border. The rows carry their own horizontal padding.
    topPadding: Theme.spaceXxs
    bottomPadding: Theme.spaceXxs

    background: Rectangle {
        implicitWidth: 220
        color: Theme.surfaceOverlay
        border.color: Theme.border
        border.width: 1
        radius: Theme.radiusSm
    }
}
