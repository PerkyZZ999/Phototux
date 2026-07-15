# Journal: Phase 1 complete — 2026-07-15

## Delivered

- Cargo workspace: `phototux`, `phototux_ui`, `phototux_engine` (GPL-3.0-or-later)
- Engine: size presets 720p/1080p/2K/4K, session zoom/brush/tool, unit tests (5)
- qtbridge `AppSession` singleton + properties/slots
- QML multi-pane shell (toolbar, tool strip, canvas placeholder, properties, layers stub, status)
- `NewDocumentDialog` presets + custom W×H
- Phosphor icons via absolute `iconRoot` path
- Quality: rustfmt, clippy `-D warnings`, rust-doctor (score 99, fail-on error)

## Dev run

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
cargo run -p phototux
```

QML loads from filesystem (`qml/Main.qml` + companions). Icons require source-tree path to `assets/icons/phosphor/regular`.

## Mockups

`docs/design_mockup/` used as **visual inspiration only** — not locked into DESIGN/IA.

## Next

Phase 1.5 interop spike (ADR-010) before GPU viewport production work.
