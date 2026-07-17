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

- [x] Distinct: object selection vs pixel selection vs focus vs context target vs active edit target (`objectSelectionLabel`, `pixelSelectionActive`, `editTarget`, workspace focus)
- [x] UI chrome never collapses these into one “selection” (status + Properties separate clauses)
- [~] Commands/announce for each concept (`lastAnnounce` + status; full a11y flood-control later)
- [x] Mask-edit target vs layer pixels clearly indicated (Properties Edit target row + status)
- [~] Selection channel ops (replace/add/subtract/intersect) complete in chrome (tool options present)
- [~] Marching-ants / overlay performance within interactive budgets (GPU ants shipped; SLO evidence → P13)
- [x] Select → mask / mask → selection flows (`selection.to-mask`, `mask.to-selection`)

**P3 exit:** DR-011 concepts visible and enforced in tools + Properties + status. **Met** for shipped concepts (multi-object select polish deferred).

---

## P4 — Masks & layer semantics

Chapters: [11](../11-Layer-System.md), [13](../13-Mask-System.md)

### P4.1 Masks

- [~] Vector masks (path-based) on layers — metadata + `mask.create-vector`; path edit deferred
- [~] Refine edge (feather/contrast/shift) with preview + commit commands — feather/density/invert via `mask.set-attributes`; contrast/shift deferred
- [x] Apply mask / disable / delete semantics complete + history
- [~] Mask density / invert / link flags in UI (commands + Properties toggles)
- [x] Paint-on-mask vs layer clarity (edit target)

### P4.2 Layers

- [x] Lock flags enforced (pixels / position / all) on tools
- [ ] Multi-select layer ops (delete/reorder/group) atomic where handbook requires
- [ ] Fill / solid-color layer kind or equivalent
- [ ] Layer styles depth beyond Drop Shadow + Stroke (as handbook 11/15)
- [~] Clipping groups UX polish
- [ ] Nondestructive effect stack ordering UI

**P4 exit:** vector + refine masks; locks real; layer stack matches handbook mental model for shipped kinds. **Partial** — locks + mask attrs + vector metadata shipped; remaining polish deferred.

---

## P5 — Creative engines depth

Chapters: [14](../14-Brush-Engine.md), [15](../15-Filter-Engine.md), [18](../18-Text-Engine.md), [19](../19-Shape-Engine.md)

### P5.1 Brush

- [~] Dynamics: size/opacity/flow pressure curves, scatter, texture (handbook subset prioritized) — opacity/flow/scatter/spacing + size/opacity pressure on `BrushParams`; texture deferred ([DR-028](Decision-Register.md#dr-028--engine-depth-deferred-beyond-p5p10-slices))
- [x] Brush preset library persistence + UI (prefs schema 4 JSON + Properties apply/save)
- [ ] Stroke journal / recovery hooks
- [x] CPU dab reference path for tests (`stamp_dab_rgba` / `paint_dabs_rgba`)
- [P] Tile-aware stroke planner (after P11 tiling)

### P5.2 Filters

- [x] Declarative filter / effect plan graph (`FilterPlan` on `Layer`; JSON round-trip)
- [ ] Filter gallery UX (browse + preview + commit)
- [~] Additional GPU executors (sharpen, noise, color ops…) with CPU reference — `cpu_sharpen_rgba` + Sharpen effect command; GPU pack path deferred
- [ ] Cancel / stale-result policy for long filters
- [~] Adjustment kinds completeness vs handbook 15 — existing subset; gallery deferred (DR-028)

### P5.3 Text

- [~] Editable text tool (on-canvas) beyond Character fields — Character + bake shipped; on-canvas edit deferred (DR-028)
- [ ] Typography: wrapping, bounds, more alignment/metrics
- [ ] Font resource discovery + fallback
- [~] Retain editable text vs bake policy UX — bake command exists
- [~] Text → path / rasterize commands explicit — bake path shipped

### P5.4 Shape

- [ ] Boolean union / intersection / difference / exclusion
- [ ] Path edit tool (add/move/delete points, close)
- [~] Parametric primitives beyond rect/ellipse/line — rect/ellipse/line shipped; more deferred (DR-028)
- [ ] Live vector contribution option vs always-raster upload
- [~] Stroke/fill/gradient style depth — fill/stroke v1; gradient deferred

**P5 exit:** engines cover handbook feature sets claimed for v1 product; remaining items marked `[P]` only with DR note. **Met for shipped spines** (presets + `FilterPlan`); depth → DR-028.

---

## P6 — Color & rendering contracts

Chapters: [16](../16-Color-Management.md), [17](../17-Rendering-Engine.md), DR-005, DR-006, DR-012

### P6.1 Color

- [x] Soft-proof mode + proof intent UI (`document.set-soft-proof`; Image menu; Properties status)
- [ ] ICC profile load/embed (document + export)
- [~] Working-space policy beyond built-in sRGB/Display-P3 — assign/convert tags; ICC bytes deferred (DR-028)
- [ ] Display profile discovery (Linux host adapter)
- [~] Assign ≠ convert disclosures everywhere (Image menu + dialogs) — menu actions present

### P6.2 Rendering

- [~] Immutable pixel snapshot / bounded delta publisher (beyond metadata leases) — generation leases shipped; pixel publisher deferred (DR-028)
- [~] Workers consume leases only (no mutable graph across async) — I/O worker path; full contract deferred
- [ ] Broader GPU↔CPU blend/filter parity fixtures
- [ ] Device-loss / surface-loss UX (reconstruct or controlled fail)
- [ ] Dirty-region / overlay separation polish
- [!] Tiling / pyramid → **P11** (gated)

**P6 exit:** soft-proof + ICC foundation; snapshot publisher for workers; interactive present remains zero-copy. **Partial Met** — soft-proof spine shipped; ICC/publisher depth → DR-028.

---

## P7 — History & lifecycle

Chapters: [02](../02-Application-Lifecycle.md), [20](../20-History-Undo.md)

### P7.1 History

- [x] Unified transaction timeline (graph + stroke + selection + transform)
- [~] Coalescing / merge policy documented and tested — coalescing exists; fuller suite deferred
- [x] History panel: labels, kinds, jump (safe) (`history.jump` + host undo loop)
- [ ] Retention budget UI
- [!] Spill-to-disk → **P11** (memory evidence)

### P7.2 Lifecycle

- [~] Formal lifecycle controller (startup / session / shutdown) — session/prefs/recovery hooks; formal controller deferred
- [~] Recovery UX (~handbook autosave bound) + restore chooser — recovery module shipped; chooser polish deferred
- [ ] Safe-start (suppress custom chrome on crash loop)
- [~] Save coordination: staged identity vs generation receipts — generation on graph
- [ ] GPU/renderer generation orchestration on device loss
- [!] Multi-window / multi-doc → **P11** (DR-024 amend)

**P7 exit:** one history model; recovery usable; lifecycle explicit. **Partial Met** — timeline + jump shipped; spill/multi-doc gated.

---

## P8 — Clipboard & interchange I/O

Chapters: [21](../21-Clipboard.md), [22](../22-Import-Export.md), [27](../27-File-Formats.md)

### P8.1 Clipboard

- [~] Capability-scoped host clipboard bridge — in-app RGBA clipboard; OS MIME bridge deferred
- [~] Multi-format negotiation (pixels / layer / SVG-ish paths as available) — pixels → paste layer
- [ ] Mask / selection payload copy
- [x] Security / size bounds (64 MiB refuse on copy)

### P8.2 Import / export

- [x] Hard allocation / dimension / decompression limits on all adapters (`MAX_DIMENSION` / `MAX_RASTER_BYTES`)
- [~] Structured loss / compatibility reports for every adapter — PSD truncation messages; broaden deferred
- [~] Cancel + progress contracts for long jobs — cancel token + I/O busy; full progress UX deferred
- [~] Expand raster codec coverage as needed (still adapters) — PNG/JPEG/WebP/TIFF/BMP/GIF
- [~] PSD subset deepen (layers/masks) with disclosure — subset + limits shipped

### P8.3 Native `.ptx` (non-sparse)

- [~] Migration / schema evolution tests for v2 chunks — v1 read / v2 write tests
- [ ] Stronger integrity diagnostics UX
- [x] Unknown optional chunk preserve round-trip tests (`skips_unknown_optional_chunk`)
- [x] Opaque extension object placeholders (prep for P12) — `DocumentGraph::extension_data`
- [!] Sparse tiles / incremental save → **P11**

**P8 exit:** clipboard + adapters meet handbook hostile-input and disclosure bar; `.ptx` solid without sparse. **Partial Met** — bounds + limits + extension placeholders; OS clipboard/MIME later.

---

## P9 — Preferences, themes, UX polish

Chapters: [01](../01-Information-Architecture.md), [24](../24-Preferences.md), [25](../25-Themes.md), [28](../28-UX-Guidelines.md)

### P9.1 Preferences

- [~] Handbook preference schema coverage (view/tool/perf/a11y keys) — schema 4: brush presets, density, contrast, motion
- [x] Versioned migrations (schema → 4 on load)
- [ ] Effective-value precedence where document vs user differs
- [~] Reset field / domain / all — workspace Essentials/factory; full domain reset deferred
- [ ] Safe-start prefs path

### P9.2 Themes

- [x] Migrate archived `DESIGN.md` tokens → handbook Themes + QML single source (`Theme.qml`)
- [x] High-contrast pack (`prefHighContrast` → `Theme.highContrast`)
- [x] Density / UI scale packs (`prefUiDensity` → `Theme.densityScale`)
- [~] No ad-hoc colors outside tokens — chrome uses Theme; audit stretch

### P9.3 UX

- [ ] Mixed-value inspector pattern
- [~] Operation progress / ack patterns — status + I/O busy
- [~] Discoverability: every command via menu or palette — palette + menus for shipped actions
- [~] Reduced-motion + 200% scale audit — prefs flag; full audit deferred
- [ ] Progressive disclosure for advanced Properties

**P9 exit:** tokens unified; prefs migrations; UX guidelines satisfied for shipped chrome. **Partial Met** — schema 4 + Theme packs shipped.

---

## P10 — Accessibility

Chapter: [29](../29-Accessibility.md), DR-016

- [x] Semantic accessibility tree from descriptors/commands (not pixel inference) — `accessibilityTreeJson`
- [ ] AT-SPI host adapter mapping
- [~] Canvas structured summary / explorer — canvas node in tree JSON
- [~] Keyboard-complete workflows (non-gesture) — shortcuts + palette; full parity deferred
- [~] Name/role/state/value on tools, panels, dialogs — QML Accessible.name on shipped controls
- [~] Flood control for announcements — `lastAnnounce` single channel
- [~] Contrast / focus / scale gates in checklist evidence — high-contrast pref; evidence pack → P13

**P10 exit:** assistive tech sees handbook semantic projection for primary workflows. **Partial Met** — semantic JSON spine; AT-SPI adapter deferred (DR-028).

---

## P11 — Scale & multi-document (gated)

Chapters: [02](../02-Application-Lifecycle.md), [03](../03-Workspace-System.md), [17](../17-Rendering-Engine.md), [20](../20-History-Undo.md), [27](../27-File-Formats.md)

### Gates (must check before coding)

- [!] Large-doc benchmark proves tiling needed ([DR-006](Decision-Register.md#dr-006--gpu-first-via-wgpu-not-gpu-only)) — **recorded; no impl** ([DR-029](Decision-Register.md#dr-029--p11p12-remain-gated-no-ungated-impl))
- [!] Explicit amend of [DR-024](Decision-Register.md#dr-024--single-document-session-v1) before multi-doc — **recorded; no impl** (DR-029)
- [!] Memory-pressure evidence before history spill — **recorded; no impl** (DR-029)
- [!] Sparse/incremental `.ptx` spike before freezing strategy ([DR-026](Decision-Register.md#dr-026--native-ptx-container-v1)) — **recorded; no impl** (DR-029)

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

**P11 exit:** gated items implemented only after gates; budgets held on large docs. **Exit for this pass:** gates recorded; **no ungated implementation** (DR-029).

---

## P12 — Extension capability seams (gated by need)

Chapter: [23](../23-Plugin-SDK.md), DR-009

- [!] Product need recorded (do not build “because handbook mentions plugins”) — **no product need; seams only** (DR-029 / DR-009)
- [ ] Contribution manifests (panels/commands/filters) behind capabilities
- [ ] Budgets + failure isolation
- [x] Opaque extension data in document + `.ptx` round-trip — `extension_data` JSON round-trip
- [~] Host mediation; no mutable document refs to extensions — opaque store only; no extension host yet
- [N] Stable native ABI / marketplace / cloud plugin store

**P12 exit:** seams exist; ABI remains Deferred unless new DR. **Partial Met** — opaque blob seam; ABI still Deferred.

---

## P13 — Verification & budget promotion

Chapters: [30](../30-Performance.md), [31](../31-Testing.md), [32](../32-Developer-Guide.md), DR-017, DR-022

### P13.1 Testing

- [x] Command-router conformance suite (all shipped IDs, headless) — `command_conformance` module
- [~] Hostile I/O fuzz / limit tests per adapter — dimension/alloc limit unit tests; fuzz deferred
- [ ] GPU device-loss suite (or documented skip matrix)
- [ ] CPU vs GPU tolerance fixtures for claimed ops
- [~] A11y evidence pack (manual + automated where possible) — semantic tree JSON; AT-SPI pack deferred

### P13.2 Performance

- [ ] Fixture harness for input→preview, pan/zoom, composite, boot
- [ ] Promote [Performance Budget Ledger](Performance-Budget-Ledger.md) rows Provisional → Accepted with evidence
- [ ] CI regression gates for promoted budgets
- [ ] Large-doc benchmark suite (feeds P11 gate)

### P13.3 Developer guide practice

- [~] Contrib checklist: new command + taxonomy + tests — taxonomy updated for soft-proof/jump
- [x] Crate boundary lint/culture (engine no Qt; UI no wgpu)
- [ ] Thread/ownership map kept current
- [N] 18-crate rename (DR-025)

**P13 exit:** claimed quality attributes have fixtures; handbook 30/31 no longer Provisional where product claims them. **Partial Met** — conformance suite green; budget promotion still Provisional (DR-017).

---

## Full parity exit criteria

- [~] All non-gated P1–P10 and P13 items `[x]` or explicitly Deferred in Decision Register with reason — spines Met; depth Deferred via DR-028
- [x] All P11/P12 items either `[x]` after gates **or** `[P]`/`[N]` with DR amend — gated/`[!]` + DR-029; P12 opaque seam `[x]`
- [~] Gap analysis has no silent MUST contradictions — remaining open rows tracked in checklist
- [~] Roadmap §1 “full parity” definition satisfied — **spine parity**; chapter-depth still open under DR-028
- [x] Journal: phase exits under `archive/docs/04-journal/` (P2–P13); full `handbook-parity-complete` when DR-028 depth closed

---

## Never / out of product

- [N] Cloud sync, accounts, collaboration
- [N] AI / generative tools
- [N] Electron / web / CLI / TUI product
- [N] Toolkit or GPU API replacement
- [N] Plugin marketplace / stable third-party ABI (until separate DR)
