# phototux_canvas

Qt RHI ↔ wgpu canvas interop. Thin C++ lives here (and QML AOT in `phototux`).

- `unsafe` only at this FFI boundary. Each block: one-line `// SAFETY:` invariant.
- Do not spread handwritten C++ into other crates.
- Build needs Qt 6 on `PATH` (`QMAKE=/usr/lib/qt6/bin/qmake`).
