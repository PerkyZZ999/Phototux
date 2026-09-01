import QtQuick
import QtQuick.Controls
import phototux_ui

/// The draggable seam between two stacked dock panels.
///
/// Sits on the top edge of a panel header and resizes the panel *above* it,
/// which is the seam a user aims at: the line they can see between the two.
/// The dock had no such affordance at all — every panel's height was a constant
/// in the shell, so the Properties panel was permanently capped at a fraction
/// of the dock and its longer groups could only be scrolled.
///
/// Commits on release, not on every motion event. A commit bumps the workspace
/// revision and writes preferences, and a drag would otherwise do that sixty
/// times a second.
Item {
    id: root

    /// Panel whose body this grip resizes — the one above the seam.
    required property string panelId
    /// Live height of that panel's body, in pixels.
    required property int currentHeight
    /// Reported while dragging so the panel above can follow the pointer.
    signal previewed(int height)
    /// Reported on release, once.
    signal committed(int height)

    /// Floor and ceiling mirror `DockTopology::MIN_PANEL_HEIGHT` / `MAX`.
    /// The engine clamps too — this is so the drag *feels* bounded rather than
    /// running past the limit and snapping back on release.
    readonly property int minimumHeight: 64
    readonly property int maximumHeight: 2000

    implicitHeight: 5
    z: 30

    property int _startHeight: 0

    /// The height this drag has reached, given a pointer `y` inside the grip.
    function heightAt(pointerY) {
        var wanted = root._startHeight + (pointerY - grip.pressY)
        return Math.max(root.minimumHeight,
                        Math.min(root.maximumHeight, Math.round(wanted)))
    }

    // The seam is a hairline until you approach it, then it lights up. A
    // permanently visible handle on every panel boundary would read as chrome
    // the dock does not otherwise have.
    Rectangle {
        anchors.fill: parent
        color: (grip.containsMouse || grip.pressed) ? Theme.primary : "transparent"
        opacity: grip.pressed ? 1.0 : 0.7
    }

    // A MouseArea rather than a DragHandler, because the panel header behind
    // this one is a MouseArea too — it handles the reorder drag — and it takes
    // the exclusive grab on press. A pointer handler would then never see the
    // motion however it was stacked. Matching the idiom is what makes the two
    // resolve by z-order the way they look like they should.
    MouseArea {
        id: grip
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.SizeVerCursor
        preventStealing: true
        property real pressY: 0

        onPressed: function (mouse) {
            root._startHeight = root.currentHeight
            grip.pressY = mouse.y
        }
        onPositionChanged: function (mouse) {
            if (grip.pressed)
                root.previewed(root.heightAt(mouse.y))
        }
        // Computed from the release position rather than from whatever the last
        // preview happened to be. Motion events are not guaranteed — a fast
        // drag, or a compositor that coalesces them, can deliver one or none —
        // and a resize that depends on having seen them commits the height it
        // started with.
        onReleased: function (mouse) {
            root.committed(root.heightAt(mouse.y))
        }
    }

    // Keyboard path: the grip is focusable and resizes in steps, so the dock is
    // adjustable without a pointer.
    Accessible.role: Accessible.Separator
    Accessible.name: qsTr("Resize the panel above, %1 pixels").arg(root.currentHeight)
    Accessible.description: qsTr("Up and down arrows resize")
    activeFocusOnTab: true
    Keys.onUpPressed: root.committed(Math.max(root.minimumHeight, root.currentHeight - 16))
    Keys.onDownPressed: root.committed(Math.min(root.maximumHeight, root.currentHeight + 16))

    // Focus is otherwise invisible on a hairline.
    Rectangle {
        anchors.fill: parent
        visible: root.activeFocus
        color: "transparent"
        border.color: Theme.focusRing
        border.width: 1
    }
}
