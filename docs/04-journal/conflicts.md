# Conflict Log

## 2026-07-15 — Phase 2/4 acceptance vs sampled presentation

- **Docs claimed:** Phase 2 zero-copy present and Phase 4 visual brush acceptance were complete.
- **Code shows:** `PhototuxCanvasRenderer` imports only a raw `VkImage` attempt, deletes the wrapper, binds no sampled texture, and renders a procedural tint.
- **Governing decisions:** ADR-005 requires the interactive canvas to display the GPU document through a zero-copy path; ADR-008 gates visible viewport and brush behavior.
- **Resolution:** Reopen Phase 2 and Phase 4 acceptance under blocker B2. Close only after real composite pixels are sampled with no steady-state CPU upload and the prior FPS/latency gates are rechecked.
- **Closed:** 2026-07-15. Same-device Vulkan sampling is live; visible brush/undo and 60 FPS were verified. Release latency remains a Phase 5 final gate.

## 2026-07-15 — Phase 5 startup gate vs measured native startup

- **Docs required:** ADR-008, SPEC, constraints, README, and AGENTS set a <250 ms cold-boot Phase 5 target and prohibited silently loosening it.
- **Measurement shows:** Statically linked QML AOT and embedded startup icons reached a 685.94 ms 10-run median and 648.17 ms best first-interactive frame on the reference host.
- **Owner direction:** Accept Phase 5 when the optimized benchmark is under 1,000 ms; continue treating <250 ms as a stretch target.
- **Resolution:** Amend ADR-008 explicitly, align current normative docs, preserve the original target as stretch, and close B3 from measured median evidence.

## 2026-07-15 — QML AOT registration vs canvas-only C++

- **ADR-003 required:** No C++ outside canvas interop without amendment.
- **Qt requires:** Supported static QML AOT integration generates C++ and needs a `Q_IMPORT_QML_PLUGIN` link anchor.
- **Resolution:** Amend ADR-003 for Qt-generated cache/plugin code plus one argument-free registration anchor under `crates/phototux/qml-aot/`. No application logic moved to C++.
