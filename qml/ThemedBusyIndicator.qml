import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import phototux_ui

/// A busy spinner drawn from `Theme.qml` rather than by the Controls style.
///
/// The status bar's "Working…" indicator was a bare `BusyIndicator`, which the
/// Basic style draws with `pen: control.palette.dark` — a dark grey on a
/// palette that assumes a light window. On this editor's dark chrome that is a
/// smudge you have to know is there. Basic also gives the control `padding: 6`
/// around a contentItem whose implicit size is 48, so squeezing it into the
/// eighteen pixels the status bar allows left about six pixels of actual
/// spinner.
///
/// Breeze spins a partial ring, which is what this draws: one arc, the accent
/// colour, rotating once a second. `padding: 0` because the caller sizes it.
BusyIndicator {
    id: control

    padding: 0
    /// A spinner is a picture of the status text beside it. Announcing it as
    /// well would have assistive technology read the same state twice, and
    /// "busy indicator" is not the half a user needs.
    Accessible.ignored: true

    contentItem: Item {
        implicitWidth: 16
        implicitHeight: 16

        Shape {
            id: ring
            anchors.fill: parent
            // The spinner is chrome, not canvas: smoothing it costs a
            // multisampled buffer for sixteen pixels of arc.
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: Theme.primary
                strokeWidth: 2
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap

                PathAngleArc {
                    centerX: ring.width / 2
                    centerY: ring.height / 2
                    radiusX: Math.max(ring.width / 2 - 2, 1)
                    radiusY: Math.max(ring.height / 2 - 2, 1)
                    startAngle: 0
                    // Three quarters, so the gap reads as motion. A full ring
                    // spins invisibly.
                    sweepAngle: 270
                }
            }

            // Stops spinning under the accessibility preference, and stays on
            // screen: the ring's *presence* is what says work is in progress,
            // and a perpetual rotation is exactly the motion the preference is
            // asked about. Everything else that animates in the shell is a
            // brief transition; this one never ends on its own.
            RotationAnimator on rotation {
                running: control.running && control.visible && !Theme.reducedMotion
                from: 0
                to: 360
                duration: 1000
                loops: Animation.Infinite
            }
        }
    }
}
