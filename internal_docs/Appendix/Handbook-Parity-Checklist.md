# Handbook Parity Checklist

Living tracker for [Handbook-Parity-Roadmap.md](Handbook-Parity-Roadmap.md).  
Prerequisite: [Alignment Roadmap](Alignment-Roadmap.md) complete; [Implementation-Checklist.md](Implementation-Checklist.md) is **alignment history** (do not reopen Phase 0–4 there).

Legend: `[ ]` todo · `[~]` partial · `[x]` done · `[!]` blocked/gated · `[P]` post-gate / optional depth · `[N]` never (out of product)

**Stack frozen:** [DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase).  
**Depth deferral:** [DR-028](Decision-Register.md#dr-028--engine-depth-deferred-beyond-p5p10-slices).  
**Scale/plugin gates:** [DR-029](Decision-Register.md#dr-029--p11p12-remain-gated-no-ungated-impl).

---

## Status snapshot (2026-07-17)

| Phase | Exit | Notes |
| --- | --- | --- |
| P1 Action chrome | **Met** | Remaining `[~]` = polish |
| P2 Workspace / docking | **Met** | Split graph / user presets / list virtualization polish |
| P3 Selection targets | **Met** | Multi-object select polish deferred |
| P4 Masks / layers | **Met** | Multi-select ops, fill layers, effect reorder, clip break, mask apply/attrs UI, OuterGlow/ColorOverlay; mask contrast/shift refine shipped |
| P5 Creative engines | **Met** | Gallery + path-edit + text frame/wrap; texture tip, noise/exposure, fonts/on-canvas, polygon/gradient/live vector + vector boolean partner shipped (DR-028 depth) |
| P6 Color / render | **Met** | ICC embed + GPU↔CPU fixtures + device-loss UX; pixel publisher; display ICC discovery + soft-proof hook shipped |
| P7 History / lifecycle | **Met** | Retention UI + safe-start; spill gated; multi-doc → P11 tabs |
| P8 Clipboard / I/O | **Met** | Mask/selection clipboard + `.ptx` integrity diagnostics UX; SVG MIME / sparse → deferred/P11 |
| P9 Prefs / themes | **Met** | Mixed inspector + safe-start + effective-source spine; full schema/audit → deferred |
| P10 Accessibility | **Met** | Semantic JSON + AT-SPI projection + evidence fixture + Qt Accessible spine; full custom D-Bus provider residual `[P]` |
| P11 Scale / multi-doc | **Partial** | Multi-doc tabs shipped (DR-024 v2); tiling/spill/sparse still gated |
| P12 Plugin seams | **Partial** | `extension_data` seam; ABI Deferred; no product need |
| P13 Verification | **Met** | Soft CI harness; Tier M B1/B2 CPU proxies; interactive present still Provisional |

**Spine parity** for P1–P10/P13 shipped concepts is in place. DR-028 chapter-depth and DR-017 device evidence remain open. Journals: `archive/docs/04-journal/*handbook-parity*`.

---

## Standing rules

- [x] Update this file when starting/finishing a slice (living practice)
- [x] Add/update `command_id` + [Command-Taxonomy.md](Command-Taxonomy.md) for document commits
- [~] Close matching rows in [Codebase-Handbook-Gap-Analysis.md](Codebase-Handbook-Gap-Analysis.md) — keep in sync on each slice
- [x] `./scripts/check-rust.sh` green (gate on every commit)
- [x] No paragraph-long workaround comments
- [x] Journal phase exits under `archive/docs/04-journal/`

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

- [x] File / Edit / Select / View / Image / Layer / Filters / Tools / Window / Help driven by action IDs (MenuBar; Tools menu N/A v1; context menus — P1.4)
- [~] Menu completeness vs IA ([01](../01-Information-Architecture.md)) for shipped commands
- [x] No document mutation from menu slots that bypass invoke (MenuBar → `invokeAction`)

### P1.3 Toolbars & tools

- [x] Tool strip consumes `tools_json` / tool descriptors
- [~] Tool-options bar bound to active tool + edit target (active tool yes; edit-target chrome in P3)
- [x] Overflow / compact layout for narrow windows — tool strip capacity + `dots-three` overflow menu
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
- [x] Fuzzy match on action labels + IDs — subsequence score + substring boost in command palette
- [x] Invoke selected command with typed result / error surface

### P1.7 Command system depth

- [x] Descriptor metadata: scope, mutation class, undo policy, conflict policy — `CommandMeta` / `COMMAND_META_ALL`
- [x] Application/workspace-scope command IDs (`app.show-preferences`, workspace toggles)
- [x] Keep host-only exemptions documented — taxonomy catalog

**P1 exit:** **Met.** Remaining `[~]` polish does not block.

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
- [x] Virtualized Layers/History lists for large stacks — `ListView` + `reuseItems` + root pipe caches
- [x] Placeholder / contribution slots for future panels
- [~] Paths / Character / Navigator parity with descriptor catalog (Navigator body + Paths/Character placeholders)

**P2 exit:** **Met.** Split graph / user presets remain polish; Layers/History virtualized.

---

## P3 — Selection & edit targets

Chapters: [01](../01-Information-Architecture.md), [12](../12-Selection-System.md), DR-011

### P3.1 Chrome

- [x] Distinct: object selection vs pixel selection vs focus vs context target vs active edit target
- [x] UI chrome never collapses these into one “selection”
- [~] Commands/announce for each concept (`lastAnnounce` + status; full a11y flood-control later)
- [x] Mask-edit target vs layer pixels clearly indicated
- [~] Selection channel ops (replace/add/subtract/intersect) complete in chrome (tool options present)
- [~] Marching-ants / overlay performance within interactive budgets (GPU ants shipped; SLO evidence → P13)
- [x] Select → mask / mask → selection flows (`selection.to-mask`, `mask.to-selection`)

**P3 exit:** **Met** for shipped concepts. Multi-object select polish deferred.

---

## P4 — Masks & layer semantics

Chapters: [11](../11-Layer-System.md), [13](../13-Mask-System.md)

### P4.1 Masks

- [~] Vector masks (path-based) — metadata + `mask.create-vector`; deep path edit residual
- [x] Refine edge — density/feather/invert/contrast/shift via `mask.set-attributes` + Properties
- [x] Apply mask / disable / delete semantics — `mask.apply` (host bake) + disable/delete + history
- [x] Mask density / invert / link flags in UI (Properties + GPU density equation)
- [x] Paint-on-mask vs layer clarity (edit target)

### P4.2 Layers

- [x] Lock flags enforced (pixels / position / all) on tools
- [x] Multi-select layer ops (delete/reorder/group/ungroup) atomic + Ctrl/Shift Layers panel
- [x] Fill / solid-color layer — `LayerKind::Fill` + `layer.create-fill` / `layer.set-fill-color`
- [x] Layer styles beyond Drop Shadow + Stroke — OuterGlow + ColorOverlay (further styles `[P]`)
- [x] Clipping groups UX — clip toggle + break-on-delete-base + Layers ↳ tooltip
- [x] Nondestructive effect stack ordering UI — `effect.reorder` / `effect.set-enabled` + Properties

**P4 exit:** **Met.** Mask contrast/shift refine shipped; deep vector-mask path edit residual.

---

## P5 — Creative engines depth

Chapters: [14](../14-Brush-Engine.md), [15](../15-Filter-Engine.md), [18](../18-Text-Engine.md), [19](../19-Shape-Engine.md)

### P5.1 Brush

- [x] Dynamics: opacity/flow/scatter/spacing + size/opacity pressure + texture tip strength; full pressure curves residual `[P]`
- [x] Brush preset library persistence + UI (prefs schema 4 + Properties apply/save)
- [x] Stroke journal / recovery hooks (`StrokeJournal` → `recovery/strokes/*.json`)
- [x] CPU dab reference path (`stamp_dab_rgba` / `paint_dabs_rgba`)
- [P] Tile-aware stroke planner (after P11 tiling)

### P5.2 Filters

- [x] Declarative filter / effect plan graph (`FilterPlan` on `Layer`)
- [x] Filter gallery UX (browse + preview + commit) — `filter.preview` / `filter.commit` / dialog
- [x] Additional GPU executors — sharpen + noise CPU/GPU pack + menu/gallery
- [x] Cancel / stale-result policy for long filters — `CancelToken` + generation reject; `filter.cancel-preview`
- [x] Adjustment kinds — brightness/levels/hue/exposure (+ invert/threshold/posterize); fuller set residual `[P]`

### P5.3 Text

- [x] Editable text tool (on-canvas) — Character + bake + TextEdit overlay when Text tool active
- [x] Typography: wrapping + frame bounds (`TextContent.frame_w/h` + `wrap`); more metrics residual
- [x] Font resource discovery + fallback — `fc-list` → `availableFontsJson`; Noto/DejaVu pinned
- [x] Retain editable text vs bake policy UX — Character panel copy + bake announce
- [~] Text → path / rasterize commands explicit — bake path shipped

### P5.4 Shape

- [x] Boolean union / intersection / difference / exclusion — coverage bake → new raster (`shape.boolean`)
- [x] Path edit tool (add/move/delete points, close) — `tool.path-edit` + `path.*` commands
- [x] Parametric primitives — rect/ellipse/line/polygon + gradient fill
- [x] Live vector contribution — `live_vector` re-raster each sync (v1; not tile GPU residency)
- [x] Stroke/fill/gradient style — fill/stroke + linear gradient fill
- [x] Vector-preserving boolean — `ShapeBooleanPartner` on result; raster fallback disclosed

**P5 exit:** **Met.** DR-028 depth spines for brush/filter/text/shape shipped; residual `[P]` = curves / lcms2 / GPU live tiles.

---

## P6 — Color & rendering contracts

Chapters: [16](../16-Color-Management.md), [17](../17-Rendering-Engine.md), DR-005, DR-006, DR-012

### P6.1 Color

- [x] Soft-proof mode + proof intent UI (`document.set-soft-proof`; Image menu; Properties)
- [x] ICC profile load/embed (document + export) — `document.set-icc`; `.ptx` graph field; PNG `iCCP` on export
- [P] Working-space policy beyond built-in sRGB/Display-P3 — assign/convert tags + optional embed; full CMS / lcms2 residual
- [x] Display profile discovery (Linux host adapter) — colord/env/xdg/`sRGB` → soft-proof “Use display profile”
- [~] Assign ≠ convert disclosures — Image menu actions present

### P6.2 Rendering

- [x] Immutable pixel snapshot / bounded delta publisher — `SnapshotPublisher` / `PixelSnapshot` (64 MiB); dense deltas still Provisional
- [~] Workers consume leases only — I/O worker path; full contract deferred
- [x] Broader GPU↔CPU blend/filter parity fixtures — `phototux_gpu::parity` (+ `gpu-tests` device path)
- [x] Device-loss / surface-loss UX (reconstruct or controlled fail) — `GpuError::DeviceLost`/`SurfaceLost`; status + Recover; `renderer_generation`
- [x] Dirty-region / overlay separation polish — `dirty_rect` + view generation; grid clip when cheap
- [!] Tiling / pyramid → **P11** (gated)

**P6 exit:** **Met.** Soft-proof + ICC embed + display discovery + pixel publisher + dirty overlay. Full lcms2 residual `[P]`; tiling → P11.

---

## P7 — History & lifecycle

Chapters: [02](../02-Application-Lifecycle.md), [20](../20-History-Undo.md)

### P7.1 History

- [x] Unified transaction timeline (graph + stroke + selection + transform)
- [~] Coalescing / merge policy documented and tested — coalescing exists; fuller suite deferred
- [x] History panel: labels, kinds, jump (safe) (`history.jump` + host undo loop)
- [x] Retention budget UI — prefs `history_retention_limit` + SpinBox (8–512); `HistoryService::set_limit`
- [!] Spill-to-disk → **P11** (memory evidence)

### P7.2 Lifecycle

- [~] Formal lifecycle controller — session/prefs/recovery hooks; formal controller deferred
- [x] Recovery UX + restore chooser — autosave + startup restore/discard dialog
- [x] Safe-start (suppress custom chrome on crash loop) — `safe_start_next` + `PHOTOTUX_SAFE_START=1`
- [~] Save coordination: staged identity vs generation receipts — generation on graph
- [~] GPU/renderer generation orchestration on device loss — `renderer_generation` + recover shipped; full Event-Catalog lifecycle → later depth (DR-028)
- [x] Multi-doc tabs → **P11** (DR-024 v2); multi-window still deferred

**P7 exit:** **Met.** Timeline + jump + recovery + retention UI + safe-start; spill/multi-doc gated.

---

## P8 — Clipboard & interchange I/O

Chapters: [21](../21-Clipboard.md), [22](../22-Import-Export.md), [27](../27-File-Formats.md)

### P8.1 Clipboard

- [x] Capability-scoped host clipboard bridge — in-app RGBA + OS image via `arboard`
- [~] Multi-format negotiation — pixels image MIME; SVG / layer MIME deferred
- [x] Mask / selection payload copy — selection R8 + layer-mask R8 app payloads; paste as selection/mask
- [x] Security / size bounds (64 MiB refuse on copy)

### P8.2 Import / export

- [x] Hard allocation / dimension / decompression limits (`MAX_DIMENSION` / `MAX_RASTER_BYTES`)
- [~] Structured loss / compatibility reports — PSD truncation messages; broaden deferred
- [~] Cancel + progress contracts — cancel token + I/O busy; full progress UX deferred
- [~] Expand raster codec coverage — PNG/JPEG/WebP/TIFF/BMP/GIF
- [~] PSD subset deepen (layers/masks) with disclosure — subset + limits shipped

### P8.3 Native `.ptx` (non-sparse)

- [~] Migration / schema evolution tests for v2 chunks — v1 read / v2 write tests
- [x] Stronger integrity diagnostics UX — `load_ptx_with_diagnostics` / CRC+magic report in open error dialog
- [x] Unknown optional chunk skip / round-trip tests
- [x] Opaque extension object placeholders — `DocumentGraph::extension_data`
- [!] Sparse tiles / incremental save → **P11**

**P8 exit:** **Met.** OS + mask/selection clipboard, hostile bounds, `.ptx` integrity diagnostics UX. SVG/layer MIME + sparse → deferred/P11.

---

## P9 — Preferences, themes, UX polish

Chapters: [01](../01-Information-Architecture.md), [24](../24-Preferences.md), [25](../25-Themes.md), [28](../28-UX-Guidelines.md)

### P9.1 Preferences

- [~] Handbook preference schema coverage — schema 4: brush presets, density, contrast, motion
- [x] Versioned migrations (schema → 4 on load)
- [x] Effective-value precedence where document vs user differs — `resolve_layered` + `prefEffectiveJson` spine
- [~] Reset field / domain / all — workspace Essentials/factory; full domain reset deferred
- [x] Safe-start prefs path — `safe_start_next` + `PHOTOTUX_SAFE_START=1`

### P9.2 Themes

- [x] Tokens → QML single source (`Theme.qml`)
- [x] High-contrast pack (`prefHighContrast` → `Theme.highContrast`)
- [x] Density / UI scale packs (`prefUiDensity` → `Theme.densityScale`)
- [~] No ad-hoc colors outside tokens — chrome uses Theme; audit stretch

### P9.3 UX

- [x] Mixed-value inspector pattern — opacity/blend show Mixed on multi-select disagreement
- [~] Operation progress / ack patterns — status + I/O busy
- [~] Discoverability: every command via menu or palette — palette + menus for shipped actions
- [~] Reduced-motion + 200% scale audit — prefs flag; full audit deferred
- [x] Progressive disclosure for advanced Properties — advanced color section toggle

**P9 exit:** **Met.** Schema 4 + Theme packs + mixed inspector + safe-start + effective-source spine. Full preference engine / 200% audit → deferred.

---

## P10 — Accessibility

Chapter: [29](../29-Accessibility.md), DR-016

- [x] Semantic accessibility tree from descriptors/commands — `accessibilityTreeJson`
- [x] AT-SPI host adapter mapping — `project_semantic_tree` → `atspiProjectionJson`; Qt Accessible on tool strip + canvas
- [x] Canvas structured summary / explorer — canvas node in tree JSON + Accessible name
- [~] Keyboard-complete workflows (non-gesture) — shortcuts + palette; full parity deferred
- [x] Name/role/state/value on tools, panels, dialogs — QML `Accessible` on tool strip, canvas, primary chrome
- [~] Flood control for announcements — `lastAnnounce` single channel
- [x] Contrast / focus / scale gates — high-contrast pref; AT-SPI evidence fixture shipped
- [P] Full custom AT-SPI D-Bus tree server (beyond Qt Accessible + projection JSON)

**P10 exit:** **Met.** Semantic JSON + AT-SPI projection + evidence fixture + Accessible spine. Custom D-Bus provider residual `[P]`.

---

## P11 — Scale & multi-document (partially ungated)

Chapters: [02](../02-Application-Lifecycle.md), [03](../03-Workspace-System.md), [17](../17-Rendering-Engine.md), [20](../20-History-Undo.md), [27](../27-File-Formats.md)

### Gates (must check before coding)

- [!] Large-doc benchmark proves tiling needed ([DR-006](Decision-Register.md#dr-006--gpu-first-via-wgpu-not-gpu-only)) — recorded; no impl ([DR-029](Decision-Register.md#dr-029--p11p12-remain-gated-no-ungated-impl))
- [x] Explicit amend of [DR-024](Decision-Register.md#dr-024--document-session-model) before multi-doc — **v2 tabs Accepted 2026-07-17**
- [!] Memory-pressure evidence before history spill — recorded; no impl (DR-029)
- [!] Sparse/incremental `.ptx` spike before freezing strategy ([DR-026](Decision-Register.md#dr-026--native-ptx-container-v1)) — recorded; no impl (DR-029)

### P11.1 Tiling / pyramid

- [!] Sparse tile store + residency
- [!] Multiresolution pyramid for navigation
- [!] Brush/filter tile planner
- [!] Eviction that never drops authoritative unsaved state

### P11.2 Multi-document

- [x] Document registry + tabs — `DocumentRegistry` + QML `TabBar` (max 8)
- [~] Per-document mutation serialization (DR-010) — UI-thread serial per active doc; dedicated queues deferred
- [!] Multi-view of one document
- [!] Cross-window document presentation (if product wants)

### P11.3 History spill & `.ptx` sparse

- [!] History spill-to-disk + restore
- [!] Tile-addressable resources in `.ptx`
- [!] Incremental / append save strategy (optional, validated)

**P11 exit:** **Partial Met.** Tabs shipped; tiling/spill/sparse remain gated (DR-029).

---

## P12 — Extension capability seams (gated by need)

Chapter: [23](../23-Plugin-SDK.md), DR-009

- [!] Product need recorded — **no product need; seams only** (DR-029 / DR-009)
- [ ] Contribution manifests (panels/commands/filters) behind capabilities
- [ ] Budgets + failure isolation
- [x] Opaque extension data in document + `.ptx` round-trip — `extension_data`
- [~] Host mediation — opaque store only; no extension host yet
- [N] Stable native ABI / marketplace / cloud plugin store

**P12 exit:** **Partial Met.** Opaque blob seam; ABI remains Deferred.

---

## P13 — Verification & budget promotion

Chapters: [30](../30-Performance.md), [31](../31-Testing.md), [32](../32-Developer-Guide.md), DR-017, DR-022

### P13.1 Testing

- [x] Command-router conformance suite — `command_conformance` (known IDs + meta, soft-proof dirty isolation, lock paint reject)
- [~] Hostile I/O fuzz / limit tests — dimension/alloc unit tests; fuzz deferred
- [x] GPU device-loss suite (or documented skip matrix) — `phototux_gpu` loss/recover tests + ledger skip matrix
- [x] CPU vs GPU tolerance fixtures for claimed ops — `phototux_gpu::parity` (`gpu-tests`)
- [x] A11y evidence pack — semantic + AT-SPI projection fixture; full AT client pack residual `[P]`

### P13.2 Performance

- [x] Fixture harness for input→preview, pan/zoom, composite, boot — `budget_harness` soft CI including present-path proxies
- [~] Promote [Performance Budget Ledger](Performance-Budget-Ledger.md) rows Provisional → Accepted with evidence — CI soft + present-path proxies Accepted; photon GPU present may stay Provisional without display
- [x] CI regression gates for promoted budgets — soft asserts in `budget_harness` tests
- [!] Large-doc benchmark suite → **P11** gate

### P13.3 Developer guide practice

- [~] Contrib checklist: new command + taxonomy + tests — taxonomy kept current for new IDs
- [x] Crate boundary lint/culture (engine no Qt; UI no wgpu)
- [x] Thread/ownership map kept current — shipping-crate implementation table
- [N] 18-crate rename (DR-025)

**P13 exit:** **Met.** Soft CI + present-path proxies; photon GPU present Provisional when no display (DR-017).

---

## Full parity exit criteria

- [~] All non-gated P1–P10 and P13 items `[x]` or explicitly Deferred (DR-028) — **P1–P10/P13 Met**; chapter-depth open under DR-028
- [x] All P11/P12 items either `[x]` after gates **or** `[!]`/`[P]`/`[N]` with DR — gated via DR-029; P12 opaque seam `[x]`
- [~] Gap analysis has no silent MUST contradictions — keep closing rows with slices
- [~] Roadmap §1 “full parity” — **spine parity** reached; DR-028 depth + device budget evidence open
- [x] Phase journals under `archive/docs/04-journal/` — full `handbook-parity-complete` when DR-028 depth + DR-017 device promotion close

---

## Recommended next slices (ungated)

Priority order for agents (see also Roadmap §7):

1. **DR-028 depth** — display ICC / colord; texture tips; on-canvas text/fonts; live vector; full AT-SPI bus (as needed)
2. **DR-017 present evidence** — photon/present B1/B2 on device (CPU proxies shipped)
3. **P11 gated** — tiling/spill/sparse only with evidence; **P12** no plugins yet

**Do not** start P11 tiling/spill/sparse or P12 ABI without gates (DR-029). Multi-doc tabs are ungated (DR-024 v2).

---

## Never / out of product

- [N] Cloud sync, accounts, collaboration
- [N] AI / generative tools
- [N] Electron / web / CLI / TUI product
- [N] Toolkit or GPU API replacement
- [N] Plugin marketplace / stable third-party ABI (until separate DR)
