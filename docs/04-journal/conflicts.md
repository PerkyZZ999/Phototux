# Conflict Log

## 2026-07-15 — Phase 2/4 acceptance vs sampled presentation

- **Docs claimed:** Phase 2 zero-copy present and Phase 4 visual brush acceptance were complete.
- **Code shows:** `PhototuxCanvasRenderer` imports only a raw `VkImage` attempt, deletes the wrapper, binds no sampled texture, and renders a procedural tint.
- **Governing decisions:** ADR-005 requires the interactive canvas to display the GPU document through a zero-copy path; ADR-008 gates visible viewport and brush behavior.
- **Resolution:** Reopen Phase 2 and Phase 4 acceptance under blocker B2. Close only after real composite pixels are sampled with no steady-state CPU upload and the prior FPS/latency gates are rechecked.
- **Closed:** 2026-07-15. Same-device Vulkan sampling is live; visible brush/undo and 60 FPS were verified. Release latency remains a Phase 5 final gate.
