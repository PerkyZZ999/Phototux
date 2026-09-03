import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import phototux_ui

/// Transient messages, stacked above the status bar.
///
/// Messages used to be written into the status bar, which also carries the
/// document summary — so a message was erased by the next summary refresh, and
/// a user looking at the canvas never saw it. The two are different: the
/// summary is state, always true; a notice is an event, true once.
///
/// Bottom centre, over the canvas, close to where the eye already is during a
/// gesture. Newest at the bottom so a stack grows away from the canvas centre
/// rather than pushing older messages under the pointer.
///
/// Hovering holds a toast open. A message worth reading is often longer than
/// three seconds' worth of reading, and the cursor is already on its way to the
/// close button.
Item {
    id: root

    /// Resolve an icon stem to a URL, matching the other chrome components.
    required property var iconUrl

    /// Milliseconds a fading notice stays before it dismisses itself.
    property int dwellMs: 3000

    readonly property var notices: {
        try {
            return JSON.parse(AppSession.noticesJson || "[]")
        } catch (e) {
            return []
        }
    }

    // Only the toasts themselves take input; the rest of this item is canvas.
    visible: root.notices.length > 0

    function levelColor(level) {
        if (level === "error")
            return Theme.error
        if (level === "warning")
            return Theme.warning
        return Theme.primary
    }

    ColumnLayout {
        id: stack
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spaceLg
        width: Math.min(parent.width - Theme.spaceXl * 2, 520)
        spacing: Theme.spaceXs

        Repeater {
            model: root.notices
            delegate: Rectangle {
                id: toast
                required property var modelData

                readonly property color accent: root.levelColor(toast.modelData.level)

                Layout.fillWidth: true
                Layout.alignment: Qt.AlignHCenter
                implicitHeight: Math.max(Theme.toolHit, toastRow.implicitHeight + Theme.spaceSm * 2)
                radius: Theme.radiusMd
                color: Theme.surfaceOverlay
                border.color: toast.accent
                border.width: 1

                // Announced as an alert so a screen reader speaks it when it
                // appears, rather than only when focus happens to land on it.
                Accessible.role: Accessible.AlertMessage
                Accessible.name: toast.modelData.spoken

                // The accent runs down the leading edge rather than tinting the
                // whole surface: severity has to be legible without relying on
                // colour, and a full wash would fight the canvas behind it.
                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    anchors.margins: 1
                    width: 3
                    radius: Theme.radiusXs
                    color: toast.accent
                }

                HoverHandler { id: toastHover }

                RowLayout {
                    id: toastRow
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spaceMd
                    anchors.rightMargin: Theme.spaceXs
                    anchors.topMargin: Theme.spaceSm
                    anchors.bottomMargin: Theme.spaceSm
                    spacing: Theme.spaceSm

                    ThemedIcon {
                        Layout.alignment: Qt.AlignVCenter
                        source: root.iconUrl(toast.modelData.icon)
                        size: Theme.iconMd
                        color: toast.accent
                    }
                    Label {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        text: toast.modelData.text
                        color: Theme.colorOnSurface
                        font.pixelSize: Theme.fontBodySm
                        wrapMode: Text.WordWrap
                        maximumLineCount: 3
                        elide: Text.ElideRight
                    }
                    // A repeat counts up in place rather than stacking another
                    // identical toast, so one refused command clicked four
                    // times does not become a wall.
                    Rectangle {
                        visible: toast.modelData.repeats > 1
                        Layout.alignment: Qt.AlignVCenter
                        implicitWidth: repeatLabel.implicitWidth + Theme.spaceSm
                        implicitHeight: repeatLabel.implicitHeight + Theme.spaceXxs
                        radius: height / 2
                        color: Theme.surfaceContainerHigh
                        Label {
                            id: repeatLabel
                            anchors.centerIn: parent
                            text: "×" + toast.modelData.repeats
                            color: Theme.colorOnSurfaceMuted
                            font.pixelSize: Theme.fontLabelSm
                            font.family: "Noto Sans Mono"
                        }
                    }
                    ChromeIconToolButton {
                        Layout.alignment: Qt.AlignVCenter
                        implicitWidth: Theme.panelHeaderBtn
                        implicitHeight: Theme.panelHeaderBtn
                        icon.source: root.iconUrl("x")
                        icon.width: Theme.iconMd
                        icon.height: Theme.iconMd
                        Accessible.name: qsTr("Dismiss this message")
                        ThemedToolTip {
                            visible: parent.hovered
                            text: parent.Accessible.name
                        }
                        onClicked: AppSession.dismissNotice(toast.modelData.id)
                    }
                }

                // Errors do not fade. A save that did not happen must not
                // scroll past while the user is looking at the canvas — it is
                // dismissed deliberately or not at all.
                Timer {
                    running: toast.modelData.autoDismiss && !toastHover.hovered
                    interval: root.dwellMs
                    onTriggered: AppSession.dismissNotice(toast.modelData.id)
                }

                // The Repeater rebuilds its delegates whenever the projection
                // changes, so a new message restarts the dwell on the ones
                // already up. That is the behaviour wanted anyway: the stack is
                // current, and a toast should outlive the message after it.
                opacity: 0
                NumberAnimation on opacity {
                    to: 1
                    duration: Theme.reducedMotion ? 0 : 120
                    easing.type: Easing.OutCubic
                }
            }
        }
    }
}
