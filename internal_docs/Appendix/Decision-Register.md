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
| Status | **Accepted** (v1 lease + bounded pixel publish); dense deltas / tiles **Provisional** |
| Docs | [17](../17-Rendering-Engine.md), 10, [Alignment Roadmap](Alignment-Roadmap.md) |
| Decision | Render workers consume versioned immutable snapshots and bounded deltas; they MUST NOT mutate authoritative documents. |
| Accepted v1 (shipping) | Document **generation** counters plus metadata **`DocumentSnapshotLease`** (and `mark_persisted`) are the authoritative stale-result / cache-key contract. Workers and GPU paths key off generation; they do not hold mutable graph refs across async work. |
| Accepted v1.1 (2026-07-17) | Bounded **`SnapshotPublisher` / `PixelSnapshot`** (`Arc` RGBA8 composite, 64 MiB cap) published from CPU composite or host GPU readback; invalidated on generation bump. |
| Provisional (later) | Dense delta streams and tile/pyramid publishers remain target architecture ([DR-006](#dr-006--gpu-first-via-wgpu-not-gpu-only) evidence). |
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
| Status | Provisional (interactive / device); **Accepted** for listed CI soft-gates |
| Docs | [30](../30-Performance.md), [Performance Budget Ledger](Performance-Budget-Ledger.md) |
| Decision | Charter and ledger thresholds are design constraints for measurement; promotion to hard gates requires fixtures, tiers, and Decision Register updates. |
| Consequences | Teams MUST measure against them; revising thresholds with evidence is not automatically a product regression. |
| Amendment (2026-07-17) | Soft CI fixtures in `phototux_engine::budget_harness` promote B2-proxy / B9 / B1-proxy rows to **Accepted (CI soft)**. Photon/present B1–B3, B5 large-doc, and GPU composite stay Provisional until Tier M evidence. |
| Amendment (2026-07-17 b) | Tier M synthetic 4K CPU proxies: `camera-nav-4k-120` (B2) and `command-batch-4k-60` (B1) Accepted as CI soft. Present/photon endpoints still Provisional. |
| Amendment (2026-07-17 c) | Present-path soft proxies: `present-nav-intervals-4k` (B2), `present-dirty-mark-4k` (B1), `session-warm-construct` (B3) Accepted as CI soft with Tier M synthetic evidence. Photon/GPU present still Provisional when no display in CI. |

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

## DR-024 — Document session model

| Field | Content |
| --- | --- |
| Status | **Accepted** (v2 — multi-document tabs) |
| Date | 2026-07-16; amended 2026-07-17 |
| Docs | [Alignment Roadmap](Alignment-Roadmap.md), archived ADR-013, [02](../02-Application-Lifecycle.md) |
| Decision (v1) | Application session hosted **one** editable document at a time. |
| Amendment (v2) | Session hosts a **tabbed document registry** (`DocumentRegistry`, max 8): one active `SessionState` + parked inactive sessions with CPU layer pixels for GPU rehydrate. Multi-**window** remains out of scope. |
| Consequences | New/Open park the current tab when opening another; Close activates another parked tab or clears. Tiling/spill/sparse stay gated ([DR-029](#dr-029--p11p12-remain-gated-no-ungated-impl)). |
| Revisit | Multi-window presentation; per-document worker pools if contention appears. |

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

## DR-028 — Engine depth deferred beyond P5–P10 slices

| Field | Content |
| --- | --- |
| Status | **Accepted** (v1 depth deferral; depth pass closed 2026-07-17) |
| Date | 2026-07-16 |
| Docs | [Handbook Parity Roadmap](Handbook-Parity-Roadmap.md) P5–P10 |
| Context | Full handbook brush dynamics, filter gallery, text/shape booleans, ICC host discovery, AT-SPI adapter, and snapshot pixel publisher exceed one parity pass. |
| Decision | Ship vertical spines now: brush presets + scatter field, `FilterPlan` on layers, soft-proof tags, history jump, clipboard 64 MiB bound, prefs schema 4 (density/contrast/motion), semantic a11y JSON, `extension_data` blobs. Remaining handbook MUST depth stays checklist `[P]` / Deferred until dedicated milestones—not silent gaps. |
| Amendment (2026-07-17) | Depth pass closed for shipping spines: brush texture tip, noise filter + exposure adjustment, font discovery + on-canvas text editor, display ICC host adapter + soft-proof, polygon/gradient/live vector + vector-preserving boolean partner, mask contrast/shift, dirty-rect overlay clip, AT-SPI evidence fixture + Qt Accessible names. |
| Residual `[P]` | Full lcms2 transform pipeline; custom AT-SPI D-Bus tree server (beyond Qt Accessible + projection JSON); GPU-resident live vectors at 60 Hz / tile residency; handbook-complete brush curves / filter gallery chapter depth. |
| Consequences | Parity checklist marks listed DR-028 depth rows `[x]` with residuals enumerated; no claim of full chapter 14–19 completeness. |
| Revisit | Per-engine milestone when product prioritizes residual `[P]` rows. |

## DR-029 — P11/P12 remain gated (no ungated impl)

| Field | Content |
| --- | --- |
| Status | **Accepted** (amended 2026-07-17) |
| Date | 2026-07-16 |
| Docs | Roadmap §4, DR-006, DR-009, DR-024, DR-026 |
| Decision | Do **not** implement tiling/pyramid, history spill, sparse `.ptx`, or plugin ABI without gates. Record gates only; `extension_data` opaque round-trip is the sole P12 seam prep. |
| Amendment (2026-07-17) | **Multi-document tabs** ungated via [DR-024](#dr-024--document-session-model) v2 product decision. Tiling / spill / sparse / plugin ABI remain gated. |
| Consequences | Checklist P11 multi-doc may ship; other P11/P12 rows stay `[!]` / seam until evidence + product need. |
| Revisit | When benchmark/UX/product gates in roadmap §4 are met. |

## DR-030 — Shared-queue barrier, not host wait, orders composite and present

| Field | Content |
| --- | --- |
| Status | **Accepted** |
| Date | 2026-08-12 |
| Docs | 17, 28, 30, DR-005, DR-006, DR-023 |
| Context | `composite()` blocked on `device.poll(wait_indefinitely())` after submit, and `recomposite()` runs on the UI thread. Handbook 28 forbids the UI thread waiting for GPU completion. The wait was load-bearing only because no other ordering was stated. |
| Decision | Zero-copy present is ordered by an **image memory barrier in submission order on the shared queue**, not by a host wait. Qt Quick adopts wgpu's `VkInstance`/`VkPhysicalDevice`/`VkDevice` and the same queue family/index, so `vkGetDeviceQueue` returns the identical `VkQueue`; a barrier recorded before submit applies to every later command on that queue, including Qt's frame. The interactive path polls non-blocking only, to retire submissions. |
| Consequences | The composite path never stalls the caller (measured: host 0.05 ms vs GPU 0.20–0.30 ms for the same pass). Worst case is one frame of staleness if Qt's frame begins before the composite submission — permitted by 28 ("present complete older frame"), and self-correcting because `FrameAnimation` repaints continuously. `SharedQueueGuard` is still required: it provides CPU-side external synchronization for `vkQueueSubmit`, which is a separate obligation from GPU ordering. |
| Invariants | Both stacks **MUST** keep sharing one device *and* one queue. Introducing a second queue, or a second device, invalidates the barrier argument and requires an exported timeline semaphore instead. Readback paths that map buffers to host memory still wait, and **MUST** keep doing so. |
| Verification | `interactive_composite_does_not_wait_for_the_gpu` asserts host time stays below measured GPU pass time; composite readback round-trip tests cover pixel correctness. |
| Revisit | If Qt or wgpu stops sharing the queue, if a present-time race is observed, or if staleness becomes visible under a non-continuous repaint policy. |

## DR-031 — GPU gates measured by timestamp query

| Field | Content |
| --- | --- |
| Status | **Accepted** |
| Date | 2026-08-12 |
| Docs | 30, Performance Budget Ledger, DR-017, DR-030 |
| Context | The ADR-008 composite gate is stated in GPU milliseconds but was measured with a host `Instant` wrapped around submit plus a blocking poll — that measures the CPU stall. Removing the stall under [DR-030](#dr-030--shared-queue-barrier-not-host-wait-orders-composite-and-present) would have made the same metric read near zero and pass vacuously. |
| Decision | Pass-level GPU budgets are measured with `TIMESTAMP_QUERY` around the pass. Results are collected asynchronously and are **one submission late**; no measurement may reintroduce a wait on the interactive path. Benchmarks and conformance gates use an explicit measured entry point that does wait. |
| Consequences | `compositeMs` is real GPU time. Timestamp support is optional, so a device without it reports 0 (surfaced as "no GPU timing") and benchmarks fall back to host wall time, which overstates GPU cost. Stroke latency now reports input→submit; end-to-end input→present needs present-side instrumentation and remains unmeasured. |
| Revisit | When present-side instrumentation exists, or if a target adapter lacks timestamp queries. |

## DR-032 — Graph kind set includes SmartObject

| Field | Content |
| --- | --- |
| Status | **Accepted** (v1 embedded sources; linked files and sub-document editing Deferred) |
| Date | 2026-09-01 |
| Docs | [10](../10-Document-Model.md), [11](../11-Layer-System.md), [27](../27-File-Formats.md), [DR-026](#dr-026--native-ptx-container-v1), [DR-027](#dr-027--graph-kind-set-includes-shape) |
| Context | Transforming a raster layer bakes: scaling to a tenth and back returns a twenty-times upscale of what survived. Photoshop's answer is the smart object, and it is the one non-destructive primitive PhotoTux had no form of. DR-027 already established that extending the kind set is the way a new layer behaviour arrives. |
| Decision | Extend the kind set with **`SmartObject`**. The layer keeps its `SmartObjectContent` — a source name, an asset key, the source's dimensions, and a `placement` transform. A placement change **restores the source and re-applies the whole transform**, never composing one placement with the last. Commands: `smartobject.create` (wrap a pixel layer), `smartobject.set-placement`, `smartobject.rasterize`. Kind and payload move as one `Batch` history entry so undo cannot separate them. |
| Where the pixels live | The engine describes documents and owns no pixel buffers, so the source is **not** in the graph. The host holds it keyed by layer id, and `.ptx` stores it in a new optional `SRCE` chunk with a `smart_asset_ids` map, mirroring how masks were added. Older readers skip the chunk (DR-026 evolve-in-place) and older documents load without one. |
| Consequences | One document-sized RGBA buffer per smart object in host memory and in the file, which is what "embedded" means and what Photoshop also pays. A document whose source is missing — opened from a build that predates this, or a converter that dropped the chunk — shows the pixels it already had and says in the inspector that it can no longer be re-placed. Only a **pixel** layer can be wrapped: a group, text, shape, adjustment or fill layer describes itself rather than owning a buffer, and wrapping one would need a flatten first, which is a separate command rather than this one pretending. |
| Deferred | Linked sources (a smart object pointing at a file on disk), editing contents as a sub-document, smart filters, and per-instance sources shared between layers. Each is additive: `SmartObjectContent` gains fields with `#[serde(default)]`. |
| Revisit | When linked sources or sub-document editing is scheduled, or when the memory cost of embedded sources needs a residency strategy. |

## Open Deferred Cluster

| Topic | Related DR | Blocking evidence |
| --- | --- | --- |
| Async runtime library mandate | DR-023 | Not required; revisit if workers insufficient |
| Plugin ABI | DR-009 / DR-029 | isolation vs performance spikes; product need |
| `.ptx` chunk/sparse evolution | DR-026 / DR-029 | sparse/incremental/recovery spikes |
| Tile geometry | DR-006 / DR-029 | large-doc + brush benchmarks |
| History spill format | DR-004 / DR-029 | memory pressure scenarios |
| Multi-document session | DR-024 v2 | **Shipped tabs**; multi-window still open |
| Tiling / spill / sparse | DR-029 | large-doc + memory evidence |
| Engine chapter depth | DR-028 | per-engine milestones |

## Cross References

- [00 — Introduction](../00-Introduction.md)
- [Alignment Roadmap](Alignment-Roadmap.md)
- [Handbook Parity Roadmap](Handbook-Parity-Roadmap.md)
- [Handbook Parity Checklist](Handbook-Parity-Checklist.md)
- [Codebase-Handbook Gap Analysis](Codebase-Handbook-Gap-Analysis.md)
- [Subsystem Dependency Matrix](Subsystem-Dependency-Matrix.md)
- [Document Format Versioning](Document-Format-Versioning.md)
- [Performance Budget Ledger](Performance-Budget-Ledger.md)
- [Cross-Reference Index](Cross-Reference-Index.md)
