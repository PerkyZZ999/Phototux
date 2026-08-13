# qml/

Qt Quick shell. Tokens come from `Theme.qml` and handbook [25 — Themes](../internal_docs/25-Themes.md).

- Dense editor chrome. Canvas-first. No second palette or spacing scale.
- Icons: `assets/icons/phosphor/` via [`assets/icons/ICON_MAP.md`](../assets/icons/ICON_MAP.md).
- `import phototux_ui` for `AppSession`. `qmllint` unresolved-import on that module is expected.
- User-facing strings: `qsTr(...)`.
- **Never call an `AppSession` slot synchronously from a handler that reacts to an `AppSession` signal** — `Connections`, binding-driven `on…Changed`, focus/popup callbacks, anything a `Loader` builds from host state. The slot that emitted still holds the session borrowed, so a re-entrant call aborts the process. Route it through `root.afterHostSlot(fn)`. Direct calls from user input (`onClicked`, `Keys.on…`) are fine. Handbook [32 — Host Slot Re-entrancy](../internal_docs/32-Developer-Guide.md#host-slot-re-entrancy).
