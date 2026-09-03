import QtQuick

/// Defers building a dialog's object tree until the dialog is first needed.
///
/// Dialogs are the bulk of the shell's QML object graph, but a typical session
/// never opens most of them, so constructing them during startup is pure
/// cold-boot cost against the DR-017 interactive gate. Once loaded the instance
/// is retained, so reopening is immediate and any in-progress state survives a
/// close.
///
/// The wrapped dialog **must** set `parent: Overlay.overlay` — a `Popup`
/// declared inside a `Loader` would otherwise center itself on the zero-sized
/// loader rather than on the window.
Loader {
    id: root

    /// The wrapped dialog, as the default property.
    ///
    /// This declaration is load-bearing. `Loader` inherits its default property
    /// from `Item`, so without it a dialog written inside `LazyDialog { … }`
    /// became an ordinary child object: `sourceComponent` stayed null, the
    /// Loader loaded nothing, and `item` was null forever. Dialogs driven by a
    /// `visible:` binding still worked — they existed as children, just eagerly
    /// rather than lazily, so the deferral this type exists for never happened.
    /// The command palette, which reaches its API through `ensure()`, got null
    /// and silently failed to open.
    ///
    /// Declaring the property as `Component` makes QML wrap the object
    /// declaration for us, so call sites are unchanged.
    default property Component dialog

    /// Bind to host state for dialogs that open declaratively. Latches the
    /// dialog into existence; the wrapped dialog's own `visible` binding then
    /// governs show and hide.
    property bool requested: false

    /// Mirrors the wrapped dialog's visibility, or false before first load.
    readonly property bool dialogVisible: root.item ? root.item.visible : false

    sourceComponent: root.dialog
    active: false
    // Synchronous so `open()` can drive the freshly created dialog immediately.
    asynchronous: false

    onRequestedChanged: if (root.requested) root.active = true

    function open() {
        root.active = true
        if (root.item)
            root.item.open()
    }

    function close() {
        if (root.item)
            root.item.close()
    }

    /// Force the dialog into existence and hand it back, for callers that drive
    /// dialog-specific API rather than plain `open()`.
    function ensure() {
        root.active = true
        return root.item
    }
}
