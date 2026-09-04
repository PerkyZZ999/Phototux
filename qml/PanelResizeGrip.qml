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

    /// Floor mirrors `DockTopology::MIN_PANEL_HEIGHT`. The engine clamps too —
    /// this is so the drag *feels* bounded rather than running past the limit
    /// and snapping back on release.
    readonly property int minimumHeight: Theme.dockPanelMinHeight
    /// Ceiling, which the dock supplies because only it knows how much room
    /// the panels below the seam still need.
    ///
    /// It was a constant 2000, mirroring `DockTopology::MAX_PANEL_HEIGHT`, and
    /// neither side subtracted the stack below — so one drag to the bottom of
    /// the screen made the panel above fill the dock and every group under it
    /// vanish, with nothing on screen saying where they had gone. A negative
    /// value means the dock could not work one out, and the absolute bound
    /// stands.
    property int maximumHeight: 2000
    readonly property int effectiveMaximum: maximumHeight > minimumHeight
                                            ? maximumHeight : 2000

    implicitHeight: 5
    z: 30

    property int _startHeight: 0

    /// The height this drag has reached, for a pointer at scene `y`.
    ///
    /// Scene coordinates, not the grip's own. The grip rides on the header of
    /// the panel *below* the one being resized, so growing that panel moves
    /// this item down by exactly the amount the drag just added — and a delta
    /// measured against a moved origin subtracts the resize from itself. The
    /// first motion event landed the right height and every one after it
    /// pulled back towards where the drag started, so the seam crawled at a
    /// fraction of the pointer instead of following it. The scene does not
    /// move.
    function heightAt(sceneY) {
        var wanted = root._startHeight + (sceneY - grip.pressSceneY)
        return Math.max(root.minimumHeight,
                        Math.min(root.effectiveMaximum, Math.round(wanted)))
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
        property real pressSceneY: 0

        function sceneY(mouse) {
            return grip.mapToItem(null, mouse.x, mouse.y).y
        }

        onPressed: function (mouse) {
            root._startHeight = root.currentHeight
            grip.pressSceneY = grip.sceneY(mouse)
        }
        onPositionChanged: function (mouse) {
            if (grip.pressed)
                root.previewed(root.heightAt(grip.sceneY(mouse)))
        }
        // Computed from the release position rather than from whatever the last
        // preview happened to be. Motion events are not guaranteed — a fast
        // drag, or a compositor that coalesces them, can deliver one or none —
        // and a resize that depends on having seen them commits the height it
        // started with.
        onReleased: function (mouse) {
            root.committed(root.heightAt(grip.sceneY(mouse)))
        }
    }

    // Keyboard path: the grip is focusable and resizes in steps, so the dock is
    // adjustable without a pointer.
    Accessible.role: Accessible.Separator
    // `currentHeight` is -1 for a panel the dock is sizing itself, and a
    // screen reader was being told "Resize the panel above, -1 pixels".
    Accessible.name: root.currentHeight >= 0
                     ? qsTr("Resize the panel above, %1 pixels").arg(root.currentHeight)
                     : qsTr("Resize the panel above, automatic height")
    Accessible.description: qsTr("Up and down arrows resize")
    activeFocusOnTab: true
    Keys.onUpPressed: root.committed(Math.max(root.minimumHeight, root.currentHeight - 16))
    Keys.onDownPressed: root.committed(Math.min(root.effectiveMaximum, root.currentHeight + 16))

    // Focus is otherwise invisible on a hairline.
    Rectangle {
        anchors.fill: parent
        visible: root.activeFocus
        color: "transparent"
        border.color: Theme.focusRing
        border.width: 1
    }
}
