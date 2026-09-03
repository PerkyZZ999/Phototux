# QA Checklist

A full-repository quality pass over PhotoTux: the Qt 6 QML desktop editor, the
six Rust crates behind it, the build and docs tooling, and the two static Astro
sites.

**Status legend** — `[ ]` not run · `[x]` pass · `[!]` fail, logged in
[QA_ISSUES.md](QA_ISSUES.md) · `[~]` partial or flaky · `[N]` not applicable,
with the reason stated inline.

## How this relates to what already exists

This file is the **execution record of one QA pass**. It does not replace the
standing collateral, and where an item is already covered there, this file says
so rather than restating it:

| Existing | Owns |
|---|---|
| [Interactive-Stability-Checklist](internal_docs/Appendix/Interactive-Stability-Checklist.md) | The living GUI stability checklist and its `T-nnn` issue log |
| [Accessibility-Checklist](internal_docs/Appendix/Accessibility-Checklist.md) | The `A-n` accessibility conformance matrix |
| [31 — Testing](internal_docs/31-Testing.md) | Headless suite policy |
| [Error-Taxonomy](internal_docs/Appendix/Error-Taxonomy.md) | The error vocabulary these tests assert against |

Issues found here are logged in [QA_ISSUES.md](QA_ISSUES.md) with `QA-nnn` ids
and cross-referenced into the stability checklist's `T-nnn` log when they are
GUI-observable.

## A note on "network failures"

PhotoTux is local-first with no network surface: no telemetry, no cloud, no
update check ([DR-001](internal_docs/Appendix/Decision-Register.md#dr-001--local-first-product-boundary)).
The analogous failure modes — an external dependency that can vanish, refuse, or
return garbage mid-operation — are the **filesystem**, the **XDG portal**, the
**GPU device**, and **fontconfig / colord**. Those are tested in §2 in place of
network tests. The two Astro sites are statically generated with no runtime
fetches, so the same applies there.

## How to run

```bash
export PATH=/usr/lib/qt6/bin:$PATH; export QMAKE=/usr/lib/qt6/bin/qmake
```

Headless items run under `cargo test`. GUI items run against
`./target/debug/phototux` in an isolated Wayland session with an isolated home,
so the host's real preferences and autosaves are never touched.

---

# 1. Happy path flows

Core journeys, start to finish. Each is a whole flow, not a single control:
the item passes only if the user could complete it and the shell agreed that
they had.

## 1.1 Document lifecycle

- [ ] **H-01** Cold start with no document → Welcome dialog offers New / Open and lists recent files
- [ ] **H-02** New Document at each preset (720p / 1080p / 2K / 4K) creates a document of that size, zoomed to fit
- [ ] **H-03** New Document at a custom size within limits creates it
- [ ] **H-04** Open a PNG → canvas shows it, layer list shows it, title carries the filename
- [ ] **H-05** Open a `.ptx` → layers, masks, styles and adjustments all return
- [ ] **H-06** Open a layered PSD → layers import, compatibility report discloses what was dropped
- [ ] **H-07** Save a new document (Save As) → file written, title loses the dirty `*`
- [ ] **H-08** Save an already-saved document (Save) → writes in place with no dialog
- [ ] **H-09** Export to each raster format (PNG, JPEG, WebP, TIFF, BMP, GIF)
- [ ] **H-10** Export to PSD → layered file, compatibility report lists anything unrepresentable
- [ ] **H-11** Close a dirty document → prompts before discarding
- [ ] **H-12** Open a second document → tab strip shows both, switching preserves each one's state
- [ ] **H-13** Autosave fires and Recovery restores a document after a simulated unclean exit

## 1.2 Painting and tools

- [ ] **H-14** Brush stroke paints, at 60 fps, with the stroke visible while dragging
- [ ] **H-15** Eraser removes to transparency
- [ ] **H-16** Every one of the 26 tools activates from the shelf, its keyboard chord, and the palette
- [ ] **H-17** Tool options bar changes to match the active tool
- [ ] **H-18** Clone stamp anchors on Alt-click and copies from the anchor
- [ ] **H-19** Fill and Gradient commit to the active layer
- [ ] **H-20** Eyedropper picks into the foreground swatch
- [ ] **H-21** Text tool creates an editable text layer; Bake Text rasterizes it
- [ ] **H-22** Shape tool creates each shape preset; Path Edit moves its anchors

## 1.3 Layers and masks

- [ ] **H-23** New / Duplicate / Delete layer
- [ ] **H-24** Reorder via Layer ▸ Arrange (all four entries and chords)
- [ ] **H-25** Group and Ungroup; a group hides its children when hidden
- [ ] **H-26** Opacity and blend mode apply, and are visible on canvas
- [ ] **H-27** Add / delete / apply a layer mask; paint into it via the edit-target switch
- [ ] **H-28** Clipping mask clips to the layer below
- [ ] **H-29** Merge Down / Merge Visible / Merge Group / Flatten
- [ ] **H-30** Layer styles add, edit and render
- [ ] **H-31** Adjustment layers: all ten kinds create, edit and composite

## 1.4 Selection and transform

- [ ] **H-32** Each selection tool makes a selection (rect, ellipse, lasso, polygon, wand, colour range)
- [ ] **H-33** Combine modes: replace, add, subtract, intersect
- [ ] **H-34** Select All / Deselect / Invert
- [ ] **H-35** Modify: expand, contract, feather, border
- [ ] **H-36** Selection ↔ mask conversion both ways
- [ ] **H-37** Free Transform: move, scale, rotate, constrain, Apply and Cancel
- [ ] **H-38** Crop commits and discards the outside

## 1.5 History, workspace, colour

- [ ] **H-39** Undo/redo across every mutating command, including the two host-side stacks
- [ ] **H-40** History panel lists entries and jumping to one restores that state
- [ ] **H-41** Workspace presets apply; Reset Workspace restores defaults
- [ ] **H-42** Panels toggle, tear off, re-dock, auto-hide and resize; state survives restart
- [ ] **H-43** Preferences persist across restart
- [ ] **H-44** Assign vs Convert profile behave differently and both are reachable
- [ ] **H-45** Soft-proof toggles without dirtying the document

## 1.6 Non-GUI surfaces

- [x] **H-46** `rust-tc quick` and `rust-tc doctor` pass from a clean tree — 697 tests
- [x] **H-47** `scripts/check-docs-links.py` reports zero broken links — 70 pages
- [x] **H-48** Both Astro sites build, and the docs search index generates — **failed first**: two malformed `<img>` tags broke the landing build; fixed, and `scripts/check-web.sh` added so it cannot recur
- [x] **H-49** The git pre-commit hook installs and runs — `core.hooksPath=.githooks`, runs on every commit in this pass

---

# 2. Edge cases and failure modes

## 2.1 Boundary values

- [x] **E-01** Document dimension at exactly `MAX_DOCUMENT_DIMENSION` (8192) is accepted; 8193 is refused with a message naming the limit
- [x] **E-02** Document dimension of 0 or 1 is handled or refused, not crashed — **failed**: `check_size` only tested the upper bound, so a `.ptx` or PSD declaring 0x0 opened as a degenerate document. Fixed
- [x] **E-03** Layer count at the composite cap (`MAX_LAYERS`); one more is refused — stops at 16 with `LayerLimitReached`
- [x] **E-04** Group nesting at `MAX_NESTING_DEPTH` (64); one deeper is refused — unreachable in practice: the 16-layer cap stops nesting at 14, which is what the constant's own comment predicts
- [ ] **E-05** Every adjustment slot at both ends of its declared range
- [ ] **E-06** Blur radius at `MAX_BLUR_RADIUS` and at 0
- [x] **E-07** Opacity at 0 and 1; zoom at min and max — opacity clamps at the layer, including out-of-range input. Zoom **failed**: `f32::clamp` propagates NaN, so `set_zoom(NaN)` stored NaN and the camera could not be recovered. Fixed at all three camera entries
- [ ] **E-08** Selection of zero area, and one covering the whole canvas
- [ ] **E-09** Undo stacks at their bounds (64 selection, 32 transform) — the oldest step falls off, nothing panics
- [x] **E-10** Text with an empty body, and with a very long single word — both accepted, 100 000 characters included

## 2.2 External-dependency failures (the "network" analogue)

- [ ] **E-11** GPU device lost mid-session → document survives, Recover restores the canvas
- [ ] **E-12** Save to a read-only directory → refused with a message, document stays dirty, no partial file
- [ ] **E-13** Save to a path whose parent does not exist
- [ ] **E-14** Disk full during save → atomic replace leaves the previous file intact
- [ ] **E-15** Open a file deleted between the dialog and the read
- [ ] **E-16** Portal file dialog cancelled at every point it can be
- [ ] **E-17** `fc-list` unavailable → font list falls back rather than hanging
- [ ] **E-18** colord unavailable → display profile falls back to sRGB
- [ ] **E-19** A long save can be cancelled, and cancelling leaves no partial file

## 2.3 Malformed and missing state

- [x] **E-20** Truncated PNG / JPEG / `.ptx` / PSD → parse error, not a panic — empty, one byte, header-only, half and pure garbage all refused with typed errors
- [x] **E-21** `.ptx` from a future version → refused with a version message — names both the file's version and the build's
- [x] **E-22** `.ptx` with an unknown layer kind or extension blob → round-trips or refuses cleanly — covered by `extension_blob_roundtrips_in_graph_json`
- [ ] **E-23** PSD with features outside the subset → imports what it can, discloses the rest
- [x] **E-24** A file whose extension lies about its content — PNG to the `.ptx` reader, `.ptx` to the PSD reader and to the raster decoder: each refused by content, not extension
- [x] **E-25** Corrupt `preferences.json` → falls back to defaults rather than refusing to start — `unwrap_or_default`, and the sparse-file case is covered by four existing tests
- [x] **E-26** Corrupt workspace / dock topology JSON → falls back to the default layout — `from_json` validates then falls back to `essentials()`
- [x] **E-27** A recovery entry pointing at a file that no longer exists — restore reports a clean error and the entry can still be discarded. **Found worse nearby**: one *unreadable* entry aborted the whole listing, so a single half-written file offered no recovery at all. Fixed
- [x] **E-28** Zero-byte file offered to every open path — refused by all three readers

## 2.4 Permission and command-precondition errors

- [!] **E-29** Every lock flag (pixels, position, alpha, all) refuses the edits it should, with a message — pixels and position are correct. **Lock All permits opacity, blend and effects** ([QA-001](QA_ISSUES.md#qa-001--lock-all-does-not-block-the-three-things-that-restyle-a-layer)); the alpha lock is unreachable and unread ([QA-002](QA_ISSUES.md#qa-002--the-transparency-lock-is-state-nothing-sets-and-nothing-reads))
- [x] **E-30** Every command invoked with no document open — typed errors throughout, no panics
- [x] **E-31** Every command invoked with no active layer — typed refusals throughout
- [~] **E-32** Commands that need a selection, invoked without one — `selection.to-mask` and `mask.to-selection` refuse. `selection.invert` and `selection.modify` succeed as no-ops; defensible (inverting nothing is Select All) and the menu entries are enablement-gated, so not logged
- [x] **E-33** Commands that need a specific layer kind, invoked on the wrong kind — all five refuse in plain English
- [x] **E-34** Merge Down on the bottom layer; Merge Group outside a group — both refuse, as does Ungroup
- [ ] **E-35** Every enablement tag actually disables its menu entry when false

## 2.5 Concurrency and ordering

- [ ] **E-36** Rapid tool switching mid-stroke
- [ ] **E-37** Undo during an in-flight save
- [ ] **E-38** Closing a document while its file operation is running
- [ ] **E-39** Switching document tabs mid-stroke
- [ ] **E-40** Opening a dialog while another is open
- [ ] **E-41** No re-entrant host-slot call from a QML signal handler (handbook 32)

---

# 3. UI / UX consistency

## 3.1 State indicators

- [ ] **U-01** Every panel has an empty state — never a blank rectangle
- [ ] **U-02** Long operations show a busy indicator and a cancel affordance
- [ ] **U-03** Errors reach a toast that does not auto-dismiss; info and warnings fade
- [ ] **U-04** Disabled controls look disabled and say why on hover where non-obvious
- [ ] **U-05** The dirty marker appears on the first edit and clears on save
- [ ] **U-06** No message is written into the status bar, which carries state only

## 3.2 Layout and density

- [ ] **U-07** All three densities (Compact, Comfortable, Dense) render without clipping or overlap
- [ ] **U-08** The shell holds together from 1280×720 up to 4K
- [ ] **U-09** No dialog pins itself to a pixel width
- [ ] **U-10** Every dialog is reachable, dismissable by Escape, and returns focus
- [ ] **U-11** Panel resize seams behave at their extremes
- [ ] **U-12** High-contrast and reduced-motion preferences take effect

## 3.3 Visual language

- [ ] **U-13** No unstyled Qt Controls reach the user
- [ ] **U-14** Every icon resolves; no blank buttons
- [ ] **U-15** Colours come from `Theme.qml`; no second palette
- [ ] **U-16** Photoshop-consistent placement for every panel, tool and menu entry
- [ ] **U-17** Every user-facing string is `qsTr(...)` and free of internal jargon

## 3.4 Accessibility basics

- [ ] **U-18** Every interactive control has an accessible name
- [ ] **U-19** Keyboard-only operation reaches every primary flow
- [ ] **U-20** Focus is always visible and its order follows the layout
- [ ] **U-21** Live regions announce without flooding
- [ ] **U-22** No meaning conveyed by colour alone

## 3.5 Web surfaces

- [ ] **U-23** Landing page loads at the top and both themes render
- [ ] **U-24** Docs site navigation, search and table of contents work
- [ ] **U-25** Both sites are usable from 320 px to 1440 px+
- [ ] **U-26** The public shortcut reference matches the shipped registry
- [ ] **U-27** No broken internal links or missing assets on either site

---

## Pass log

| Date | Scope | Result |
|---|---|---|
| 2026-09-03 | §1.6 non-GUI surfaces | 4/4 pass; H-48 failed first and was fixed |
