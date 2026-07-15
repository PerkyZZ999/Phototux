# PhotoTux Icon Map (Phosphor)

Canonical mapping of **UI tools & actions** → **Phosphor SVG** files vendored under `assets/icons/phosphor/`.

| Field | Convention |
|-------|------------|
| **Pack** | Phosphor Icons (`@phosphor-icons/core` 2.1.1) |
| **Path pattern** | `assets/icons/phosphor/{weight}/{name}.svg` |
| **Default weight** | `regular` (dense chrome) |
| **Active / selected tool** | Prefer `fill/{name}-fill.svg` when available, else `regular` + selection chrome from `DESIGN.md` |
| **Small 16–18px glyphs** | Consider `bold` if regular reads thin |
| **Tint** | SVGs use `currentColor` — colorize in QML |
| **Naming in code** | Stable **Action IDs** → resolve via this map only |

**Related:** [README.md](./README.md) · IA · ADR-013 G15.

---

## Uniqueness policy

1. **Primary icon** = the SVG in the “Icon file” column (not Alts).
2. **Each primary SVG is used for at most one Action ID**, except entries on the **Shared allowlist**.
3. **Alts** are fallbacks only; they must not be another action’s primary.
4. **Dynamic** icons (`status.tool` = active tool) mirror another action’s primary on purpose (not a second fixed mapping).
5. **Control state** (e.g. pan grab, dock caret flip) may swap a secondary glyph without a second Action ID primary.

### Shared allowlist (intentional multi-use)

| Primary SVG | Action IDs | Why sharing is OK |
|-------------|------------|-------------------|
| `trash.svg` | `edit.delete`, `layer.delete` | Same “delete” affordance |
| `copy-simple.svg` | `edit.duplicate`, `layer.duplicate` | Same “duplicate” affordance |
| `check.svg` | `xform.apply`, `app.confirm` | Affirmative commit |
| `x.svg` | `doc.close`, `app.close_dialog` | Close / dismiss surface |
| `x-circle.svg` | `app.cancel`, `xform.cancel` | Generic cancel |
| `magnifying-glass.svg` | `tool.zoom`, `status.zoom` | Same zoom concept (tool vs readout) |
| `intersect.svg` | `sel.intersect`, `bool.intersect` | Same boolean op |

---

## 1. Left tool strip (exclusive modes)

One active at a time. Order ≈ recommended strip top→bottom (Phase 4+; earlier phases use a subset).

| Action ID | UI label | Icon file (`regular/`) | Alt / notes |
|-----------|----------|------------------------|-------------|
| `tool.brush` | Brush | `paint-brush.svg` | Active: `fill/paint-brush-fill.svg` if present |
| `tool.pencil` | Pencil | `pencil-simple.svg` | Alt: `pencil.svg` |
| `tool.eraser` | Eraser | `eraser.svg` | |
| `tool.fill` | Fill / bucket | `paint-bucket.svg` | |
| `tool.eyedropper` | Eyedropper | `eyedropper.svg` | Alt: `eyedropper-sample.svg` |
| `tool.select_rect` | Rectangular select | `selection.svg` | Alt: `rectangle-dashed.svg` (alt only) |
| `tool.select_ellipse` | Elliptical select | `circle-dashed.svg` | |
| `tool.select_lasso` | Lasso | `lasso.svg` | |
| `tool.select_polygon` | Polygonal select | `polygon.svg` | |
| `tool.magic_wand` | Magic wand | `magic-wand.svg` | |
| `tool.move` | Move | `arrows-out-cardinal.svg` | Layer/selection move |
| `tool.transform` | Transform | `bounding-box.svg` | Free-transform box |
| `tool.crop` | Crop | `crop.svg` | |
| `tool.pan` | Pan | `hand.svg` | Drag **state** may show `hand-grabbing.svg` (not a second Action ID) |
| `tool.zoom` | Zoom | `magnifying-glass.svg` | Allowlist: also `status.zoom` |
| `tool.text` | Text | `text-t.svg` | Phase 4+ |
| `tool.shape` | Shape | `shapes.svg` | Subtools: `rectangle`, `circle`, `triangle`, `line-segment` (not separate strip primaries unless promoted later) |
| `tool.pen` | Pen / path | `pen-nib.svg` | Alt: `path.svg` |
| `tool.clone` | Clone stamp | `stamp.svg` | Phase 4+ |
| `tool.gradient` | Gradient | `gradient.svg` | Phase 4+ |

### Tool strip chrome

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `tools.overflow` | More tools | `dots-three.svg` | Horizontal overflow |
| `tools.presets` | Brush presets | `swatches.svg` | Distinct from `props.swatches` |

---

## 2. Document lifecycle (toolbar / File menu)

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `doc.new` | New… | `file-plus.svg` | New Document dialog + presets |
| `doc.open` | Open… | `folder-open.svg` | |
| `doc.save` | Save | `floppy-disk.svg` | |
| `doc.save_as` | Save As… | `note-pencil.svg` | Distinct from Save |
| `doc.export` | Export… | `export.svg` | |
| `doc.close` | Close | `x.svg` | Allowlist: `app.close_dialog` |
| `doc.import` | Import / place | `download.svg` | |
| `doc.image` | Document / raster | `image.svg` | Type indicator |

---

## 3. Edit menu & history

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `edit.undo` | Undo | `arrow-counter-clockwise.svg` | **Unique** — not rotate/reset |
| `edit.redo` | Redo | `arrow-clockwise.svg` | **Unique** — not rotate |
| `edit.cut` | Cut | `scissors.svg` | |
| `edit.copy` | Copy | `copy.svg` | Distinct from duplicate |
| `edit.paste` | Paste | `clipboard.svg` | |
| `edit.delete` | Delete | `trash.svg` | Allowlist: `layer.delete` |
| `edit.select_all` | Select all | `selection-all.svg` | |
| `edit.deselect` | Deselect | `selection-slash.svg` | **Not** selection subtract mode |
| `edit.invert_selection` | Invert selection | `selection-inverse.svg` | |
| `edit.duplicate` | Duplicate | `copy-simple.svg` | Allowlist: `layer.duplicate` |

---

## 4. View, zoom & workspace

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `view.zoom_in` | Zoom in | `magnifying-glass-plus.svg` | |
| `view.zoom_out` | Zoom out | `magnifying-glass-minus.svg` | |
| `view.zoom_fit` | Zoom to fit | `corners-in.svg` | ADR-013 G18 |
| `view.zoom_100` | Actual size (100%) | `frame-corners.svg` | Distinct from fit / fullscreen |
| `view.grid` | Show grid | `grid-four.svg` | Alt only: `grid-nine.svg` |
| `view.rulers` | Show rulers | `ruler.svg` | |
| `view.fullscreen` | Full screen | `corners-out.svg` | Distinct from fit / 100% |
| `view.reset` | Reset view | `arrows-counter-clockwise.svg` | Plural arrows — **not** undo |
| `workspace.toggle_left` | Toggle tool strip | `sidebar-simple.svg` | |
| `workspace.toggle_right` | Toggle docks | `sidebar.svg` | |
| `workspace.collapse_dock` | Collapse panel | `caret-left.svg` | Expanded state may flip to `caret-right` (state, not second ID) |

---

## 5. Layers panel

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `layer.panel` | Layers | `stack.svg` | No `layers.svg` in Phosphor |
| `layer.add` | New layer | `stack-plus.svg` | |
| `layer.delete` | Delete layer | `trash.svg` | Allowlist: `edit.delete` |
| `layer.duplicate` | Duplicate layer | `copy-simple.svg` | Allowlist: `edit.duplicate` |
| `layer.merge` | Merge down | `stack-simple.svg` | |
| `layer.group` | Group | `folders.svg` | Phase 3+ |
| `layer.visible` | Visible | `eye.svg` | Toggle with hidden |
| `layer.hidden` | Hidden | `eye-slash.svg` | |
| `layer.locked` | Locked | `lock.svg` | Toggle with unlocked |
| `layer.unlocked` | Unlocked | `lock-open.svg` | |
| `layer.move_up` | Move up in stack | `caret-double-up.svg` | Distinct from section carets |
| `layer.move_down` | Move down in stack | `caret-double-down.svg` | |
| `layer.opacity` | Opacity | `drop-half.svg` | Distinct from hardness |
| `layer.blend` | Blend mode | `square-half.svg` | Weak metaphor; unique stem |
| `layer.mask` | Layer mask | `rectangle-dashed.svg` | Distinct from ellipse select (`circle-dashed`) |

---

## 6. Properties / inspector

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `props.panel` | Properties | `sliders-horizontal.svg` | |
| `props.color` | Color | `palette.svg` | |
| `props.swatches` | Swatches | `circles-three.svg` | Distinct from `tools.presets` (`swatches`) |
| `props.size` | Size / diameter | `circle.svg` | Distinct from unsaved indicator |
| `props.hardness` | Hardness | `circle-half.svg` | Distinct from opacity (`drop-half`) |
| `props.advanced` | Advanced section | `caret-down.svg` | Expand/collapse only |
| `props.reset` | Reset parameter | `arrow-u-up-left.svg` | “Return” — **not** undo |

---

## 7. Transform & geometry (context toolbar)

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `xform.rotate_cw` | Rotate 90° CW | `arrow-arc-right.svg` | **Not** redo |
| `xform.rotate_ccw` | Rotate 90° CCW | `arrow-arc-left.svg` | **Not** undo |
| `xform.flip_h` | Flip horizontal | `flip-horizontal.svg` | |
| `xform.flip_v` | Flip vertical | `flip-vertical.svg` | |
| `xform.scale` | Scale up / grow box | `arrows-out.svg` | Distinct from sel.grow |
| `xform.shrink` | Scale down / shrink box | `arrows-in.svg` | Distinct from sel.shrink |
| `xform.apply` | Apply transform | `check.svg` | Allowlist: `app.confirm` |
| `xform.cancel` | Cancel transform | `x-circle.svg` | Allowlist: `app.cancel` |

---

## 8. Selection operations

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `sel.add` | Add to selection | `selection-plus.svg` | |
| `sel.subtract` | Subtract from selection | `minus-circle.svg` | **Not** deselect (`selection-slash`) |
| `sel.intersect` | Intersect selection | `intersect.svg` | Allowlist: `bool.intersect` |
| `sel.feather` | Feather | `drop-half-bottom.svg` | Distinct from opacity / hardness |
| `sel.grow` | Expand selection | `plus-circle.svg` | **Not** transform scale |
| `sel.shrink` | Contract selection | `minus.svg` | Distinct from `minus-circle` (subtract mode) |

---

## 9. Application chrome & dialogs

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `app.menu` | Application menu | `list.svg` | |
| `app.settings` | Preferences | `gear.svg` | |
| `app.about` | About | `info.svg` | |
| `app.help` | Help / shortcuts | `question.svg` | |
| `app.warning` | Warning | `warning.svg` | |
| `app.error` | Error | `warning-circle.svg` | |
| `app.success` | Success | `check-circle.svg` | |
| `app.close_dialog` | Close dialog | `x.svg` | Allowlist: `doc.close` |
| `app.confirm` | OK / Confirm | `check.svg` | Allowlist: `xform.apply` |
| `app.cancel` | Cancel | `x-circle.svg` | Allowlist: `xform.cancel` |
| `app.overflow` | More | `dots-three-vertical.svg` | Distinct from tools overflow |
| `app.pin` | Pin panel | `push-pin.svg` | |
| `app.unpin` | Unpin | `push-pin-slash.svg` | |

---

## 10. Status bar / HUD

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `status.zoom` | Zoom level | `magnifying-glass.svg` | Allowlist: `tool.zoom` |
| `status.tool` | Active tool | *(dynamic = active tool primary)* | Not a fixed stem |
| `status.gpu` | GPU / Vulkan | `cpu.svg` | |
| `status.perf` | FPS / HUD | `gauge.svg` | |
| `status.unsaved` | Unsaved | `circle-notch.svg` | Distinct from `props.size` (`circle`) |

---

## 11. Boolean / path ops (later)

| Action ID | UI label | Icon file (`regular/`) | Notes |
|-----------|----------|------------------------|-------|
| `bool.unite` | Unite | `unite.svg` | |
| `bool.subtract` | Subtract | `subtract.svg` | Distinct from `sel.subtract` |
| `bool.intersect` | Intersect | `intersect.svg` | Allowlist: `sel.intersect` |
| `bool.exclude` | Exclude | `exclude.svg` | |

---

## Reverse index (primary stem → Action ID)

| Stem | Action ID(s) |
|------|----------------|
| `paint-brush` | `tool.brush` |
| `pencil-simple` | `tool.pencil` |
| `eraser` | `tool.eraser` |
| `paint-bucket` | `tool.fill` |
| `eyedropper` | `tool.eyedropper` |
| `selection` | `tool.select_rect` |
| `circle-dashed` | `tool.select_ellipse` |
| `lasso` | `tool.select_lasso` |
| `polygon` | `tool.select_polygon` |
| `magic-wand` | `tool.magic_wand` |
| `arrows-out-cardinal` | `tool.move` |
| `bounding-box` | `tool.transform` |
| `crop` | `tool.crop` |
| `hand` | `tool.pan` |
| **`magnifying-glass`** | **`tool.zoom`**, **`status.zoom`** |
| `text-t` | `tool.text` |
| `shapes` | `tool.shape` |
| `pen-nib` | `tool.pen` |
| `stamp` | `tool.clone` |
| `gradient` | `tool.gradient` |
| `dots-three` | `tools.overflow` |
| `swatches` | `tools.presets` |
| `file-plus` | `doc.new` |
| `folder-open` | `doc.open` |
| `floppy-disk` | `doc.save` |
| `note-pencil` | `doc.save_as` |
| `export` | `doc.export` |
| **`x`** | **`doc.close`**, **`app.close_dialog`** |
| `download` | `doc.import` |
| `image` | `doc.image` |
| `arrow-counter-clockwise` | `edit.undo` |
| `arrow-clockwise` | `edit.redo` |
| `scissors` | `edit.cut` |
| `copy` | `edit.copy` |
| `clipboard` | `edit.paste` |
| **`trash`** | **`edit.delete`**, **`layer.delete`** |
| `selection-all` | `edit.select_all` |
| `selection-slash` | `edit.deselect` |
| `selection-inverse` | `edit.invert_selection` |
| **`copy-simple`** | **`edit.duplicate`**, **`layer.duplicate`** |
| `magnifying-glass-plus` | `view.zoom_in` |
| `magnifying-glass-minus` | `view.zoom_out` |
| `corners-in` | `view.zoom_fit` |
| `frame-corners` | `view.zoom_100` |
| `grid-four` | `view.grid` |
| `ruler` | `view.rulers` |
| `corners-out` | `view.fullscreen` |
| `arrows-counter-clockwise` | `view.reset` |
| `sidebar-simple` | `workspace.toggle_left` |
| `sidebar` | `workspace.toggle_right` |
| `caret-left` | `workspace.collapse_dock` |
| `stack` | `layer.panel` |
| `stack-plus` | `layer.add` |
| `stack-simple` | `layer.merge` |
| `folders` | `layer.group` |
| `eye` | `layer.visible` |
| `eye-slash` | `layer.hidden` |
| `lock` | `layer.locked` |
| `lock-open` | `layer.unlocked` |
| `caret-double-up` | `layer.move_up` |
| `caret-double-down` | `layer.move_down` |
| `drop-half` | `layer.opacity` |
| `square-half` | `layer.blend` |
| `rectangle-dashed` | `layer.mask` |
| `sliders-horizontal` | `props.panel` |
| `palette` | `props.color` |
| `circles-three` | `props.swatches` |
| `circle` | `props.size` |
| `circle-half` | `props.hardness` |
| `caret-down` | `props.advanced` |
| `arrow-u-up-left` | `props.reset` |
| `arrow-arc-right` | `xform.rotate_cw` |
| `arrow-arc-left` | `xform.rotate_ccw` |
| `flip-horizontal` | `xform.flip_h` |
| `flip-vertical` | `xform.flip_v` |
| `arrows-out` | `xform.scale` |
| `arrows-in` | `xform.shrink` |
| **`check`** | **`xform.apply`**, **`app.confirm`** |
| **`x-circle`** | **`xform.cancel`**, **`app.cancel`** |
| `selection-plus` | `sel.add` |
| `minus-circle` | `sel.subtract` |
| **`intersect`** | **`sel.intersect`**, **`bool.intersect`** |
| `drop-half-bottom` | `sel.feather` |
| `plus-circle` | `sel.grow` |
| `minus` | `sel.shrink` |
| `list` | `app.menu` |
| `gear` | `app.settings` |
| `info` | `app.about` |
| `question` | `app.help` |
| `warning` | `app.warning` |
| `warning-circle` | `app.error` |
| `check-circle` | `app.success` |
| `dots-three-vertical` | `app.overflow` |
| `push-pin` | `app.pin` |
| `push-pin-slash` | `app.unpin` |
| `cpu` | `status.gpu` |
| `gauge` | `status.perf` |
| `circle-notch` | `status.unsaved` |
| `unite` | `bool.unite` |
| `subtract` | `bool.subtract` |
| `exclude` | `bool.exclude` |

---

## Phase → which icons to wire first

| Phase | Action ID groups |
|-------|------------------|
| **1** | `doc.*`, `app.*`, `props.panel`, `layer.panel` + eye/lock, `view.zoom_*`, `edit.undo/redo`, `tool.brush` / `tool.pan` / `tool.zoom` stubs |
| **2** | Full `view.*`, pan/zoom tools, `status.*` |
| **3** | Full `layer.*`, live `edit.*` |
| **4** | Full tool strip, transform §7, selection §8 |
| **5** | Menu icons for portals (`doc.open` / `doc.export`) |

---

## QML resolution contract

```
iconSource(actionId, weight = "regular", filled = false)
  → assets/icons/phosphor/{weight}/{stem}.svg
```

| Rule | Behavior |
|------|----------|
| Unknown actionId | Fallback `question.svg`; log once |
| `filled` and file missing | regular + selection chrome |
| `status.tool` | Resolve active tool’s Action ID, then this map |

---

## Conflicts fixed (uniqueness pass)

| Collision | Was | Now |
|-----------|-----|-----|
| Undo / rotate CCW / props reset | all `arrow-counter-clockwise` | undo keeps; rotate → `arrow-arc-left`; reset → `arrow-u-up-left` |
| Redo / rotate CW | both `arrow-clockwise` | redo keeps; rotate → `arrow-arc-right` |
| Deselect / sel.subtract | both `selection-slash` | deselect keeps; subtract → `minus-circle` |
| Opacity / hardness | both `circle-half` | hardness keeps; opacity → `drop-half` |
| Scale / sel.grow | both `arrows-out` | scale keeps; grow → `plus-circle` |
| Shrink box / sel.shrink | both `arrows-in` | shrink keeps; contract → `minus` |
| Layer reorder / advanced | `caret-down` clash | advanced keeps; reorder → `caret-double-*` |
| Presets / props swatches | both `swatches` | presets keep; props → `circles-three` |
| Ellipse / layer mask | dashed-circle clash | ellipse → `circle-dashed`; mask → `rectangle-dashed` |
| Unsaved / brush size | both `circle` | size keeps; unsaved → `circle-notch` |
| Fit / 100% / fullscreen | corner ambiguity | fit `corners-in`, 100% `frame-corners`, FS `corners-out` |

---

## Verification

### Stems exist

```bash
ICON_ROOT=assets/icons/phosphor/regular
for s in paint-brush pencil-simple eraser paint-bucket eyedropper selection \
  circle-dashed lasso polygon magic-wand arrows-out-cardinal bounding-box crop \
  hand magnifying-glass text-t shapes pen-nib stamp gradient dots-three swatches \
  file-plus folder-open floppy-disk note-pencil export x download image \
  arrow-counter-clockwise arrow-clockwise scissors copy clipboard trash \
  selection-all selection-slash selection-inverse copy-simple \
  magnifying-glass-plus magnifying-glass-minus corners-in frame-corners \
  grid-four ruler corners-out arrows-counter-clockwise sidebar-simple sidebar \
  caret-left stack stack-plus stack-simple folders eye eye-slash lock lock-open \
  caret-double-up caret-double-down drop-half square-half rectangle-dashed \
  sliders-horizontal palette circles-three circle circle-half caret-down \
  arrow-u-up-left arrow-arc-right arrow-arc-left flip-horizontal flip-vertical \
  arrows-out arrows-in check x-circle selection-plus minus-circle intersect \
  drop-half-bottom plus-circle minus list gear info question warning \
  warning-circle check-circle dots-three-vertical push-pin push-pin-slash \
  cpu gauge circle-notch unite subtract exclude; do
  test -f "$ICON_ROOT/$s.svg" && echo "OK $s" || echo "MISSING $s"
done
```

### Uniqueness (outside allowlist)

Every reverse-index stem maps to exactly one Action ID, except the seven allowlist multi-use rows (bold in reverse index).

---

*Last updated: 2026-07-15 — uniqueness pass complete.*
