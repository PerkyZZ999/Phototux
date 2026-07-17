# Decision Register

## Purpose

Register of high-reversal-cost architectural decisions for PhotoTux. Entries summarize context, options, decision, consequences, evidence status, and revisit triggers. Detailed prose lives in numbered specs; this register is the index and change log. Normative keywords follow [Requirement Keywords](Requirement-Keywords.md).

Status values:

| Status | Meaning |
| --- | --- |
| `Accepted` | Binding for conforming implementations |
| `Provisional` | Direction set; thresholds/encoding still evidence-gated |
| `Deferred` | Explicitly not chosen yet; seams only |
| `Superseded` | Replaced by another entry |

## DR-001 — Local-first product boundary

| Field | Content |
| --- | --- |
| Status | Accepted |
| Date | Handbook foundation |
| Decs | [00](../00-Introduction.md) |
| Context | Professional raster editing can drift into cloud sync, accounts, and generative services. |
| Decision | PhotoTux is local-first. Cloud storage/sync/collaboration, accounts, telemetry-dependent features, AI/generative tools, and proprietary service integrations are out of product boundary. |
| Consequences | No network required for normal editing; extensions cannot require accounts; diagnostics stay local unless user exports. |
| Revisit | Only via charter amendment. |

## DR-002 — Document owns truth

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 00, [10](../10-Document-Model.md) |
| Decision | Authoritative editable state belongs to the document model. Views, panels, render caches, and GPU resources are projections. |
| Consequences | UI cannot be the undo stack; renderer cannot commit pixels as truth. |
| Revisit | Never without replacing core architecture. |

## DR-003 — Commands are mutation spine

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 00, [08](../08-Command-System.md) |
| Alternatives considered | Direct view-model mutation; widget callbacks writing models |
| Decision | Every user-visible semantic mutation enters through a named command with validation, transaction, and typed results. |
| Consequences | More ceremony; unified undo, concurrency, a11y actions, plugins, headless tests. |
| Revisit | If measured command overhead breaks brush budgets after optimization—adjust batching, not bypass. |

## DR-004 — History stores transactions

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | [20](../20-History-Undo.md) |
| Alternatives | Whole-document snapshots only; UI event journals |
| Decision | Undo/redo operate on committed transactions (with optional checkpoints). Versions remain monotonic. |
| Consequences | Precise invalidation; complex inverses; budgeted retention. |

## DR-005 — Immutable render snapshots

| Field | Content |
| --- | --- |
| Status | **Accepted** (v1 lease); full pixel-immutable snapshots **Provisional** |
| Docs | [17](../17-Rendering-Engine.md), 10, [Alignment Roadmap](Alignment-Roadmap.md) |
| Decision | Render workers consume versioned immutable snapshots and bounded deltas; they MUST NOT mutate authoritative documents. |
| Accepted v1 (shipping) | Document **generation** counters plus metadata **`DocumentSnapshotLease`** (and `mark_persisted`) are the authoritative stale-result / cache-key contract. Workers and GPU paths key off generation; they do not hold mutable graph refs across async work. |
| Provisional (later) | Full immutable pixel snapshot blobs, dense delta streams, and tile/pyramid publishers remain target architecture (Phase 5 / [DR-006](#dr-006--gpu-first-via-wgpu-not-gpu-only) evidence). |
| Consequences | Snapshot publisher required at the semantic level; stale result policy required; GPU caches keyed by full semantic inputs (generation + params). Do not rewrite interactive present for paper-pure pixel clones until evidence demands. |

## DR-006 — GPU-first via wgpu, not GPU-only

| Field | Content |
| --- | --- |
| Status | Accepted (engine); Provisional (tile size / scheduling knobs) |
| Docs | 00, 17, [30](../30-Performance.md) |
| Alternatives | CPU-first core; vendor-specific APIs only |
| Decision | wgpu is primary rendering/compute abstraction; CPU reference/fallback remains mandatory for correctness and unsupported paths. |
| Consequences | Device loss paths; feature tiers; tolerance fixtures. |
| Evidence needed | Representative workloads before freezing tile size and submission policy. |

## DR-007 — Portable core, native Linux edges

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 00, [02](../02-Application-Lifecycle.md), 29 |
| Alternatives | Cross-platform application shell as source of truth |
| Decision | Semantic core is portable; Linux host adapters own surfaces, portals, clipboard, AT-SPI, session, tablet, themes. Toolkit objects terminate at host/presentation boundary. |
| Consequences | Dual maintenance of adapters later; higher Linux quality bar now. |
| Host presentation | Qt 6 QML + qtbridge on Linux ([DR-023](#dr-023--tech-stack-frozen-to-shipping-codebase)); portable core still must not import toolkit types. |

## DR-008 — UI toolkit and application runtime deferred

| Field | Content |
| --- | --- |
| Status | **Superseded** by [DR-023](#dr-023--tech-stack-frozen-to-shipping-codebase) |
| Docs | 00, [32](../32-Developer-Guide.md) |
| Former decision | Toolkit/runtime left open until spikes. |
| Supersession | Shipping stack chose Qt 6 + qtbridge; no alternate toolkit. Async runtime remains non-mandated (workers/channels OK). |
| Revisit | Only if Qt/qtbridge blocks zero-copy or a11y with no viable fix (catastrophic). |

## DR-009 — Plugin ABI deferred; capability seams now

| Field | Content |
| --- | --- |
| Status | Deferred (ABI); Accepted (seams) |
| Docs | [23](../23-Plugin-SDK.md), 08 |
| Alternatives | Stable C ABI now; in-process unrestricted plugins |
| Decision | Define manifests, capabilities, budgets, contribution types, and failure isolation now. Freeze out-of-process protocol / Wasm / C ABI only after validation. |
| Consequences | No third-party binary compatibility promise yet; documents must preserve opaque extension data. |

## DR-010 — Per-document mutation serialization

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 08, [Thread Ownership Map](Thread-Ownership-Map.md) |
| Alternatives | Global application lock |
| Decision | Conflicting authoritative mutations serialize per document (or equivalent conflict-safe model). |
| Consequences | Imports/filters on one doc should not stall others; cross-doc exclusive ops explicit. |

## DR-011 — Selection concepts are distinct

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | [01](../01-Information-Architecture.md), [12](../12-Selection-System.md) |
| Decision | Object selection, pixel selection, focus, context target, and active edit target are distinct concepts with explicit commands/announcements. |
| Consequences | Richer a11y and tools; forbids “whatever was last clicked” implicit surfaces. |

## DR-012 — Assign profile ≠ convert profile

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | [16](../16-Color-Management.md) |
| Decision | Assigning changes interpretation; converting changes pixels. Separate commands and disclosures. |
| Consequences | Prevents silent destructive color changes. |

## DR-013 — Native format vs interchange adapters

| Field | Content |
| --- | --- |
| Status | Accepted (policy); encoding detail in [DR-026](#dr-026--native-ptx-container-v1) |
| Docs | [27](../27-File-Formats.md), [22](../22-Import-Export.md) |
| Decision | Native versioned container is editable persistence authority. Third-party formats are adapters with loss disclosure. |
| Encoding | **`.ptx` v2 write / v1 read** — writers emit format version 2 typed chunks (`MANI` / `RASL` / `MASK` + whole-body CRC32); readers still open v1 monolithic bodies ([DR-026](#dr-026--native-ptx-container-v1)). Product extension stays `.ptx`. |
| Evidence needed | Huge sparse, incremental save, recovery, unknown preserve (guides evolution, not a greenfield format). |

## DR-014 — Staged save and atomic replace

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 27, 02, 10 |
| Decision | Saves write staged complete generation, verify, then atomically replace where FS supports; modified clears only if persisted identity matches current. |
| Consequences | Disk use during save; read-back verification cost. |

## DR-015 — Workspace state separate from documents

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | [03](../03-Workspace-System.md), [24](../24-Preferences.md) |
| Decision | Workspace/layout/view presentation persist separately from editable documents by default. |
| Accepted v1 (shipping) | Panel/tool **descriptors** in `phototux_engine::shell` are the semantic catalog; XDG prefs + Reset Essentials persist panel visibility / last tool; Qt QML hardcodes menus, shortcuts, and context menus. Full `WorkspaceTransaction` topology, tear-off docking, and action-driven chrome remain **target** (not v1 blockers). |
| Consequences | Closing views ≠ closing documents; restore is reconciliation. Agents MUST NOT treat hardcoded QML chrome as a contract violation of this DR while v1 stands. |

## DR-016 — Accessibility is semantic, not pixel inference

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | [29](../29-Accessibility.md) |
| Decision | Assistive technology sees projected semantic trees from descriptors/commands; canvas has structured summary/explorer; AT-SPI adapter is host-owned. |
| Consequences | Extra projection work; flood control mandatory. |

## DR-017 — Performance budgets provisional

| Field | Content |
| --- | --- |
| Status | Provisional |
| Docs | [30](../30-Performance.md), [Performance Budget Ledger](Performance-Budget-Ledger.md) |
| Decision | Charter and ledger thresholds are design constraints for measurement; promotion to hard gates requires fixtures, tiers, and Decision Register updates. |
| Consequences | Teams MUST measure against them; revising thresholds with evidence is not automatically a product regression. |

## DR-018 — Least authority for files and extensions

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 00, 22, 23, 21 |
| Decision | File/clipboard/drag and extensions receive explicit capabilities; parsers use checked allocation; paths not reconstructed from untrusted metadata. |
| Consequences | Portal/capability plumbing; more denial UX. |

## DR-019 — Vendor-neutral IA and naming

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 01, [28](../28-UX-Guidelines.md), [Glossary](Glossary.md) |
| Decision | Familiar concepts without proprietary branding or vendor workflow copying. Command IDs and docs stay vendor-neutral. |
| Consequences | Writers must avoid trademarked workflow names; interchange still allowed via adapters. |

## DR-020 — Text and shape are deterministic local engines

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | [18](../18-Text-Engine.md), [19](../19-Shape-Engine.md) |
| Decision | Text/shape content is local and deterministic; rasterize boundaries explicit. “Generated” means procedural/deterministic, never generative AI. |
| Consequences | Font/shaping portability issues managed explicitly; no model downloads. |

## DR-021 — Error taxonomy and fail-closed mutation

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | 00, [Error Taxonomy](Error-Taxonomy.md), 08 |
| Decision | Typed errors with preserved-state and retry policy; invariant failures freeze mutations and avoid speculative repair. |
| Consequences | Stronger recovery story; some ops unavailable until reopen. |

## DR-022 — Headless testability of core

| Field | Content |
| --- | --- |
| Status | Accepted |
| Docs | [31](../31-Testing.md), 32 |
| Decision | Core document/command tests MUST run without graphical desktop; GPU tests tolerate bounded variance; fuzz untrusted parsers. |
| Consequences | Command spine and pure migrations favored over toolkit-coupled logic. |

## Decision Process

1. Identify high reversal cost (format bytes, ABI, toolkit, thread model, truth ownership).
2. Write context, options, forces in this register (and ADR file if detail warrants).
3. Cite affected numbered docs and appendices; update [Cross-Reference Index](Cross-Reference-Index.md) if navigation changes.
4. Mark Provisional when evidence pending; list measurements.
5. Supersede by adding new DR and setting old status to Superseded with link.
6. Narrower specs MUST NOT silently contradict Accepted decisions; resolve via new DR.

## Conflict Resolution Order

When requirements conflict ([Requirement Keywords](Requirement-Keywords.md)):

1. document integrity and user safety;
2. security and least authority;
3. narrower subsystem specification;
4. earlier foundation documents;
5. recommendations and optional behavior.

## DR-023 — Tech stack frozen to shipping codebase

| Field | Content |
| --- | --- |
| Status | **Accepted** |
| Date | 2026-07-16 |
| Docs | [Alignment Roadmap](Alignment-Roadmap.md), [32](../32-Developer-Guide.md), archived ADR-002/003/004/005 |
| Context | Handbook left toolkit/runtime open (former DR-008); codebase already ships a working stack. Owner directed: keep tech stack exactly as code. |
| Decision | Freeze: Linux/Wayland desktop GUI; Rust edition 2024; Qt 6.10+ QML Controls 2; qtbridge 0.2 for app logic; thin C++ canvas + QML AOT only; wgpu 30 Vulkan-first; zero-copy interactive present; workspace crates `phototux` / `phototux_ui` / `phototux_engine` / `phototux_gpu` / `phototux_canvas` / `phototux_io`; GPL-3.0-or-later; Phosphor icons. |
| Consequences | Handbook MUST describe this stack as binding. No toolkit/GPU/shell rewrite for “neutrality.” Semantic contracts (commands, snapshots, workspace models) still apply **on top of** this stack. |
| Revisit | Catastrophic blocker only (zero-copy impossible, qtbridge abandoned upstream with no path, etc.). |

## DR-024 — Single-document session v1

| Field | Content |
| --- | --- |
| Status | **Accepted** (v1) |
| Date | 2026-07-16 |
| Docs | [Alignment Roadmap](Alignment-Roadmap.md), archived ADR-013 |
| Decision | Application session hosts **one** editable document at a time. Multi-window / tabs / multi-doc registry are out of v1. |
| Consequences | Lifecycle/workspace handbook multi-doc sections are **target architecture**; implementation waits for an explicit amend of this DR. |
| Revisit | When Phase 5 multi-doc project is scheduled with UX + session design. |

## DR-025 — Crate topology: coarse workspace

| Field | Content |
| --- | --- |
| Status | **Accepted** |
| Docs | [32](../32-Developer-Guide.md), [Alignment Roadmap](Alignment-Roadmap.md) |
| Decision | Keep the current Cargo members. Handbook’s fine-grained crate list is a **logical ownership map** implemented as modules (and later optional splits) inside existing crates. |
| Consequences | No big-bang package rename. Dependency rules of §32 still apply inside the coarse layout. |
| Revisit | Compile-time or ownership pain with measured split proposal. |

## DR-026 — Native `.ptx` container v1

| Field | Content |
| --- | --- |
| Status | **Accepted** (v1); **v2 chunked writes Accepted** (2026-07-16) |
| Docs | [27](../27-File-Formats.md), `phototux_io`, archived ADR-016 |
| Decision | Product native editable format is **`.ptx`**. Open/save paths continue to use it. Future work evolves chunking, integrity, and sparse resources **compatibly** (versioned schema), not a second native extension. |
| Encoding (2026-07-16) | Writers emit **format version 2**: typed chunks `MANI` / `RASL` / `MASK` + whole-body CRC32. Readers still open **v1** monolithic bodies. Unknown optional chunks are skipped. |
| Consequences | Handbook “bytes deferred” means future encoding improvements, not “format unset.” |
| Revisit | Sparse/tile manifests and incremental save when large-doc evidence demands. |

## DR-027 — Graph kind set includes Shape

| Field | Content |
| --- | --- |
| Status | **Accepted** |
| Date | 2026-07-16 |
| Docs | [10](../10-Document-Model.md), [19](../19-Shape-Engine.md), [DR-020](#dr-020--text-and-shape-are-deterministic-local-engines), archived ADR-017 |
| Context | ADR-017 closed the typed kind list; DR-020 already requires deterministic shape engines. Phase 4 needs `LayerKind::Shape` without rewriting DR-020. |
| Decision | Extend the document graph kind set with **`Shape`**. Shape layers own vector geometry (`ShapeContent` / path + fill/stroke) and contribute via explicit rasterization (GPU upload or bake-to-raster). Free document `PathDocument` paths remain separate. Old graphs without Shape continue to deserialize. |
| Consequences | `GRAPH_SCHEMA_VERSION` may bump for clarity; serde must remain backward-compatible. Full handbook 19 (booleans, parametric primitives) is incremental after v1 rect/ellipse/line. |
| Revisit | When Shape payload needs a breaking schema change. |

## Open Deferred Cluster

| Topic | Related DR | Blocking evidence |
| --- | --- | --- |
| Async runtime library mandate | DR-023 | Not required; revisit if workers insufficient |
| Plugin ABI | DR-009 | isolation vs performance spikes |
| `.ptx` chunk/sparse evolution | DR-026 | sparse/incremental/recovery spikes |
| Tile geometry | DR-006 | large-doc + brush benchmarks |
| History spill format | DR-004 | memory pressure scenarios |
| Multi-document session | DR-024 | product scheduling + UX |

## Cross References

- [00 — Introduction](../00-Introduction.md)
- [Alignment Roadmap](Alignment-Roadmap.md)
- [Codebase-Handbook Gap Analysis](Codebase-Handbook-Gap-Analysis.md)
- [Subsystem Dependency Matrix](Subsystem-Dependency-Matrix.md)
- [Document Format Versioning](Document-Format-Versioning.md)
- [Performance Budget Ledger](Performance-Budget-Ledger.md)
- [Cross-Reference Index](Cross-Reference-Index.md)
