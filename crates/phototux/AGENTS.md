# phototux

Desktop binary and QML AOT composition root (`cargo run -p phototux`).

- Depends on `phototux_ui` and `phototux_canvas` only at this layer.
- New QML files under `qml/` are globbed by the AOT module. `pragma Singleton` types must also be listed in `QML_SINGLETONS`.
- Qt 6 on `PATH` is required to link.
