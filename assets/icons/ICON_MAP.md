# PhotoTux Icon Map (Phosphor)

Canonical mapping of **UI tools & actions** → **Phosphor SVG** files vendored under `assets/icons/phosphor/`.

| Field | Convention |
|-------|------------|
| **Pack** | Phosphor Icons (`@phosphor-icons/core` 2.1.1) |
| **Path pattern** | `assets/icons/phosphor/{weight}/{name}.svg` |
| **Default weight** | `regular` (dense chrome) |
| **Active / selected tool** | Prefer `fill` of the same base name when available, else `regular` + selection chrome from `DESIGN.md` |
| **Small 16–18px glyphs** | Consider `bold` if regular reads thin |
| **Tint** | SVGs use `currentColor` — colorize in QML; do not hardcode strokes in assets |
| **Naming in code** | Prefer stable **Action IDs** (left column); resolve to SVG path via this map |

**Related:** pack layout & license → [README.md](./README.md) · IA tools → `docs/INFORMATION_ARCHITECTURE.md` · ADR-013 G15.

---

## 1. Left tool strip (exclusive modes)

Primary creative tools. One active at a time (radio group). Order ≈ recommended strip top→bottom for Phase 4+; earlier phases may show a subset.

| Action ID | UI label | Icon file (`regular/`) | Alt / notes |
|-----------|----------|------------------------|-------------|
| `tool.brush` | Brush | `paint-brush.svg` | Primary paint. **Active:** `fill/paint-brush-fill.svg` if present |
| `tool.pencil` | Pencil | `pencil-simple.svg` | Hard-edge draw. Alt: `pencil.svg` (more ornate) |
| `tool.eraser` | Eraser | `eraser.svg` | |
| `tool.fill` | Fill / bucket | `paint-bucket.svg` | Contiguous / flood fill |
| `tool.eyedropper` | Eyedropper | `eyedropper.svg` | Sample color. Alt: `eyedropper-sample.svg` |
| `tool.select_rect` | Rectangular select | `selection.svg` | Marquee. Alt: `rectangle-dashed.svg` |
| `tool.select_ellipse` | Elliptical select | `circle-dashed.svg` | |
| `tool.select_lasso` | Lasso | `lasso.svg` | Freehand selection |
| `tool.select_polygon` | Polygonal select | `polygon.svg` | |
| `tool.magic_wand` | Magic wand | `magic-wand.svg` | Region select by color |
| `tool.move` | Move | `arrows-out-cardinal.svg` | Layer/selection move |
| `tool.transform` | Transform | `bounding-box.svg` | Free transform box. Alt: `frame-corners.svg` |
| `tool.crop` | Crop | `crop.svg` | |
| `tool.pan` | Pan | `hand.svg` | Canvas pan (also Space+drag). Grabbing: `hand-grabbing.svg` while dragging |
| `tool.zoom` | Zoom | `magnifying-glass.svg` | Click zoom tool. Also see View zoom actions |
| `tool.text` | Text | `text-t.svg` | Phase 4+ |
| `tool.shape` | Shape | `shapes.svg` | Phase 4+. Specifics: `rectangle.svg`, `circle.svg`, `triangle.svg`, `line-segment.svg` |
| `tool.pen` | Pen / path | `pen-nib.svg` | Paths later. Alt: `path.svg`, `pen.svg` |
| `tool.clone` | Clone stamp | `stamp.svg` | Phase 4+ |
| `tool.gradient` | Gradient | `gradient.svg` | Phase 4+ |

### Tool strip chrome

| Action ID | UI label | Icon | Notes |
|-----------|----------|------|-------|
| `tools.overflow` | More tools | `dots-three.svg` | Overflow menu when strip exceeds max |
| `tools.presets` | Brush presets | `swatches.svg` | Drawer Phase 4+ |

---

## 2. Document lifecycle (toolbar / File menu)

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `doc.new` | New… | `file-plus.svg` | Opens New Document dialog (presets) |
| `doc.open` | Open… | `folder-open.svg` | Portal / file open |
| `doc.save` | Save | `floppy-disk.svg` | |
| `doc.save_as` | Save As… | `note-pencil.svg` | Distinct from Save |
| `doc.export` | Export… | `export.svg` | |
| `doc.close` | Close | `x.svg` | Or window close only |
| `doc.import` | Import / place | `download.svg` | Place image into doc |
| `doc.image` | Document / raster | `image.svg` | Generic document type |

---

## 3. Edit menu & history

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `edit.undo` | Undo | `arrow-counter-clockwise.svg` | One gesture = one step (ADR-013) |
| `edit.redo` | Redo | `arrow-clockwise.svg` | |
| `edit.cut` | Cut | `scissors.svg` | |
| `edit.copy` | Copy | `copy.svg` | Alt: `copy-simple.svg` |
| `edit.paste` | Paste | `clipboard.svg` | Alt: `clipboard-text.svg` |
| `edit.delete` | Delete | `trash.svg` | Alt: `trash-simple.svg` |
| `edit.select_all` | Select all | `selection-all.svg` | |
| `edit.deselect` | Deselect | `selection-slash.svg` | |
| `edit.invert_selection` | Invert selection | `selection-inverse.svg` | |
| `edit.duplicate` | Duplicate | `copy-simple.svg` | Layer/selection duplicate |

---

## 4. View, zoom & workspace

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `view.zoom_in` | Zoom in | `magnifying-glass-plus.svg` | |
| `view.zoom_out` | Zoom out | `magnifying-glass-minus.svg` | |
| `view.zoom_fit` | Zoom to fit | `corners-in.svg` | Default on open/new (ADR-013 G18) |
| `view.zoom_100` | Actual size (100%) | `frame-corners.svg` | Or `corners-out.svg` for “expand” |
| `view.grid` | Show grid | `grid-four.svg` | Alt: `grid-nine.svg` denser |
| `view.rulers` | Show rulers | `ruler.svg` | |
| `view.fullscreen` | Full screen | `corners-out.svg` | |
| `view.reset` | Reset view | `arrows-counter-clockwise.svg` | Pan/zoom reset |
| `workspace.toggle_left` | Toggle tool strip | `sidebar-simple.svg` | Optional |
| `workspace.toggle_right` | Toggle docks | `sidebar.svg` | Mirrored in UI |
| `workspace.collapse_dock` | Collapse panel | `caret-right.svg` / `caret-left.svg` | Directional |

---

## 5. Layers panel

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `layer.panel` | Layers (panel title) | `stack.svg` | Not `layers` — Phosphor uses `stack` |
| `layer.add` | New layer | `stack-plus.svg` | Alt: `plus.svg` in row actions |
| `layer.delete` | Delete layer | `trash.svg` | |
| `layer.duplicate` | Duplicate layer | `copy-simple.svg` | |
| `layer.merge` | Merge down | `stack-simple.svg` | Semantic “flatten stack” |
| `layer.group` | Group | `folders.svg` | Phase 3+ groups |
| `layer.visible` | Visible | `eye.svg` | Toggle on |
| `layer.hidden` | Hidden | `eye-slash.svg` | Toggle off. Alt: `eye-closed.svg` |
| `layer.locked` | Locked | `lock.svg` | Alt: `lock-simple.svg` |
| `layer.unlocked` | Unlocked | `lock-open.svg` | |
| `layer.move_up` | Move up | `caret-up.svg` | Stack order |
| `layer.move_down` | Move down | `caret-down.svg` | |
| `layer.opacity` | Opacity | `circle-half.svg` | Or control without icon |
| `layer.blend` | Blend mode | `square-half.svg` | Closest “half/half” metaphor; no dedicated blend icon |
| `layer.mask` | Layer mask | `mask-happy.svg` | Better than none; or `circle-dashed.svg` for mask outline |

---

## 6. Properties / inspector

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `props.panel` | Properties | `sliders-horizontal.svg` | Panel title |
| `props.color` | Color | `palette.svg` | Alt: `drop.svg` for single swatch |
| `props.swatches` | Swatches | `swatches.svg` | |
| `props.size` | Size / diameter | `circle.svg` | Brush size metaphor |
| `props.hardness` | Hardness | `circle-half.svg` | Soft↔hard |
| `props.advanced` | Advanced section | `caret-down.svg` | Collapsible |
| `props.reset` | Reset parameter | `arrow-counter-clockwise.svg` | Local reset |

---

## 7. Transform & geometry (context toolbar)

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `xform.rotate_cw` | Rotate 90° CW | `arrow-clockwise.svg` | No dedicated rotate glyph |
| `xform.rotate_ccw` | Rotate 90° CCW | `arrow-counter-clockwise.svg` | |
| `xform.flip_h` | Flip horizontal | `flip-horizontal.svg` | |
| `xform.flip_v` | Flip vertical | `flip-vertical.svg` | |
| `xform.scale` | Scale | `arrows-out.svg` | |
| `xform.shrink` | Shrink bounds | `arrows-in.svg` | |
| `xform.apply` | Apply transform | `check.svg` | |
| `xform.cancel` | Cancel transform | `x.svg` | |

---

## 8. Selection operations

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `sel.add` | Add to selection | `selection-plus.svg` | |
| `sel.subtract` | Subtract from selection | `selection-slash.svg` | Or mode badge |
| `sel.intersect` | Intersect | `intersect.svg` | Boolean |
| `sel.feather` | Feather | `drop-half.svg` | Soft edge metaphor |
| `sel.grow` | Expand | `arrows-out.svg` | |
| `sel.shrink` | Contract | `arrows-in.svg` | |

---

## 9. Application chrome & dialogs

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `app.menu` | Application menu | `list.svg` | If hamburger needed |
| `app.settings` | Preferences | `gear.svg` | Alt: `gear-six.svg` |
| `app.about` | About | `info.svg` | |
| `app.help` | Help / shortcuts | `question.svg` | Keyboard shortcuts overlay |
| `app.warning` | Warning | `warning.svg` | Confirm destructive |
| `app.error` | Error | `warning-circle.svg` | |
| `app.success` | Success | `check-circle.svg` | Toast |
| `app.close_dialog` | Close dialog | `x.svg` | |
| `app.confirm` | OK / Confirm | `check.svg` | |
| `app.cancel` | Cancel | `x-circle.svg` | Soft cancel vs hard X |
| `app.overflow` | More | `dots-three-vertical.svg` | Vertical menus |
| `app.pin` | Pin panel | `push-pin.svg` | Floating later |
| `app.unpin` | Unpin | `push-pin-slash.svg` | |

---

## 10. Status bar / HUD (dev & user)

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `status.zoom` | Zoom level | `magnifying-glass.svg` | Beside “100%” text |
| `status.tool` | Active tool | *(same as active tool icon)* | Reflect tool strip |
| `status.gpu` | GPU / Vulkan | `cpu.svg` | Alt: `monitor.svg` / `desktop.svg` |
| `status.perf` | FPS / HUD | `gauge.svg` | Alt: `pulse.svg` |
| `status.unsaved` | Unsaved | `circle.svg` (small / fill weight) | Or badge on `floppy-disk.svg` |

---

## 11. Boolean / path ops (later vector-adjacent)

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `bool.unite` | Unite | `unite.svg` | |
| `bool.subtract` | Subtract | `subtract.svg` | |
| `bool.intersect` | Intersect | `intersect.svg` | |
| `bool.exclude` | Exclude | `exclude.svg` | |

---

## 12. Phase → which icons to wire first

| Phase | Wire these Action ID groups |
|-------|----------------------------|
| **1** | `doc.*` (new/open placeholders), `app.*` chrome, `props.panel`, `layer.panel` + eye/lock stubs, `view.zoom_*`, `edit.undo/redo` (no-op ok), `tool.brush` + `tool.pan` + `tool.zoom` stubs |
| **2** | Full `view.*` + pan/zoom tools; `status.zoom` / `status.perf` |
| **3** | Full `layer.*`, `edit.*` history real |
| **4** | Full tool strip §1, selection §8, transform §7 |
| **5** | Portals still use system UI; keep `doc.open` / `doc.export` icons in menus |

---

## 13. QML resolution helper (contract)

Suggested API when implementing (not code yet):

```
iconSource(actionId, weight = "regular", filled = false)
  → "qrc:/…/phosphor/{weight}/{stem}.svg"
```

| Input | Stem resolution |
|-------|-----------------|
| `tool.brush`, filled false | `paint-brush` |
| `tool.brush`, filled true | `paint-brush` under `fill/` if file exists, else regular + UI selection ring |
| Missing file | Log once; fall back to `question.svg` |

Keep **Action ID → stem** as a single table in code generated from or kept in sync with **this document**.

---

## 14. Disambiguation / rejected alternatives

| Need | Rejected | Why we picked primary |
|------|----------|------------------------|
| Layers | — (no `layers.svg`) | `stack.svg` is Phosphor’s stack/layers metaphor |
| Brush | `paint-brush-household` | Too “cleaning”; `paint-brush` is artistic |
| Pencil | `pencil` | Busier; `pencil-simple` denser UI |
| Pan | `hand-palm` | `hand` clearer “grab canvas” |
| Transform | no `transform.svg` | `bounding-box` matches editor free-transform affordance |
| Blend mode | no blend icon | `square-half` = dual regions; document as weak metaphor |
| Zoom to fit | `corners-in` | Reads as “fit content in frame” vs `corners-out` expand |

---

## 15. Verification checklist (before wiring)

Confirm files exist (from repo root):

```bash
ICON_ROOT=assets/icons/phosphor/regular
for s in paint-brush pencil-simple eraser paint-bucket eyedropper selection \
  circle-dashed lasso polygon magic-wand arrows-out-cardinal bounding-box crop \
  hand magnifying-glass text-t shapes pen-nib stamp gradient stack stack-plus \
  eye eye-slash lock lock-open file-plus folder-open floppy-disk export \
  arrow-counter-clockwise arrow-clockwise scissors copy clipboard trash \
  magnifying-glass-plus magnifying-glass-minus corners-in frame-corners \
  sliders-horizontal palette gear info question; do
  test -f "$ICON_ROOT/$s.svg" && echo "OK $s" || echo "MISSING $s"
done
```

---

*Last updated: 2026-07-15 — Phosphor core 2.1.1 mapping for PhotoTux desktop GUI.*
