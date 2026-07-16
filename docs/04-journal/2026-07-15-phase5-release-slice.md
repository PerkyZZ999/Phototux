# Phase 5 Release Slice Evidence — 2026-07-15

## Scope delivered

- PNG/JPEG decode and encode through `phototux_io`, with explicit dimension/allocation limits, EXIF orientation, JPEG alpha flattening, and atomic destination replacement.
- One-shot decoded raster upload and flattened composite readback at the ADR-015 file-operation boundary. Steady-state presentation remains zero-copy.
- Background Open/Export worker, busy/error state, document naming, dirty tracking, and guarded New/Open/Close/Quit actions.
- File/Edit/View/Help menus, standard shortcuts, Qt Quick native file dialogs, export-only wording, and desktop-open argument handling.
- XDG desktop entry, AppStream metadata, application icon, MIME associations, and Linux installation notes.

## Automated evidence

- `./scripts/check-rust.sh`: passed, including formatting, workspace tests, Clippy with warnings denied, and `rust-doctor`.
- `qmllint qml/Main.qml qml/NewDocumentDialog.qml`: passed without diagnostics.
- `desktop-file-validate packaging/linux/io.github.PerkyZZ999.PhotoTux.desktop`: passed.
- `appstreamcli validate --no-net packaging/linux/io.github.PerkyZZ999.PhotoTux.metainfo.xml`: passed at pedantic level.
- Raster tests cover PNG/JPEG round trips, decode limits, EXIF orientation, JPEG alpha flattening, malformed input, and atomic path export.
- GPU tests cover layer upload followed by composite readback.
- Engine tests cover creation of a named single-layer flattened document.

## Manual Wayland evidence

Validated with a release build in an isolated KWin Wayland session on Intel Arc B580:

- Desktop-open PNG decoded and displayed with correct top/bottom orientation.
- Brush stroke drawn near the document's top edge remained aligned with pointer input after the imported-texture UV correction.
- Stroke set the dirty state and displayed the unsaved indicator.
- New-document shortcut opened the discard confirmation; Discard continued to the New Document dialog.
- Shell reported 60 FPS during the sampled idle and brush run.

## Startup measurement

Instrumentation starts at the first statement in `main` and stops on the first `FrameAnimation` callback after the `ApplicationWindow` becomes visible.

- Build: `cargo build --release -p phototux`
- Environment: isolated KWin Wayland session, Intel Arc B580, Mesa 26.1.4
- Canvas type registration: 0.03 ms
- Shared wgpu/Vulkan device ready: 94.44 ms
- First interactive frame samples: 653.71 ms and 780.17 ms
- ADR-008 target: under 250 ms

Result: target not met. B3 blocks Phase 5 exit. The dominant cost follows GPU initialization and includes QML loading, object creation, and first Qt Quick frame. Next pass should package QML in Qt resources with `qmlcachegen` ahead-of-time units, then profile remaining first-frame work.

## Outcome

Release-slice functionality is implemented and quality gates pass. Phase 5 remains open solely on startup SLO B3; no ADR-008 exception is claimed.
