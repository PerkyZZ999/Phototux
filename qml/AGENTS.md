# qml/

Qt Quick shell. Tokens come from `Theme.qml` and handbook [25 — Themes](../internal_docs/25-Themes.md).

- Dense editor chrome. Canvas-first. No second palette or spacing scale.
- Icons: `assets/icons/phosphor/` via [`assets/icons/ICON_MAP.md`](../assets/icons/ICON_MAP.md).
- `import phototux_ui` for `AppSession`. `qmllint` unresolved-import on that module is expected.
- User-facing strings: `qsTr(...)`.
