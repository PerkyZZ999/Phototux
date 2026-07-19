# Former ADR → Decision Register Map

| Field | Value |
| --- | --- |
| Status | Accepted (hygiene) |
| Date | 2026-07-16 (files removed 2026-07-18) |
| Live index | [Decision-Register.md](Decision-Register.md) |
| Source files | Removed — former `/docs/01-decisions/` then `archive/docs/01-decisions/`; this map is the remaining index |

Historical ADR ids are **not** a second authority. Prefer Decision Register (DR) status. New architecture locks amend the Decision Register (and Alignment Roadmap when stack-affecting).

| Former ADR | Topic | Live DR / status |
| --- | --- | --- |
| ADR-001 | Linux / Wayland platform | [DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase) (stack); platform spirit in DR-007 |
| ADR-002 | Qt 6 QML UI | DR-023; [DR-008](Decision-Register.md#dr-008--ui-toolkit-and-application-runtime-deferred) **Superseded** |
| ADR-003 | qtbridge FFI + thin C++ canvas | DR-023 / DR-007 host adapters |
| ADR-004 | wgpu / Vulkan-first | DR-023 / [DR-006](Decision-Register.md#dr-006--gpu-first-via-wgpu-not-gpu-only) |
| ADR-005 | Zero-copy present | DR-023 |
| ADR-006 | Workspace crate layout | [DR-025](Decision-Register.md#dr-025--crate-topology-coarse-workspace) |
| ADR-007 | Threading / command queue | [DR-010](Decision-Register.md#dr-010--per-document-mutation-serialization); paint queue remains host/engine |
| ADR-008 | Performance SLOs | [DR-017](Decision-Register.md#dr-017--performance-budgets-provisional) Provisional until fixtures |
| ADR-009 | Testing / profiling | [DR-022](Decision-Register.md#dr-022--headless-testability-of-core) |
| ADR-010 | Interop spike | Evidence only; production canvas shipped on stack |
| ADR-011 | Document model timing | [DR-002](Decision-Register.md) / graph chapters; graph v2 in engine |
| ADR-012 | License GPL-3.0-or-later | DR-023 workspace license |
| ADR-013 | Product prefs / single doc / icons | [DR-024](Decision-Register.md#dr-024--document-session-model) (v2 tabs); prefs in ch.24 |
| ADR-014 | Desktop GUI only | DR-023 product surface |
| ADR-015 | Raster I/O boundary | [DR-013](Decision-Register.md#dr-013--native-format-vs-interchange-adapters) / ch.22 |
| ADR-016 | Native `.ptx` | [DR-026](Decision-Register.md#dr-026--native-ptx-container-v1) (v2 write / v1 read) |
| ADR-017 | Graph v2 + history | Document/history chapters; [DR-004](Decision-Register.md); Shape via [DR-027](Decision-Register.md#dr-027--graph-kind-set-includes-shape) |
| ADR-018 | PSD interchange subset | DR-013 adapters + ch.22/27 |

**Rule for agents:** implement against handbook + Decision Register. Cite former ADR ids only via this map, never as a second authoritative MUST set.
