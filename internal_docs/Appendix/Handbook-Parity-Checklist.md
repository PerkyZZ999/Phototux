# Handbook Parity Checklist

Living tracker for [Handbook-Parity-Roadmap.md](Handbook-Parity-Roadmap.md).  
Prerequisite: [Alignment Roadmap](Alignment-Roadmap.md) complete; [Implementation-Checklist.md](Implementation-Checklist.md) is **alignment history** (do not reopen Phase 0–4 there).

Legend: `[ ]` todo · `[~]` partial · `[x]` done · `[!]` blocked/gated · `[P]` post-gate / optional depth · `[N]` never (out of product)

**Stack frozen:** [DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase).

---

## Standing rules

- [ ] Update this file when starting/finishing a slice
- [ ] Add/update `command_id` + [Command-Taxonomy.md](Command-Taxonomy.md) for document commits
- [ ] Close matching rows in [Codebase-Handbook-Gap-Analysis.md](Codebase-Handbook-Gap-Analysis.md)
- [ ] `./scripts/check-rust.sh` green
- [ ] No paragraph-long workaround comments
- [ ] Journal phase exits under `archive/docs/04-journal/`

---

## Baseline (shipped — do not re-open)

- [x] Alignment Phases 0–4 + handbook-ready exit
- [x] Document command spine (`SessionState::invoke`)
- [x] Generation + snapshot leases (DR-005 v1)
- [x] Shell descriptors + prefs + Essentials reset (DR-015 v1)
- [x] Shape / text bake / filters-styles subset / guides / color assign-convert / `.ptx` v2

---

## P1 — Action & command chrome

Chapters: [06](../06-Toolbar-System.md), [07](../07-Context-Menus.md), [08](../08-Command-System.md), [09](../09-Shortcut-System.md), [26](../26-Dialogs.md)

### P1.1 Action registry

- [x] Stable `ActionId` / contribution descriptors (label, icon, command, enablement deps)
- [x] Resolve presentation → action → `SessionState::invoke` (or host-only handler)
- [x] Enablement from command applicability (not ad-hoc QML bools)

### P1.2 Menus

- [x] File / Edit / Select / View / Image / Layer / Filters / Tools / Window / Help driven by action IDs (MenuBar; Tools menu N/A v1; context menus still hardcoded — P1.4)
- [~] Menu completeness vs IA ([01](../01-Information-Architecture.md)) for shipped commands
- [x] No document mutation from menu slots that bypass invoke (MenuBar → `invokeAction`)

### P1.3 Toolbars & tools

- [x] Tool strip consumes `tools_json` / tool descriptors
- [~] Tool-options bar bound to active tool + edit target (active tool yes; edit-target chrome in P3.1)
- [ ] Overflow / compact layout for narrow windows
- [~] Cancel-in-progress policy on tool switch (handbook 06) — transform/crop cancel on switch already

### P1.4 Context menus

- [x] Layer / canvas / selection / mask / path context menus from action registry (layer/canvas/selection/mask tagged; path deferred)
- [~] Selection preserved across menu open (handbook 07) — activate layer before invoke; full DR-011 snapshot later
- [x] Enablement matches command validation (`actionEnabled` / enablement tags)

### P1.5 Shortcuts

- [x] Shortcut map keyed by action/command IDs
- [x] Customizable bindings + conflict detection UI
- [x] Persist keymap in prefs
- [x] IME / text-field yield rules
- [~] Keyboard path for every non-gesture primary operation (shipped actions with defaults; some dialogs mouse-first)

### P1.6 Command search

- [x] Command palette / search dialog (handbook 26)
- [~] Fuzzy match on action labels + IDs (substring match on label/id/menu; fuzzy stretch later)
- [x] Invoke selected command with typed result / error surface

### P1.7 Command system depth

- [x] Descriptor metadata: scope, mutation class, undo policy, conflict policy (taxonomy axes) — `CommandMeta` / `COMMAND_META_ALL`
- [x] Application/workspace-scope command IDs where chrome needs them (`app.show-preferences`, workspace toggles)
- [x] Keep host-only exemptions documented (previews, paint stream, I/O adapters) — taxonomy catalog

**P1 exit:** chrome is action/command driven for shipped surfaces; palette works; shortcuts customizable. **Met** (remaining `[~]` polish elsewhere does not block).

---

## P2 — Workspace & docking

Chapters: [03](../03-Workspace-System.md), [04](../04-Docking-System.md), [05](../05-Panel-System.md)

### P2.1 Workspace model

- [x] Semantic workspace state separate from document (DR-015)
- [x] Workspace transaction / undo policy (layout changes ≠ document dirty)
- [x] Named workspace presets (built-in Essentials/Compact/Painting/Factory; user presets deferred)
- [x] Reset scopes: Essentials / last saved / factory
- [x] Active view / focus / panel context as distinct fields

### P2.2 Docking topology

- [x] Dock topology model (right-stack v1 + validation; full split graph deferred)
- [x] Tear-off floating docks
- [x] Auto-hide / reveal
- [x] Persist topology across restart (reconcile on display change deferred)
- [x] Drag/drop placement UX (header drag + keyboard reorder; full zone solver deferred)

### P2.3 Panel system

- [x] QML consumes `panels_json` for visibility, titles, regions
- [~] Panel lifecycle: open/close/pin shipped; follow-context deferred
- [ ] Virtualized Layers/History lists for large stacks
- [x] Placeholder / contribution slots for future panels
- [~] Paths / Character / Navigator parity with descriptor catalog (Navigator body + Paths/Character placeholders)

**P2 exit:** layout is model-driven; tear-off + presets work; document dirty unaffected by layout. **Met** (split graph / user presets / list virtualization remain polish).

---

## P3 — Selection & edit targets

Chapters: [01](../01-Information-Architecture.md), [12](../12-Selection-System.md), DR-011

### P3.1 Chrome (this batch)

- [~] Distinct: object selection vs pixel selection vs focus vs context target vs active edit target — pixel selection + edit target exposed (`pixelSelectionActive`, `editTarget`); object-selection later
- [x] UI chrome never collapses these into one “selection” (status + Properties separate clauses)
- [ ] Commands/announce for each concept (full DR-011 announce suite later)
- [x] Mask-edit target vs layer pixels clearly indicated (Properties Edit target row + status)
- [~] Selection channel ops (replace/add/subtract/intersect) complete in chrome (tool options present)
- [ ] Marching-ants / overlay performance within interactive budgets
- [ ] Select → mask / mask → selection flows

**P3 exit:** DR-011 concepts visible and enforced in tools + Properties + status.

---

## P4 — Masks & layer semantics

Chapters: [11](../11-Layer-System.md), [13](../13-Mask-System.md)

### P4.1 Masks

- [ ] Vector masks (path-based) on layers
- [ ] Refine edge (feather/contrast/shift) with preview + commit commands
- [ ] Apply mask / disable / delete semantics complete + history
- [ ] Mask density / invert / link flags in UI
- [ ] Paint-on-mask vs layer clarity (edit target)

### P4.2 Layers

- [ ] Lock flags enforced (pixels / position / all) on tools
- [ ] Multi-select layer ops (delete/reorder/group) atomic where handbook requires
- [ ] Fill / solid-color layer kind or equivalent
- [ ] Layer styles depth beyond Drop Shadow + Stroke (as handbook 11/15)
- [ ] Clipping groups UX polish
- [ ] Nondestructive effect stack ordering UI

**P4 exit:** vector + refine masks; locks real; layer stack matches handbook mental model for shipped kinds.

---

## P5 — Creative engines depth

Chapters: [14](../14-Brush-Engine.md), [15](../15-Filter-Engine.md), [18](../18-Text-Engine.md), [19](../19-Shape-Engine.md)

### P5.1 Brush

- [ ] Dynamics: size/opacity/flow pressure curves, scatter, texture (handbook subset prioritized)
- [ ] Brush preset library persistence + UI
- [ ] Stroke journal / recovery hooks
- [ ] CPU dab reference path for tests
- [ ] [P] Tile-aware stroke planner (after P11 tiling)

### P5.2 Filters

- [ ] Declarative filter / effect plan graph
- [ ] Filter gallery UX (browse + preview + commit)
- [ ] Additional GPU executors (sharpen, noise, color ops…) with CPU reference
- [ ] Cancel / stale-result policy for long filters
- [ ] Adjustment kinds completeness vs handbook 15

### P5.3 Text

- [ ] Editable text tool (on-canvas) beyond Character fields
- [ ] Typography: wrapping, bounds, more alignment/metrics
- [ ] Font resource discovery + fallback
- [ ] Retain editable text vs bake policy UX
- [ ] Text → path / rasterize commands explicit

### P5.4 Shape

- [ ] Boolean union / intersection / difference / exclusion
- [ ] Path edit tool (add/move/delete points, close)
- [ ] Parametric primitives beyond rect/ellipse/line
- [ ] Live vector contribution option vs always-raster upload
- [ ] Stroke/fill/gradient style depth

**P5 exit:** engines cover handbook feature sets claimed for v1 product; remaining items marked `[P]` only with DR note.

---

## P6 — Color & rendering contracts

Chapters: [16](../16-Color-Management.md), [17](../17-Rendering-Engine.md), DR-005, DR-006, DR-012

### P6.1 Color

- [ ] Soft-proof mode + proof intent UI
- [ ] ICC profile load/embed (document + export)
- [ ] Working-space policy beyond built-in sRGB/Display-P3
- [ ] Display profile discovery (Linux host adapter)
- [ ] Assign ≠ convert disclosures everywhere (Image menu + dialogs)

### P6.2 Rendering

- [ ] Immutable pixel snapshot / bounded delta publisher (beyond metadata leases)
- [ ] Workers consume leases only (no mutable graph across async)
- [ ] Broader GPU↔CPU blend/filter parity fixtures
- [ ] Device-loss / surface-loss UX (reconstruct or controlled fail)
- [ ] Dirty-region / overlay separation polish
- [ ] [!] Tiling / pyramid → **P11** (gated)

**P6 exit:** soft-proof + ICC foundation; snapshot publisher for workers; interactive present remains zero-copy.

---

## P7 — History & lifecycle

Chapters: [02](../02-Application-Lifecycle.md), [20](../20-History-Undo.md)

### P7.1 History

- [ ] Unified transaction timeline (graph + stroke + selection + transform)
- [ ] Coalescing / merge policy documented and tested
- [ ] History panel: labels, kinds, jump (safe)
- [ ] Retention budget UI
- [ ] [!] Spill-to-disk → **P11** (memory evidence)

### P7.2 Lifecycle

- [ ] Formal lifecycle controller (startup / session / shutdown)
- [ ] Recovery UX (~handbook autosave bound) + restore chooser
- [ ] Safe-start (suppress custom chrome on crash loop)
- [ ] Save coordination: staged identity vs generation receipts
- [ ] GPU/renderer generation orchestration on device loss
- [ ] [!] Multi-window / multi-doc → **P11** (DR-024 amend)

**P7 exit:** one history model; recovery usable; lifecycle explicit.

---

## P8 — Clipboard & interchange I/O

Chapters: [21](../21-Clipboard.md), [22](../22-Import-Export.md), [27](../27-File-Formats.md)

### P8.1 Clipboard

- [ ] Capability-scoped host clipboard bridge
- [ ] Multi-format negotiation (pixels / layer / SVG-ish paths as available)
- [ ] Mask / selection payload copy
- [ ] Security / size bounds

### P8.2 Import / export

- [ ] Hard allocation / dimension / decompression limits on all adapters
- [ ] Structured loss / compatibility reports for every adapter
- [ ] Cancel + progress contracts for long jobs
- [ ] Expand raster codec coverage as needed (still adapters)
- [ ] PSD subset deepen (layers/masks) with disclosure

### P8.3 Native `.ptx` (non-sparse)

- [ ] Migration / schema evolution tests for v2 chunks
- [ ] Stronger integrity diagnostics UX
- [ ] Unknown optional chunk preserve round-trip tests
- [ ] Opaque extension object placeholders (prep for P12)
- [ ] [!] Sparse tiles / incremental save → **P11**

**P8 exit:** clipboard + adapters meet handbook hostile-input and disclosure bar; `.ptx` solid without sparse.

---

## P9 — Preferences, themes, UX polish

Chapters: [01](../01-Information-Architecture.md), [24](../24-Preferences.md), [25](../25-Themes.md), [28](../28-UX-Guidelines.md)

### P9.1 Preferences

- [ ] Handbook preference schema coverage (view/tool/perf/a11y keys)
- [ ] Versioned migrations
- [ ] Effective-value precedence where document vs user differs
- [ ] Reset field / domain / all
- [ ] Safe-start prefs path

### P9.2 Themes

- [ ] Migrate archived `DESIGN.md` tokens → handbook Themes + QML single source
- [ ] High-contrast pack
- [ ] Density / UI scale packs
- [ ] No ad-hoc colors outside tokens

### P9.3 UX

- [ ] Mixed-value inspector pattern
- [ ] Operation progress / ack patterns
- [ ] Discoverability: every command via menu or palette
- [ ] Reduced-motion + 200% scale audit
- [ ] Progressive disclosure for advanced Properties

**P9 exit:** tokens unified; prefs migrations; UX guidelines satisfied for shipped chrome.

---

## P10 — Accessibility

Chapter: [29](../29-Accessibility.md), DR-016

- [ ] Semantic accessibility tree from descriptors/commands (not pixel inference)
- [ ] AT-SPI host adapter mapping
- [ ] Canvas structured summary / explorer
- [ ] Keyboard-complete workflows (non-gesture)
- [ ] Name/role/state/value on tools, panels, dialogs
- [ ] Flood control for announcements
- [ ] Contrast / focus / scale gates in checklist evidence

**P10 exit:** assistive tech sees handbook semantic projection for primary workflows.

---

## P11 — Scale & multi-document (gated)

Chapters: [02](../02-Application-Lifecycle.md), [03](../03-Workspace-System.md), [17](../17-Rendering-Engine.md), [20](../20-History-Undo.md), [27](../27-File-Formats.md)

### Gates (must check before coding)

- [!] Large-doc benchmark proves tiling needed ([DR-006](Decision-Register.md#dr-006--gpu-first-via-wgpu-not-gpu-only))
- [!] Explicit amend of [DR-024](Decision-Register.md#dr-024--single-document-session-v1) before multi-doc
- [!] Memory-pressure evidence before history spill
- [!] Sparse/incremental `.ptx` spike before freezing strategy ([DR-026](Decision-Register.md#dr-026--native-ptx-container-v1))

### P11.1 Tiling / pyramid

- [!] Sparse tile store + residency
- [!] Multiresolution pyramid for navigation
- [!] Brush/filter tile planner
- [!] Eviction that never drops authoritative unsaved state

### P11.2 Multi-document

- [!] Document registry + tabs
- [!] Per-document mutation serialization (DR-010)
- [!] Multi-view of one document
- [!] Cross-window document presentation (if product wants)

### P11.3 History spill & `.ptx` sparse

- [!] History spill-to-disk + restore
- [!] Tile-addressable resources in `.ptx`
- [!] Incremental / append save strategy (optional, validated)

**P11 exit:** gated items implemented only after gates; budgets held on large docs.

---

## P12 — Extension capability seams (gated by need)

Chapter: [23](../23-Plugin-SDK.md), DR-009

- [!] Product need recorded (do not build “because handbook mentions plugins”)
- [ ] Contribution manifests (panels/commands/filters) behind capabilities
- [ ] Budgets + failure isolation
- [ ] Opaque extension data in document + `.ptx` round-trip
- [ ] Host mediation; no mutable document refs to extensions
- [N] Stable native ABI / marketplace / cloud plugin store

**P12 exit:** seams exist; ABI remains Deferred unless new DR.

---

## P13 — Verification & budget promotion

Chapters: [30](../30-Performance.md), [31](../31-Testing.md), [32](../32-Developer-Guide.md), DR-017, DR-022

### P13.1 Testing

- [ ] Command-router conformance suite (all shipped IDs, headless)
- [ ] Hostile I/O fuzz / limit tests per adapter
- [ ] GPU device-loss suite (or documented skip matrix)
- [ ] CPU vs GPU tolerance fixtures for claimed ops
- [ ] A11y evidence pack (manual + automated where possible)

### P13.2 Performance

- [ ] Fixture harness for input→preview, pan/zoom, composite, boot
- [ ] Promote [Performance Budget Ledger](Performance-Budget-Ledger.md) rows Provisional → Accepted with evidence
- [ ] CI regression gates for promoted budgets
- [ ] Large-doc benchmark suite (feeds P11 gate)

### P13.3 Developer guide practice

- [ ] Contrib checklist: new command + taxonomy + tests
- [ ] Crate boundary lint/culture (engine no Qt; UI no wgpu)
- [ ] Thread/ownership map kept current
- [ ] [N] 18-crate rename (DR-025)

**P13 exit:** claimed quality attributes have fixtures; handbook 30/31 no longer Provisional where product claims them.

---

## Full parity exit criteria

- [ ] All non-gated P1–P10 and P13 items `[x]` or explicitly Deferred in Decision Register with reason
- [ ] All P11/P12 items either `[x]` after gates **or** `[P]`/`[N]` with DR amend
- [ ] Gap analysis has no silent MUST contradictions
- [ ] Roadmap §1 “full parity” definition satisfied
- [ ] Journal: `handbook-parity-complete` entry

---

## Never / out of product

- [N] Cloud sync, accounts, collaboration
- [N] AI / generative tools
- [N] Electron / web / CLI / TUI product
- [N] Toolkit or GPU API replacement
- [N] Plugin marketplace / stable third-party ABI (until separate DR)
