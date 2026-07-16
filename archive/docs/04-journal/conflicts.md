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

## 2026-07-16 — Preferred IA multi-doc vs ADR-013

- **IA / PREFERED_IA require:** Document tabs, multi-document workspace, multi-image editing.
- **ADR-013 G14:** Single document only for v1.
- **Resolution:** Keep multi-doc as `[!]` / **Blocked** in IA + checklist until ADR-013 is amended. Do not implement tabs silently.

## 2026-07-16 — Preferred IA Shape layers vs ADR-017

- **IA requires:** First-class Shape layers + shape tools.
- **ADR-017:** Graph kind set does not include Shape (paths may land as vector data without a Shape kind).
- **Resolution:** Ship Paths engine against raster stroke/fill first if needed; Shape layers stay `[!]` until ADR-017 kind amendment.

## 2026-07-16 — Preferred IA plugins / automation product surface

- **IA lists:** Plugin Manager, Scripts, Actions as application modules.
- **ADRs:** New major subsystem requires an ADR; ADR-014 forbids non-desktop product surfaces.
- **Resolution:** Track as `[P]` / Deferred in checklist; no plugin store UI until a dedicated ADR.
