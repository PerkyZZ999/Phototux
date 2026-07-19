import QtQuick
import Qt5Compat.GraphicalEffects

/// Phosphor SVG (`fill="currentColor"`) tinted for dark chrome (WCAG non-text ≥ 3:1).
Item {
    id: root

    property url source
    property color color: Theme.iconOnSurface
    property int size: 18
    property alias asynchronous: img.asynchronous
    property alias mirror: img.mirror
    property alias fillMode: img.fillMode

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    Image {
        id: img
        anchors.fill: parent
        source: root.source
        sourceSize: Qt.size(root.size * 2, root.size * 2)
        fillMode: Image.PreserveAspectFit
        visible: false
        smooth: true
        mipmap: true
        asynchronous: true
    }

    ColorOverlay {
        anchors.fill: parent
        source: img
        color: root.color
        cached: true
    }
}
