pragma Singleton
import QtQuick

// Chrome tokens (handbook Phase 3) — dense KDE Plasma–aligned; values from handbook Themes.
QtObject {
    readonly property color primary: "#3DAEE9"
    readonly property color primaryHover: "#5CB8ED"
    readonly property color primaryOn: "#0A1620"
    readonly property color secondary: "#A0A0A8"
    readonly property color tertiary: "#F67400"
    readonly property color neutral: "#1E1E22"
    readonly property color background: "#131317"
    readonly property color surface: "#2B2B30"
    readonly property color surfaceRaised: "#323238"
    readonly property color surfaceSunken: "#121214"
    readonly property color surfaceOverlay: "#232328"
    readonly property color surfaceContainer: "#1F1F23"
    readonly property color surfaceContainerHigh: "#2A2A2E"
    readonly property color border: "#3D3D45"
    readonly property color borderSubtle: "#2F2F36"
    readonly property color colorOnSurface: "#EFF0F1"
    readonly property color colorOnSurfaceMuted: "#A0A0A8"
    readonly property color colorOnSurfaceVariant: "#BEC8D1"
    readonly property color colorOnSurfaceDisabled: "#9A9AA3"
    readonly property color focusRing: "#3DAEE9"
    readonly property color success: "#2ECC71"
    readonly property color warning: "#FF9F1A"
    readonly property color error: "#DA4453"
    readonly property color selection: "#3DAEE933"
    readonly property color canvasLetterbox: "#0C0C0E"
    readonly property color toolActiveBg: "#3DAEE940"

    readonly property int radiusXs: 2
    readonly property int radiusSm: 4
    readonly property int radiusMd: 6
    readonly property int radiusLg: 8

    readonly property int spaceXxs: 2
    readonly property int spaceXs: 4
    readonly property int spaceSm: 8
    readonly property int spaceMd: 12
    readonly property int spaceLg: 16
    readonly property int spaceXl: 24

    readonly property int toolStripWidth: 48
    readonly property int dockWidth: 280
    readonly property int toolbarHeight: 40
    readonly property int statusbarHeight: 28
    readonly property int panelHeaderHeight: 28
    readonly property int controlHeight: 28
    readonly property int toolHit: 40

    // Density / a11y packs (prefs → AppSession); Theme remains single token source.
    property bool highContrast: false
    property bool reducedMotion: false
    property string uiDensity: "dense"
    readonly property real densityScale: uiDensity === "comfortable" ? 1.15 : 1.0
    readonly property color borderEffective: highContrast ? "#8A8A96" : border
    readonly property color colorOnSurfaceEffective: highContrast ? "#FFFFFF" : colorOnSurface

    readonly property int fontWindow: Math.round(13 * densityScale)
    readonly property int fontHeadline: Math.round(16 * densityScale)
    readonly property int fontHeadlineSm: Math.round(13 * densityScale)
    readonly property int fontBody: Math.round(12 * densityScale)
    readonly property int fontBodySm: Math.round(11 * densityScale)
    readonly property int fontLabel: Math.round(11 * densityScale)
    readonly property int fontLabelSm: Math.round(10 * densityScale)
    readonly property int fontMono: Math.round(11 * densityScale)

    readonly property url logoUrl: "qrc:/qt/qml/PhotoTux/App/logo-ui.png"

    function iconUrl(iconRoot, stem) {
        if (!iconRoot || iconRoot.length === 0)
            return ""
        if (iconRoot.indexOf("qrc:") === 0)
            return iconRoot + "/" + stem + ".svg"
        if (iconRoot.charAt(0) === "/")
            return "file://" + iconRoot + "/" + stem + ".svg"
        return "file:///" + iconRoot + "/" + stem + ".svg"
    }
}
