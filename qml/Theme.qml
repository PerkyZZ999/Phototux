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
    // Borders ≥ 3:1 vs surface / surfaceRaised (WCAG 2.1 non-text UI).
    readonly property color border: "#7B7B81"
    readonly property color borderSubtle: "#7B7B81"
    // Text ≥ 4.5:1 vs surface / background (WCAG 2.1 AA).
    readonly property color colorOnSurface: "#EFF0F1"
    readonly property color colorOnSurfaceMuted: "#A8A8B0"
    readonly property color colorOnSurfaceVariant: "#C4CCD4"
    readonly property color colorOnSurfaceDisabled: "#9A9AA3"
    readonly property color focusRing: "#3DAEE9"
    readonly property color success: "#2ECC71"
    readonly property color warning: "#FF9F1A"
    readonly property color error: "#DA4453"
    readonly property color selection: "#3DAEE933"
    readonly property color canvasLetterbox: "#0C0C0E"
    readonly property color toolActiveBg: "#3DAEE940"
    /// Modal overlay scrim (welcome / dialogs).
    readonly property color scrimModal: "#000000B8"
    /// Soft primary wash for logo wells and selected chrome accents.
    readonly property color primarySubtle: "#3DAEE91A"
    /// Inactive document tab fill (keeps dark shell coherent).
    readonly property color tabInactive: "#1A1A1E"
    /// Soft success wash for status chips (GPU path healthy).
    readonly property color successSubtle: "#2ECC712E"
    /// Symbolic icons on dark chrome (white; ≥ 3:1 non-text).
    readonly property color iconOnSurface: "#FFFFFF"
    readonly property color iconDisabled: "#9A9AA3"

    // Corner radii are a fixed visual signature and do not scale with density.
    readonly property int radiusXs: 2
    readonly property int radiusSm: 4
    readonly property int radiusMd: 6
    readonly property int radiusLg: 8

    // Spacing, control heights, and hit targets scale with density. Without
    // this, "comfortable" only enlarged text and left the shell just as tight,
    // which defeats the preference and the 200%-scale accessibility target.
    readonly property int spaceXxs: Math.round(2 * densityScale)
    readonly property int spaceXs: Math.round(4 * densityScale)
    readonly property int spaceSm: Math.round(8 * densityScale)
    readonly property int spaceMd: Math.round(12 * densityScale)
    readonly property int spaceLg: Math.round(16 * densityScale)
    readonly property int spaceXl: Math.round(24 * densityScale)

    readonly property int toolStripWidth: Math.round(48 * densityScale)
    readonly property int dockWidth: Math.round(280 * densityScale)
    readonly property int toolbarHeight: Math.round(40 * densityScale)
    readonly property int statusbarHeight: Math.round(28 * densityScale)
    readonly property int panelHeaderHeight: Math.round(28 * densityScale)
    readonly property int controlHeight: Math.round(28 * densityScale)
    readonly property int toolHit: Math.round(40 * densityScale)
    /// Panel-header Phosphor glyphs (uniform optical box).
    readonly property int iconMd: Math.round(16 * densityScale)
    readonly property int panelHeaderBtn: Math.round(24 * densityScale)

    // Density / a11y packs (prefs → AppSession); Theme remains single token source.
    property bool highContrast: false
    property bool reducedMotion: false
    property string uiDensity: "dense"
    readonly property real densityScale: uiDensity === "comfortable" ? 1.15 : 1.0
    readonly property color borderEffective: highContrast ? "#B0B0BA" : border
    readonly property color colorOnSurfaceEffective: highContrast ? "#FFFFFF" : colorOnSurface
    readonly property color iconOnSurfaceEffective: highContrast ? "#FFFFFF" : iconOnSurface
    readonly property color iconDisabledEffective: highContrast ? "#C0C0C8" : iconDisabled

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
