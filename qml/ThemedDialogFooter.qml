import QtQuick
import QtQuick.Controls
import phototux_ui

/// Dialog button row drawn from `Theme` tokens.
///
/// `DialogButtonBox` takes two things from the Controls style: its own
/// background strip, and — whenever `standardButtons` is used instead of
/// explicit children — the buttons themselves. The shell configures no style,
/// so on a clean profile that is Basic: a light strip carrying light buttons,
/// welded to the bottom of a dark dialog. `ThemedButton` cannot fix that on
/// its own because the call site never writes the buttons down.
///
/// `alignment` matters as much as `delegate`. Left unset, `DialogButtonBox`
/// resizes its buttons to fill the row, so a two-button dialog gets two
/// half-width slabs and a narrow one gets elided labels. Right-aligned buttons
/// keep their natural width and sit where a Plasma dialog puts them.
DialogButtonBox {
    id: control

    delegate: ThemedButton {}

    alignment: Qt.AlignRight
    padding: Theme.spaceMd
    topPadding: Theme.spaceSm
    spacing: Theme.spaceSm

    background: Rectangle {
        color: "transparent"
    }
}
