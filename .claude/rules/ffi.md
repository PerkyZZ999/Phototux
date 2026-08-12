---
paths:
  - "crates/phototux-canvas/**"
  - "crates/phototux/qml-aot/**"
  - "**/*.cpp"
  - "**/*.h"
---

# FFI / canvas C++

- Handwritten C++ stays in `phototux_canvas` and the QML AOT anchor. Do not spread it.
- `unsafe` Rust at this boundary only; each block states the invariant in one `// SAFETY:` line.
- Qt 6 headers: `PATH`/`QMAKE` → `/usr/lib/qt6/bin`.
