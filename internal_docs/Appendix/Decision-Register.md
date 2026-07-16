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
| Status | Accepted |
| Docs | [17](../17-Rendering-Engine.md), 10 |
| Decision | Render workers consume versioned immutable snapshots and bounded deltas; they MUST NOT mutate authoritative documents. |
| Consequences | Snapshot publisher required; stale result policy required; GPU caches keyed by full semantic inputs. |

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
| Deferred | Specific UI toolkit choice. |

## DR-008 — UI toolkit and application runtime deferred

| Field | Content |
| --- | --- |
| Status | Deferred |
| Docs | 00, [32](../32-Developer-Guide.md) |
| Decision | No binding commitment to a UI toolkit or async runtime until prototypes measure input latency, a11y bridge cost, and packaging. |
| Seams | Presentation contract, host contract, command/actions already specified. |
| Revisit | After measured spikes recorded in journal/evidence. |

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
| Status | Accepted |
| Docs | [27](../27-File-Formats.md), [22](../22-Import-Export.md) |
| Decision | Native chunked versioned container is editable persistence authority. Third-party formats are adapters with loss disclosure. |
| Deferred | Exact bytes, magic, compression, container library. |
| Evidence needed | Huge sparse, incremental save, recovery, unknown preserve. |

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
| Consequences | Closing views ≠ closing documents; restore is reconciliation. |

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

## Open Deferred Cluster

| Topic | Related DR | Blocking evidence |
| --- | --- | --- |
| UI toolkit | DR-008 | latency, a11y, packaging spikes |
| Async runtime | DR-008 | cancellation + scheduling spikes |
| Plugin ABI | DR-009 | isolation vs performance spikes |
| Native container bytes | DR-013 | sparse/incremental/recovery spikes |
| Tile geometry | DR-006 | large-doc + brush benchmarks |
| History spill format | DR-004 | memory pressure scenarios |

## Cross References

- [00 — Introduction](../00-Introduction.md)
- [Subsystem Dependency Matrix](Subsystem-Dependency-Matrix.md)
- [Document Format Versioning](Document-Format-Versioning.md)
- [Performance Budget Ledger](Performance-Budget-Ledger.md)
- [Cross-Reference Index](Cross-Reference-Index.md)
