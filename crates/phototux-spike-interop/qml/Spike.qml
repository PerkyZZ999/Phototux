import QtQuick
import QtQuick.Controls
import PhototuxSpike 1.0
import phototux_spike_interop

ApplicationWindow {
    id: root
    visible: true
    width: 960
    height: 640
    title: qsTr("PhotoTux Spike — wgpu / Qt RHI interop")
    color: "#121214"

    // SpikeStatus comes from qtbridge package name = phototux_spike_interop
    // PhototuxSpike.SpikeCanvas is the C++ QQuickRhiItem

    Column {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        Label {
            width: parent.width
            wrapMode: Text.WordWrap
            color: "#EFF0F1"
            font.pixelSize: 13
            font.bold: true
            text: qsTr("Phase 1.5 interop spike (Intel Arc / Xe)")
        }

        Label {
            width: parent.width
            wrapMode: Text.WrapAnywhere
            color: "#A0A0A8"
            font.pixelSize: 11
            text: SpikeStatus.gpuText
        }

        Label {
            width: parent.width
            wrapMode: Text.WordWrap
            color: "#A0A0A8"
            font.pixelSize: 11
            text: SpikeStatus.rhiNote
        }

        SpikeCanvas {
            id: canvas
            width: parent.width
            height: 420
            phase: SpikeStatus.phase
        }

        Label {
            color: "#3DAEE9"
            font.pixelSize: 11
            text: qsTr("Animated panel = Qt RHI GPU clear (hybrid C++). wgpu probe text above.")
        }
    }

    Timer {
        interval: 16
        running: true
        repeat: true
        onTriggered: SpikeStatus.tick(0.05)
    }

    Component.onCompleted: SpikeStatus.refreshGpuText()
}
