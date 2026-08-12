---
paths:
  - "qml/**/*.qml"
---

# QML

- Tokens from `qml/Theme.qml` and handbook 25 — Themes. Do not invent a second palette or spacing scale.
- Dense editor chrome, canvas-first. Motion is for docks/structure, never paint delay.
- Icons from `assets/icons/phosphor/` via `assets/icons/ICON_MAP.md`.
- `import phototux_ui` for `AppSession`. User-facing strings: `qsTr(...)`.
- New `pragma Singleton` types must be listed in `QML_SINGLETONS` (AOT module).
