# Phase 5 Release Slice Evidence — 2026-07-15

## Scope delivered

- PNG/JPEG decode and encode through `phototux_io`, with explicit dimension/allocation limits, EXIF orientation, JPEG alpha flattening, and atomic destination replacement.
- One-shot decoded raster upload and flattened composite readback at the ADR-015 file-operation boundary. Steady-state presentation remains zero-copy.
- Background Open/Export worker, busy/error state, document naming, dirty tracking, and guarded New/Open/Close/Quit actions.
- File/Edit/View/Help menus, standard shortcuts, Qt Quick native file dialogs, export-only wording, and desktop-open argument handling.
- XDG desktop entry, AppStream metadata, application icon, MIME associations, and Linux installation notes.
- Qt-supported `qmlcachegen` AOT compilation through a statically linked CMake QML module. Release startup uses embedded `qrc:` QML and startup icons; `PHOTOTUX_QML` remains a filesystem diagnostic override.

## Automated evidence

- `./scripts/check-rust.sh`: passed, including formatting, workspace tests, Clippy with warnings denied, and `rust-doctor`.
- `qmllint qml/Main.qml qml/NewDocumentDialog.qml`: passed without diagnostics.
- `desktop-file-validate packaging/linux/io.github.PerkyZZ999.PhotoTux.desktop`: passed.
- `appstreamcli validate --no-net packaging/linux/io.github.PerkyZZ999.PhotoTux.metainfo.xml`: passed at pedantic level.
- Raster tests cover PNG/JPEG round trips, decode limits, EXIF orientation, JPEG alpha flattening, malformed input, and atomic path export.
- GPU tests cover layer upload followed by composite readback.
- Engine tests cover creation of a named single-layer flattened document.
- Default release startup and `PHOTOTUX_QML` override both reached the first interactive frame without QML load errors.

## Manual Wayland evidence

Validated with a release build in an isolated KWin Wayland session on Intel Arc B580:

- Desktop-open PNG decoded and displayed with correct top/bottom orientation.
- Brush stroke drawn near the document's top edge remained aligned with pointer input after the imported-texture UV correction.
- Stroke set the dirty state and displayed the unsaved indicator.
- New-document shortcut opened the discard confirmation; Discard continued to the New Document dialog.
- Shell reported 60 FPS during the sampled idle and brush run.

Follow-up validation used `computer-use-linux` on the host Plasma Wayland session:

- Embedded-QML release shell exposed the expected menu, toolbar, properties, layers, and New Document controls.
- Creating the default document produced a 1920×1080, two-layer, zoom-to-fit document and reported the shared-Vulkan wgpu composite.
- Open invoked the native `Open Image` file dialog.

## Startup measurement

Instrumentation starts at the first statement in `main` and stops on the first `FrameAnimation` callback after the `ApplicationWindow` becomes visible.

- Build: `cargo build --release -p phototux`
- Environment: host Plasma Wayland session, Intel Arc B580, Mesa 26.1.4
- Pre-AOT 10-run baseline: 680.52 ms median; 672.79–731.48 ms
- Embedded QML AOT 10-run series: 678.71 ms median; 662.75–702.36 ms
- Embedded QML AOT + startup icons 10-run series: 685.94 ms median; **648.17 ms best**, 706.10 ms max
- QML root-load median improved from 504.32 ms to 484.27 ms after embedding startup icons.
- A later accepted-code host-contention series measured 738.03 ms median with two outliers above one second; the median remained below the gate.
- ADR-008 amended Phase 5 gate: under 1,000 ms median; under 250 ms retained as a stretch target.

Result: Phase 5 gate met. Dominant residual cost is Qt Quick object creation plus shared Vulkan RHI setup. Deferring file dialogs and overlapping Qt/wgpu initialization regressed sampled startup, so both experiments were reverted.

## Outcome

Release-slice functionality is implemented, quality gates pass, B3 is resolved, and Phase 5 is closed. The <250 ms objective remains tracked as a stretch target rather than an exit blocker.
