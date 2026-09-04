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

- [x] **H-01** Cold start with no document → Welcome dialog offers New / Open and lists recent files
- [x] **H-02** New Document at each preset (720p / 1080p / 2K / 4K) creates a document of that size, zoomed to fit — all four presets present with correct dimensions; 1080p created 1920×1080 at 53% fit
- [x] **H-03** New Document at a custom size within limits creates it — and typing 99999 clamps to 8192, deselecting the preset (E-01 through the GUI)
- [x] **H-04** Open a PNG → canvas shows it, layer list shows it, title carries the filename —
      640×360 PNG opened at zoom-to-fit 160%, layer named `qa-open.png`, title
      `qa-open.png — PhotoTux`, tab clean
- [~] **H-05** Open a `.ptx` → layers, masks, styles and adjustments all return — a raster
      layer and an Invert adjustment round-tripped, editor and composite intact. Marked
      partial only because the reopened document arrives already flagged as modified
      ([QA-011](QA_ISSUES.md)); the content itself came back correctly
- [x] **H-06** Open a layered PSD → layers import, compatibility report discloses what was
      dropped — a PSD exported from PhotoTux reopened with both its raster layers and the
      composite intact, and a Compatibility report dialog naming `[psd.subset]` and
      `[psd.effects]`. The `[psd.effects]` line is shown for every layered PSD rather than
      only for files that carry styles or vector masks — a blanket disclosure trains users
      to dismiss it, but it is not wrong
- [x] **H-07** Save a new document (Save As) → file written, title loses the dirty `*` —
      wrote `qa-doc.ptx` (2532 bytes); title became `qa-doc.ptx — PhotoTux` and the tab
      lost its asterisk
- [x] **H-08** Save an already-saved document (Save) → writes in place with no dialog —
      Ctrl+S rewrote the same path (2532 → 2786 bytes) with no dialog, and cleared the
      dirty marker from both the title and the tab
- [x] **H-09** Export to each raster format (PNG, JPEG, WebP, TIFF, BMP, GIF) — **failed
      first**: the Export dialog offered four of the six. `phototux_io` writes all six;
      the filter list was hand-written in QML and had gone stale, so BMP and GIF could
      be opened and never saved. The list is now published from `RasterFormat::ALL` and
      a GIF exported at 640×360 carries the adjustment layer's effect
- [x] **H-10** Export to PSD → layered file, compatibility report lists anything
      unrepresentable — a four-layer document (text, shape, raster, background) wrote a
      24 MB layered PSD carrying its two raster layers, and the report said exactly which
      two were left out: "Only raster layers are written to PSD. These were left out:
      Shape layer, Text layer. Rasterize them first to keep them." Confirmed with an
      external parse: the file's layer count is 2
- [x] **H-11** Close a dirty document → prompts before discarding — Ctrl+W on a painted
      document raises "Unsaved changes · Save the document as .ptx, discard changes, or
      cancel?" with Save / Discard / Cancel. An earlier close that did *not* prompt was
      correct: the document was clean and its tab was lying about it (T-040, fixed)
- [x] **H-12** Open a second document → tab strip shows both, switching preserves each
      one's state — switching back to a four-layer document restored its layers, its
      active tool, the on-canvas text frame and the Character panel. The strip reorders
      itself as you switch, which is [QA-011](QA_ISSUES.md)
- [x] **H-13** Autosave fires and Recovery restores a document after a simulated unclean exit —
      exercised repeatedly across this pass: every `kill` of the app was followed by a
      Recover dialog listing the autosaves with per-entry Restore/Discard and Discard All,
      and Restore brought back the full layer stack. The empty state ("Nothing left to
      recover") is correct too

## 1.2 Painting and tools

- [x] **H-14** Brush stroke paints, at 60 fps, with the stroke visible while dragging — 60 fps, comp 0.01 ms, navigator thumbnail follows
- [x] **H-15** Eraser removes to transparency
- [x] **H-16** Every one of the 26 tools activates from the shelf, its keyboard chord, and the palette — all three chord forms land (`e`, `Shift+M`, `Ctrl+T`); the rail's 15 buttons are all AT-named, and `the_tool_rail_and_the_tool_vocabulary_describe_the_same_tools` covers the set
- [x] **H-17** Tool options bar changes to match the active tool — Brush, Rectangular Marquee, Free Transform and Paint Bucket each show their own controls
- [x] **H-18** Clone stamp anchors on Alt-click and copies from the anchor — Alt-click on a
      stroke then dragging 280 px below it painted a copy of the source at the same offset.
      Its options bar is empty, where Photoshop offers aligned/sample and the brush
      controls, and nothing on screen says Alt-click is how the anchor is set
- [x] **H-19** Fill and Gradient commit to the active layer — **the bucket's own colour field was wrong**: it showed and edited a *fill layer's* colour while the bucket poured the foreground. Fixed and guarded
- [x] **H-20** Eyedropper picks into the foreground swatch — filled `#CC3366`, set the
      foreground to `#111111`, sampled the canvas, and the field read `#CC3366` again
- [~] **H-21** Text tool creates an editable text layer; Bake Text rasterizes it
      — the layer is created and is editable from both the on-canvas frame and the
      Character panel (they mirror each other live), and Bake Text converts it to a
      raster layer with the notice "Text baked to pixels — editable text discarded".
      Two defects: the creating click's position is discarded ([QA-007](QA_ISSUES.md)),
      and the bake uses a 5×7 bitmap alphabet rather than the previewed face
      ([QA-008](QA_ISSUES.md)). A third, found here and fixed in place, was worse
      than either: after the bake every single-key tool shortcut was dead until
      something else was clicked, because the hidden on-canvas editor kept the
      keyboard — see T-037 in the
      [Interactive-Stability-Checklist](internal_docs/Appendix/Interactive-Stability-Checklist.md).
- [~] **H-22** Shape tool creates each shape preset; Path Edit moves its anchors
      — `Layer ▸ Shape` lists all nine presets and Star built correct geometry with
      fill, stroke and inspector W/H/X/Y; Rasterize Shape was correctly disabled on a
      raster layer. Path Edit does move an anchor (dragging a rectangle's corner
      deformed it and the inspector followed), but the layer was named for its kind
      rather than the preset — fixed here, commit `d779b06` — and the anchors are
      never drawn ([QA-009](QA_ISSUES.md)).

## 1.3 Layers and masks

- [x] **H-23** New / Duplicate / Delete layer — Ctrl+Shift+N and Ctrl+J both land, layer count follows
- [x] **H-24** Reorder via Layer ▸ Arrange (all four entries and chords) — all four entries present with their Photoshop chords; Send to Back moved a group below Background and carried its child
- [x] **H-25** Group and Ungroup; a group hides its children when hidden — New Group nests the layer with a `G` badge and the blend combo switches to Pass Through
- [x] **H-26** Opacity and blend mode apply, and are visible on canvas — the slider reached 12% and reports "Layer opacity percent 12" to AT
- [x] **H-27** Add / delete / apply a layer mask; paint into it via the edit-target switch — Add Mask switches the edit target to "Layer mask", exposes the four mask sliders, and enables Apply/Delete
- [x] **H-28** Clipping mask clips to the layer below — Layer ▸ Create Clipping Mask indents
      the row with the clip arrow and the shape disappears because its base layer's only
      opaque pixels lie outside it; Ctrl+Z brings it back, so the clip is what removed it
- [x] **H-29** Merge Down / Merge Visible / Merge Group / Flatten — all four exercised on a
      four-layer document: Merge Down folded a shape into the raster below it and kept the
      lower name, Merge Visible collapsed to one layer with the composite unchanged, Merge
      Group flattened a group to a raster layer, Flatten Image reduced to `Background`.
      Each is a separate History entry, and all four correctly grey out at one layer
- [x] **H-30** Layer styles add, edit and render — Layer ▸ Layer Style lists eight kinds;
      Color Overlay applied, rendered over the layer, and was recorded in History as a
      graph edit. The Layer Styles disclosure group carries the editors (it is priority 3
      and closed by default, so it sits below the fold on a short panel)
- [x] **H-31** Adjustment layers: all ten kinds create, edit and composite — Layer ▸ New
      Adjustment Layer lists all ten (Brightness/Contrast, Levels, Exposure,
      Hue/Saturation, Invert, Threshold, Posterize, Vibrance, Black & White, White
      Balance). Threshold created a live adjustment layer with an `A` badge and a Level
      slider; dragging it to 0.00 whitened the whole composite, so the GPU canvas honours
      it. A thumbnail that briefly disagreed with the canvas mid-drag settled to match —
      it was an intermediate snapshot, not a CPU/GPU divergence

## 1.4 Selection and transform

- [~] **H-32** Each selection tool makes a selection (rect, ellipse, lasso, polygon, wand, colour range) — rect and ellipse verified; the remaining four not exercised
- [x] **H-33** Combine modes: replace, add, subtract, intersect — Shift-drag unions the rects and the Mode row switches to Add live, with the "Shift add · Alt subtract" hint shown
- [x] **H-34** Select All / Deselect / Invert — Ctrl+A, Ctrl+Shift+I and Ctrl+D all land
- [x] **H-35** Modify: expand, contract, feather, border — the prompt opens with the registry's default radius and Expand applies. A radius above the default used to block the UI thread for minutes ([QA-006](QA_ISSUES.md#qa-006--select--modify-blocks-the-ui-thread-for-minutes)); **fixed** — 38× faster at radius 40, and the exact reproduction now completes
- [x] **H-36** Selection ↔ mask conversion both ways — **failed first**: Selection to Mask
      wrote the mask and left the canvas showing the composite from before it, so the
      command looked like a no-op. Fixed in `T-039`; re-verified both directions —
      Selection to Mask now masks the layer the moment it runs, and Mask to Selection
      restores a pixel selection after Ctrl+D has cleared one
- [~] **H-37** Free Transform: move, scale, rotate, constrain, Apply and Cancel — arming it
      shows the options bar with Rotate and Constrain; the first drag begins the session
      ("Transform in progress", Apply/Cancel live, a Rotate slider and the hint "Drag to
      move · handles scale · Enter apply · Esc cancel"). Enter applied a move, Esc
      cancelled the next one and the layer returned. The bounding box is drawn unclipped
      and overlaps the tab strip when the layer is moved up ([QA-010](QA_ISSUES.md))
- [x] **H-38** Crop commits and discards the outside — dragging showed the kept region and
      "Crop 1220 × 619" in the options bar; Apply resized the document to 1220×619, re-fit
      the zoom to 84% and clipped the stroke at the new edge

## 1.5 History, workspace, colour

- [!] **H-39** Undo/redo across every mutating command, including the two host-side
      stacks — **three gaps found in the colour submenu**. A new conformance test walks
      `default_actions()`, invokes each action's command, and for every one that
      changed the serialised graph asserts an undo entry was recorded, that undo
      restores the document exactly, and that redo puts it back; 42 actions reach the
      graph. Assign Profile edited the document and recorded nothing. Embed/Clear ICC
      recorded a `Transform` entry, which routes undo to the host's transform snapshot
      stack — a stack it never wrote to — so undoing it would have restored an
      unrelated layer-pixel commit and left the profile embedded. Both fixed: a new
      `GraphCommand::SetColorState` carries the document's colour state, and both
      commands record it. Convert to Profile is the third and is not a correction —
      it rewrites every pixel on the GPU with no snapshot ([QA-014](QA_ISSUES.md)) —
      so it is named in the test's `NOT_UNDOABLE` list rather than skipped silently.
      Soft-proof is named there too: it is view chrome stored in the graph, and
      Photoshop's Proof Colors is not undoable either
- [x] **H-40** History panel lists entries and jumping to one restores that state — entries carry their scope, the undone one is dimmed, and clicking "Add layer" returned the document to 3 layers
- [x] **H-41** Workspace presets apply; Reset Workspace restores defaults — **failed first**:
      Reset Workspace announced itself but left an auto-hidden panel hidden. Fixed in
      `c8e300e` (T-038) and re-verified: auto-hide Properties, reset, panel returns
- [~] **H-42** Panels toggle, tear off, re-dock, auto-hide and resize; state survives
      restart — all six work and none aborts the process (T-027's class). Two findings:
      the seam **under-travelled**, moving 60 px for a 120 px drag because it measured
      the pointer against an item its own resize had moved — fixed in `1d94112`, now
      118 px for 120, and the height survives a restart. And a torn-off panel is a
      window containing a message rather than the panel ([QA-012](QA_ISSUES.md))
- [x] **H-43** Preferences persist across restart — "Show rulers" and "High contrast
      chrome", both off by default, were still on after a relaunch
- [x] **H-44** Assign vs Convert profile behave differently and both are reachable — both
      sit under Image ▸ Color. Assign Profile: Display-P3 left the pixels byte-identical
      (`#2060C8` before and after); Convert to sRGB rewrote them (`#2060C8` → `#0062CF`,
      `#DC285A` → `#F00058`) and warned "Converted pixels to sRGB (from Display-P3) — this
      rewrote layer data" as a toast
- [x] **H-45** Soft-proof toggles without dirtying the document — Image ▸ Color ▸
      Soft-Proof: Display-P3 on a freshly created document left the title reading
      `Untitled — PhotoTux` with no dirty marker, and the state is disclosed as
      "Soft-proof: Display-P3" in the Document properties

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
- [!] **E-05** Every adjustment slot at both ends of its declared range — **two findings**. A NaN slot produced NaN pixels in all ten kinds; fixed at the command and in `clamped()`. And three slots' editor ranges are narrower than what `clamped()` keeps ([QA-004](QA_ISSUES.md#qa-004--an-adjustments-editor-range-and-its-clamp-disagree))
- [x] **E-06** Blur radius at `MAX_BLUR_RADIUS` and at 0 — both ends accepted, and the command refuses without a blur effect present
- [x] **E-07** Opacity at 0 and 1; zoom at min and max — opacity clamps at the layer, including out-of-range input. Zoom **failed**: `f32::clamp` propagates NaN, so `set_zoom(NaN)` stored NaN and the camera could not be recovered. Fixed at all three camera entries
- [!] **E-08** Selection of zero area, and one covering the whole canvas — zero area refused, one pixel and whole-canvas accepted, past-the-edge kept whole. **A marquee entirely off-canvas reports a selection covering no pixels** ([QA-005](QA_ISSUES.md#qa-005--a-selection-entirely-off-canvas-reports-itself-as-a-selection))
- [x] **E-09** Undo stacks at their bounds (64 selection, 32 transform) — the oldest step falls off, nothing panics — `the_oldest_step_falls_off_the_bottom` covers it on the shared `HostUndoStack`
- [x] **E-10** Text with an empty body, and with a very long single word — both accepted, 100 000 characters included

## 2.2 External-dependency failures (the "network" analogue)

- [x] **E-11** GPU device lost mid-session → document survives, Recover restores the canvas — the painted stroke, layers and dirty state all survive; a red error toast names the loss, the Recover control appears in the status bar, and using it announces "Graphics recovered — canvas restored" and hides itself again
- [x] **E-12** Save to a read-only directory → refused with a message, document stays dirty, no partial file — `Permission denied (os error 13)` carried through, no temp left behind
- [x] **E-13** Save to a path whose parent does not exist — refused with the OS reason
- [x] **E-14** Disk full during save → atomic replace leaves the previous file intact — covered by `a_failed_write_leaves_the_previous_contents_intact` and `a_failed_write_leaves_no_temporary_behind`; a genuine ENOSPC was not simulated
- [x] **E-15** Open a file deleted between the dialog and the read — refused with `No such file or directory`. **Found alongside**: a 0-edge raster was refused as "dimensions exceed the 32,768 pixel limit". Fixed
- [x] **E-16** File dialog cancelled at every point it can be — **one gap found and
      fixed**. No portal runs in the isolated session, so this exercised Qt's own
      Quick dialog, which is the fallback a user without a portal also gets. Cancel
      leaves nothing behind at any of the four points: Ctrl+S on an untitled document
      (no stuck "Saving…", no `ioBusy` lock), Ctrl+O with a document open, and Save
      or Cancel from the unsaved-changes prompt. Cancelling the save that a Ctrl+W
      opened correctly abandons the close — the document and its stroke survive.
      **The gap was the other branch**: answering the prompt with Save wrote the file
      and then stopped, leaving the document open; File ▸ Quit ▸ Save left the
      application running. Nothing resumed the parked action, because a save lands
      asynchronously long after the button handler returns. Fixed via a new
      `documentSaved` signal, with the dialog's own cancel clearing the parked action
      so it cannot be picked up by an unrelated Ctrl+S later. Both verified live: the
      document saves and closes, and after a cancel an ordinary save leaves it open.
      The I/O error dialog was exercised on the way through, on a save into a folder
      that does not exist — it reports the real message and the document survives
- [x] **E-17** `fc-list` unavailable → font list falls back rather than hanging — a spawn failure and a non-zero exit both return `FALLBACK_FONTS`
- [x] **E-18** colord unavailable → display profile falls back to sRGB — the probe is wrapped in `timeout 1s` so a stuck session bus cannot hang startup, then falls through env → xdg → tagged sRGB
- [x] **E-19** A long save can be cancelled, and cancelling leaves no partial file — cancel is wired through `CancelToken` and guarded by `a_running_file_operation_can_be_cancelled`; the atomic-write tests cover the no-partial-file half

## 2.3 Malformed and missing state

- [x] **E-20** Truncated PNG / JPEG / `.ptx` / PSD → parse error, not a panic — empty, one byte, header-only, half and pure garbage all refused with typed errors
- [x] **E-21** `.ptx` from a future version → refused with a version message — names both the file's version and the build's
- [x] **E-22** `.ptx` with an unknown layer kind or extension blob → round-trips or refuses cleanly — covered by `extension_blob_roundtrips_in_graph_json`
- [x] **E-23** PSD with features outside the subset → imports what it can, discloses the
      rest — covered by H-06: the import took the raster layers and disclosed the subset
      boundary and the unimported effects rather than failing
- [x] **E-24** A file whose extension lies about its content — PNG to the `.ptx` reader, `.ptx` to the PSD reader and to the raster decoder: each refused by content, not extension
- [x] **E-25** Corrupt `preferences.json` → falls back to defaults rather than refusing to start — `unwrap_or_default`, and the sparse-file case is covered by four existing tests
- [x] **E-26** Corrupt workspace / dock topology JSON → falls back to the default layout — `from_json` validates then falls back to `essentials()`
- [x] **E-27** A recovery entry pointing at a file that no longer exists — restore reports a clean error and the entry can still be discarded. **Found worse nearby**: one *unreadable* entry aborted the whole listing, so a single half-written file offered no recovery at all. Fixed
- [x] **E-28** Zero-byte file offered to every open path — refused by all three readers

## 2.4 Permission and command-precondition errors

- [!] **E-29** Every lock flag (pixels, position, alpha, all) refuses the edits it should,
      with a message — pixels and position were correct. **Lock All permitted opacity,
      blend and effects** ([QA-001](QA_ISSUES.md#qa-001--lock-all-does-not-block-the-three-things-that-restyle-a-layer)):
      now fixed, and the same list greys the menus and the panel controls, so nothing
      moves and snaps back. Two more surfaced once the state was visible — the lock
      buttons showed no checked state, and Lock All left pixels and position locked
      when turned off; both fixed. The alpha lock is still unreachable and unread
      ([QA-002](QA_ISSUES.md#qa-002--the-transparency-lock-is-state-nothing-sets-and-nothing-reads))
- [x] **E-30** Every command invoked with no document open — typed errors throughout, no panics
- [x] **E-31** Every command invoked with no active layer — typed refusals throughout
- [~] **E-32** Commands that need a selection, invoked without one — `selection.to-mask` and `mask.to-selection` refuse. `selection.invert` and `selection.modify` succeed as no-ops; defensible (inverting nothing is Select All) and the menu entries are enablement-gated, so not logged
- [x] **E-33** Commands that need a specific layer kind, invoked on the wrong kind — all five refuse in plain English
- [x] **E-34** Merge Down on the bottom layer; Merge Group outside a group — both refuse, as does Ungroup
- [x] **E-35** Every enablement tag actually disables its menu entry when false — Apply/Delete/Toggle Mask greyed with no mask; Merge Group, Ungroup and Bake Text greyed on a plain raster layer

## 2.5 Concurrency and ordering

- [x] **E-36** Rapid tool switching mid-stroke — pressed `E` then `M` during a held brush
      drag and released over the canvas: the stroke ended where the first switch happened,
      the tool became the last one pressed, no marquee was created from the tail of the
      drag, and the process stayed healthy (state `S`, 9% CPU, clean log)
- [~] **E-37** Undo during an in-flight save — the race was **not reproduced**: a `.ptx`
      write of a 1080p document finishes faster than injected input can interleave, and
      saying it passed on that basis would be a claim, not a check. The machinery it
      depends on is now covered instead:
      `a_save_that_finishes_after_an_edit_does_not_report_the_document_clean` holds that a
      save pinned to an older generation answers `false`, which is what leaves the
      document dirty. Watched failing against a `mark_persisted` that always returned true
- [x] **E-38** Closing a document while its file operation is running — answered by
      enablement rather than by a race: `action.file.close` is registered under the
      `has_document_io_idle` predicate, one of nineteen actions gated that way, so Close
      is unavailable for the duration of the operation
- [x] **E-39** Switching document tabs mid-stroke — dragging a brush stroke off the canvas
      and releasing it over the tab strip neither switched tabs nor crashed; a deliberate
      click afterwards switched cleanly and each document kept its own size, zoom,
      selection and layers
- [x] **E-40** Opening a dialog while another is open — with New Document up, Ctrl+O did
      nothing: the modal blocks the shortcut rather than stacking a second dialog
      (T-018's fix). Escape dismissed the one that was open
- [x] **E-41** No re-entrant host-slot call from a QML signal handler (handbook 32) — the
      eleven `Connections { target: AppSession }` blocks are clean, and
      `a_handler_for_a_host_signal_does_not_call_the_host_back` now fails on any host-slot
      call made directly inside one; watched failing against a planted call. The other
      shape — a reactive `on…Changed:` on a non-AppSession object, which is what T-028
      was — is held by `refreshShortcutYield` deferring inside the function

---

# 3. UI / UX consistency

## 3.1 State indicators

- [x] **U-01** Every panel has an empty state — never a blank rectangle — Properties, Layers, History and Welcome/recent all carry one
- [~] **U-02** Long operations show a busy indicator and a cancel affordance — `a_running_file_operation_can_be_cancelled` guards the call and its `ioBusy` gate; not exercised against a genuinely slow operation
- [x] **U-03** Errors reach a toast that does not auto-dismiss; info and warnings fade — an I/O failure raises a modal "File operation failed" naming the cause; the info toast carries a dismiss control
- [x] **U-04** Disabled controls look disabled and say why on hover where non-obvious — greyed entries throughout the Layer menu, matching the active layer's kind and state
- [~] **U-05** The dirty marker appears on the first edit and clears on save — appears: title `Untitled*`, tab `* Untitled`, status `Unsaved`. Clear-on-save not exercised (portal dialog)
- [x] **U-06** No message is written into the status bar, which carries state only — **failed**: three startup writers assigned through local bindings the guard could not see, so after a failed startup open the bar kept reading "Opening …". Fixed, and the guard widened to any binding

## 3.2 Layout and density

- [~] **U-07** All three densities (Compact, Comfortable, Dense) render without clipping or overlap — Dense and Comfortable verified: no overlap, panels scroll rather than clip, and the tool rail engages its overflow as capacity drops. Compact not yet exercised
- [x] **U-08** The shell holds together from 1280×720 up to 4K — both ends exercised in
      their own KWin session. At 1280×720 the window fills the screen, the tool rail
      engages its `…` overflow, the right dock keeps all three groups, the New
      Document dialog fits inside the shell, and a 1080p document opens at 46% zoom
      with the options bar and status bar intact. At 3840×2160 maximised, nothing
      stretches that should not: the welcome card and the New Document dialog keep
      their natural size and stay centred, the tool rail shows all seventeen tools
      with no overflow, and a 4K document opens at 84% zoom with Properties,
      Navigator and Layers all readable
- [x] **U-09** No dialog pins itself to a pixel width — `no_dialog_pins_itself_to_a_pixel_width` green
- [x] **U-10** Every dialog is reachable, dismissable by Escape, and returns focus — nine
      exercised end to end: New Document, Image Size, Canvas Size, Filter Gallery,
      Feather Selection, Preferences, Command Palette, About, and Unsaved changes.
      Each opened from its Photoshop home, closed on Escape, and handed focus back —
      proved each time by a bare tool shortcut landing straight after (`m` →
      `tool.select.rect`, `b` → `tool.brush`), which only reaches the shell if the
      window has focus. Escape on Unsaved changes cancels rather than discards: the
      document and its selection survive. Three are state-driven and open no other
      way — recovery, compatibility report, and I/O error; the first and last are
      driven imperatively by `open()`. The compatibility dialog is the one driven by
      a `visible:` binding a close would write to, so a standalone Qt 6 probe checked
      whether `Popup.close()` breaks that binding and leaves the dialog unable to
      reopen: it does not — closed, then re-shown when the property changed again
- [!] **U-11** Panel resize seams behave at their extremes — the minimum behaves: the
      panel keeps its header and its scroll bar and nothing is lost. The maximum does
      not: one drag to the bottom of the screen makes the panel above fill the dock and
      the four panels below it vanish, with nothing on screen saying where they went
      ([QA-013](QA_ISSUES.md)). Reset Workspace brings them back, which is itself a
      re-verification of `c8e300e`
- [x] **U-12** High-contrast and reduced-motion preferences take effect — **both failed in
      part**. High contrast does take effect and is measurable (panel headers lift from
      `#131315` to `#1F1F23`, borders from `#1A1A1D` to `#2B2B30`). Reduced motion reached
      two of five animated surfaces; the busy indicator spun forever and the selection's
      marching ants crawled regardless. Fixed in `fefbe02` and verified: with the
      preference on, the selection outline is pixel-identical across two seconds

## 3.3 Visual language

- [x] **U-13** No unstyled Qt Controls reach the user — `no_unstyled_controls_reach_the_user` and `no_attached_tool_tips_reach_the_user` both green
- [x] **U-14** Every icon resolves; no blank buttons — `every_icon_key_is_packaged_into_the_qrc` checks both directions, panels included
- [~] **U-15** Colours come from `Theme.qml`; no second palette — panel chrome is tokenised. Six canvas-overlay colours are literals ([QA-003](QA_ISSUES.md#qa-003--canvas-overlay-colours-are-a-second-palette)); not swapped mechanically because none is an exact substitute and doing so would change what the user sees
- [x] **U-16** Photoshop-consistent placement for every panel, tool and menu entry — Layer menu order matches Photoshop's, Arrange carries Photoshop's four chords, tool rail is Photoshop's slot order
- [x] **U-17** Every user-facing string is `qsTr(...)` and free of internal jargon — no untranslated `text:`/`title:`/`placeholderText:` literals in `qml/`

## 3.4 Accessibility basics

- [x] **U-18** Every interactive control has an accessible name — icon-only tool buttons, sliders and combo boxes each have a guard; AT-SPI queries in this pass returned named elements throughout
- [x] **U-19** Keyboard-only operation reaches every primary flow — driven with no pointer
      at all: `Alt+F` opens the File menu and the arrow keys walk it, stepping over the
      disabled entries; `Ctrl+N` opens New Document with Create already focused; the
      single-letter tool shortcuts switch tools (`m`, `b`, `h`, each confirmed in the
      status bar); `Ctrl+Shift+P` opens the command palette with its filter focused, and
      typing `invert` then Return ran the filter — Properties gained its Effects group;
      `Ctrl+Z` took it away again; `Ctrl+W` raised the close prompt and Escape cancelled
      it with the document and its selection intact. The tool rail itself is not in the
      tab chain, which matches Photoshop: tools are reached by their letters
- [!] **U-20** Focus is always visible and its order follows the layout — **order passes,
      visibility failed**. Tab order follows the layout: the toolbar left to right,
      skipping the disabled buttons, then into the right dock's panel headers, and within
      the Properties panel a disclosure header hands off to the control below it. But
      focus was invisible on every icon-only button: AT-SPI reported it moving from Redo
      to About PhotoTux while a pixel diff of the whole window found nothing had changed.
      Eleven hand-rolled `ToolButton` backgrounds drew hover and checked and not focus.
      Fixed — all eleven plus `ChromeIconToolButton` now paint `Theme.focusRing` on
      `visualFocus`; verified live on the toolbar's New button and on the Properties
      header's Collapse all groups
- [x] **U-21** Live regions announce without flooding — mask sliders report their values ("Mask density, 100 percent", "Mask feather, 0.0 pixels"); T-009's status-bar flood remains fixed
- [x] **U-22** No meaning conveyed by colour alone — every state that carries an accent
      also carries a shape or a word. Notices pair the accent with a per-level glyph
      (`info`, `warning`, `x-circle`) from `NoticeLevel::icon_key`; the active tool takes
      a filled slot and a left bar, not just a tint; the dirty marker is a `*` in the
      title and the tab plus the word Unsaved in the status bar; disabled controls dim
      *and* stop responding; the selection is a dashed outline; the GPU badge spells out
      GPU ACCELERATED. The one marginal case is the menu highlight, which is a fill one
      step lighter than the menu — legible, but the least of these

## 3.5 Web surfaces

- [x] **U-23** Landing page loads at the top and both themes render — opens at `scrollY 0` (the earlier `scrollIntoView` fix holds); the toggle switches root theme and repaints the body
- [x] **U-24** Docs site navigation, search and table of contents work — nav, search and TOC all present; `/search-index.json` serves 17 entries and "layer mask" returns two relevant pages
- [x] **U-25** Both sites are usable from 320 px to 1440 px+ — neither page scrolls horizontally at 375 px. On the landing page the architecture table and gallery tabs scroll *inside* their own `overflow-x: auto` containers, which is the rule; nothing escapes containment on the docs site
- [x] **U-26** The public shortcut reference matches the shipped registry — all 56 chords agree in both directions, and `the_published_reference_lists_the_chords_that_ship` now keeps them that way
- [x] **U-27** No broken internal links or missing assets on either site — `check-docs-links.py` resolves 70 pages, routes and assets included

---

## Pass log

| Date | Scope | Result |
|---|---|---|
| 2026-09-03 | §1.6, §2.1–2.4, parts of §1.1/1.3/1.5 and §3 | 35 pass · 3 partial · 1 fail · 78 not yet run |
