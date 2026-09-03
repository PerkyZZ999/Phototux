# PhotoTux Icon Map (Phosphor)

Canonical mapping of **shipped tools & actions** → **Phosphor SVG stems** under `assets/icons/phosphor/`.

| Field | Convention |
|-------|------------|
| **Pack** | Phosphor Icons (`@phosphor-icons/core` 2.1.1) — [DR-023](../../internal_docs/Appendix/Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase) |
| **Path pattern** | `assets/icons/phosphor/{weight}/{stem}.svg` |
| **Default weight** | `regular` (dense chrome) |
| **Active / selected tool** | Prefer `fill/{stem}-fill.svg` when present, else `regular` + Theme selection chrome ([25 — Themes](../../internal_docs/25-Themes.md)) |
| **Small 16–18px glyphs** | Consider `bold` if regular reads thin |
| **Tint** | SVGs use `currentColor` — colorize in QML via Theme |
| **Authority for IDs** | `phototux_engine` `tool_id::*`, `ToolDescriptor`, `ActionDescriptor` — not this file alone |
| **Commands** | [Command-Taxonomy](../../internal_docs/Appendix/Command-Taxonomy.md) |

**Related:** [README.md](./README.md) · [06 — Toolbar](../../internal_docs/06-Toolbar-System.md) · [AGENTS.md](../../AGENTS.md) iconography skill.

---

## ID spaces (do not collapse)

| Kind | Example | Where `icon_key` lives | Resolves to |
|------|---------|------------------------|-------------|
| **Tool** | `tool.brush` | `ToolDescriptor.icon_key` (Phosphor stem) | Tool strip / overflow |
| **Action** | `action.file.new` | `ActionDescriptor.icon_key` (Phosphor stem) | Menus, toolbar, palette |
| **Chrome-only** | `tools.overflow` | QML (`dots-three`) | Not always in the action table |

Stems are **Phosphor file basenames** without `.svg`. Tool IDs use `tool.*`; action IDs use `action.*` (handbook command chrome).

A descriptor's `icon_key` is already a stem, so the shell draws it directly —
there is no translation step. `Main.qml` used to route every tool stem through
a seventeen-entry fallback map keyed on `tool.*` ids, in case a descriptor
still carried an id where a stem belongs. None did, and none can: a `tool.*`
`icon_key` is not in `ICON_NAMES`, so `every_icon_key_is_packaged_into_the_qrc`
fails on it before the fallback would ever be reached. The map was a second
copy of this table that nothing checked, and it is gone.

---

## Uniqueness policy

1. **Primary stem** = the SVG in the “Stem” column (not Alts).
2. **Each primary stem is used for at most one stable ID**, except the **Shared allowlist**.
3. **Alts** are fallbacks only; they must not be another ID’s primary.
4. **Dynamic** icons (status = active tool) mirror another ID’s primary on purpose.
5. **Control state** (pan grab, dock caret flip) may swap a secondary glyph without a second primary.

### Shared allowlist (intentional multi-use)

| Primary stem | IDs | Why |
|--------------|-----|-----|
| `trash` | `action.edit.delete`, `action.layer.delete` (when present) | Same delete affordance |
| `copy-simple` | duplicate edit/layer actions | Same duplicate affordance |
| `check` | confirm / apply transform | Affirmative commit |
| `x` | close document / dismiss dialog | Close surface |
| `x-circle` | cancel / cancel transform | Generic cancel |
| `magnifying-glass` | `tool.zoom`, status zoom readout | Same zoom concept |
| `intersect` | selection intersect / shape boolean intersect | Same boolean op |

---

## 1. Left tool strip (shipped)

Source of truth: `phototux_engine::shell::default_tools()` + `tool_id::*`.  
Order = descriptor order (overflow when strip height is tight — see Interactive Stability T-016).

| Tool ID | UI label | Stem (`regular/`) | Notes |
|---------|----------|-------------------|-------|
| `tool.brush` | Brush | `paint-brush` | Active: `fill/paint-brush-fill` if present |
| `tool.eraser` | Eraser | `eraser` | |
| `tool.select.rect` | Rectangular Marquee | `selection` | |
| `tool.select.ellipse` | Elliptical Marquee | `circle-dashed` | |
| `tool.select.lasso` | Lasso | `lasso` | |
| `tool.select.polygon` | Polygonal Lasso | `polygon` | |
| `tool.move` | Move | `arrows-out-cardinal` | |
| `tool.transform` | Free Transform | `arrows-out` | Not `bounding-box` (shipping stem) |
| `tool.crop` | Crop | `crop` | |
| `tool.fill` | Fill | `paint-bucket` | |
| `tool.gradient` | Gradient | `gradient` | |
| `tool.eyedropper` | Eyedropper | `eyedropper` | |
| `tool.select.wand` | Magic Wand | `magic-wand` | Contiguous colour select |
| `tool.select.color-range` | Color Range | `selection-foreground` | Global colour select |
| `tool.clone` | Clone Stamp | `stamp` | Retouch |
| `tool.dodge` | Dodge | `sun-dim` | Retouch |
| `tool.burn` | Burn | `flame` | Retouch |
| `tool.sponge` | Sponge | `drop` | Retouch |
| `tool.blur` | Blur | `drop-half` | Retouch |
| `tool.sharpen` | Sharpen | `sparkle` | Retouch |
| `tool.smudge` | Smudge | `scribble` | Retouch |
| `tool.text` | Text | `text-t` | |
| `tool.shape` | Shape | `shapes` | Subtools are options, not strip primaries |
| `tool.path-edit` | Path Edit | `pen-nib` | |
| `tool.pan` | Pan | `hand` | Drag state may use `hand-grabbing` |
| `tool.zoom` | Zoom | `magnifying-glass` | Allowlist: status zoom |

### Tool strip chrome

| ID | UI label | Stem | Notes |
|----|----------|------|-------|
| `tools.overflow` | More tools | `dots-three` | Overflow popup when strip is short |
| `tools.presets` | Brush presets | `swatches` | Distinct from Swatches panel |

### Planned / not on strip yet

| Tool ID (reserved) | Suggested stem | Notes |
|--------------------|----------------|-------|
| `tool.pencil` | `pencil-simple` | Not in `default_tools()` yet |

---

## 2. Document lifecycle (File / toolbar)

Action IDs from `default_actions()` (shipping stems).

| Action ID | UI label | Stem | Notes |
|-----------|----------|------|-------|
| `action.file.new` | New… | `file-plus` | New Document dialog |
| `action.file.open` | Open… | `folder-open` | |
| `action.file.save` | Save | `floppy-disk` | |
| `action.file.save-as` | Save As… | `note-pencil` | When wired |
| `action.file.export` | Export… | `export` | |
| `action.file.close` | Close | `x` | Allowlist with dialog close |
| `action.edit.undo` | Undo | `arrow-counter-clockwise` | |
| `action.edit.redo` | Redo | `arrow-clockwise` | |
| `action.help.about` | About | `info` | |
| `action.view.zoom-fit` | Zoom to fit | `corners-in` | When present on action |

Additional menu actions may omit `icon_key` (text-only MenuItem) — that is OK; add a stem when the toolbar or palette needs an icon.

---

## 3. View, layers, properties (chrome targets)

Prefer handbook panel descriptors (`panel.layers`, …) for panel chrome. Suggested stems when wiring icons:

| Concept | Stem | Notes |
|---------|------|-------|
| Layers panel | `stack` | No `layers.svg` in Phosphor |
| New layer | `stack-plus` | |
| Visible / hidden | `eye` / `eye-slash` | |
| Locked / unlocked | `lock` / `lock-open` | |
| Properties | `sliders-horizontal` | |
| Color / palette | `palette` | |
| Swatches panel | `circles-three` | Distinct from `swatches` (brush presets) |
| Preferences | `gear` | `action.app.*` / Window prefs |
| Grid / rulers | `grid-four` / `ruler` | View toggles |
| Zoom in / out | `magnifying-glass-plus` / `magnifying-glass-minus` | |
| Actual size | `frame-corners` | Distinct from fit (`corners-in`) |
| Full screen | `corners-out` | |

---

## 4. Transform, selection, boolean (context / later chrome)

| Concept | Stem | Notes |
|---------|------|-------|
| Rotate 90° CW / CCW | `arrow-arc-right` / `arrow-arc-left` | **Not** undo/redo stems |
| Flip H / V | `flip-horizontal` / `flip-vertical` | |
| Apply / cancel transform | `check` / `x-circle` | Allowlist |
| Add / subtract / intersect selection | `selection-plus` / `minus-circle` / `intersect` | |
| Feather / expand / contract | `drop-half-bottom` / `plus-circle` / `minus` | |
| Shape boolean unite / subtract / intersect / exclude | `unite` / `subtract` / `intersect` / `exclude` | Align with `shape.boolean` commands |
| Align left / centre / right | `align-left` / `align-center-horizontal` / `align-right` | `AlignOp::icon_key` |
| Align top / middle / bottom | `align-top` / `align-center-vertical` / `align-bottom` | `AlignOp::icon_key` |
| Distribute horizontally / vertically | `arrows-out-line-horizontal` / `arrows-out-line-vertical` | Phosphor has no distribute glyph; the outward arrows read as spreading |

---

## 5. Reverse index (shipping strip + File/Edit toolbar)

| Stem | ID(s) |
|------|-------|
| `paint-brush` | `tool.brush` |
| `eraser` | `tool.eraser` |
| `selection` | `tool.select.rect` |
| `circle-dashed` | `tool.select.ellipse` |
| `lasso` | `tool.select.lasso` |
| `polygon` | `tool.select.polygon` |
| `arrows-out-cardinal` | `tool.move` |
| `arrows-out` | `tool.transform` |
| `crop` | `tool.crop` |
| `paint-bucket` | `tool.fill` |
| `gradient` | `tool.gradient` |
| `eyedropper` | `tool.eyedropper` |
| `text-t` | `tool.text` |
| `shapes` | `tool.shape` |
| `pen-nib` | `tool.path-edit` |
| `hand` | `tool.pan` |
| **`magnifying-glass`** | **`tool.zoom`** (+ status zoom) |
| `dots-three` | `tools.overflow` |
| `file-plus` | `action.file.new` |
| `folder-open` | `action.file.open` |
| `floppy-disk` | `action.file.save` |
| `export` | `action.file.export` |
| `arrow-counter-clockwise` | `action.edit.undo` |
| `square-half` | reset foreground / background to black and white |
| `magnet` | `action.view.toggle-snap` |
| `rectangle-dashed` | `action.view.toggle-guides` |
| `arrow-clockwise` | `action.edit.redo` |
| `info` | `action.help.about` |
| `corners-in` | zoom-fit actions |

When adding a new tool or toolbar action: update `default_tools` / `default_actions` **first**, then this map.

---

## QML resolution contract

```
Theme.iconUrl(iconRoot, stem)
  → {iconRoot}/regular/{stem}.svg   # weight may vary by helper
```

| Rule | Behavior |
|------|----------|
| Unknown stem | Missing image; prefer fallback `question` + log once |
| `filled` and fill file missing | `regular` + Theme selection chrome |
| Active tool status icon | Resolve active `tool.*` → stem from §1 |

Parity / UX: [Handbook-Parity-Checklist](../../internal_docs/Appendix/Handbook-Parity-Checklist.md), [Interactive-Stability-Checklist](../../internal_docs/Appendix/Interactive-Stability-Checklist.md).

---

## Conflicts to avoid (uniqueness)

| Collision risk | Keep | Avoid |
|----------------|------|-------|
| Undo vs rotate CCW | undo → `arrow-counter-clockwise` | Do not reuse for rotate |
| Redo vs rotate CW | redo → `arrow-clockwise` | Rotate → `arrow-arc-*` |
| Deselect vs sel.subtract | deselect → `selection-slash` | Subtract → `minus-circle` |
| Opacity vs hardness | opacity → `drop-half` | Hardness → `circle-half` |
| Fit vs 100% vs fullscreen | `corners-in` / `frame-corners` / `corners-out` | Do not share one “corners” glyph |
| Brush presets vs Swatches panel | presets → `swatches` | Panel → `circles-three` |
| Free Transform vs Move | transform → `arrows-out` | Move → `arrows-out-cardinal` |

---

## Verification

### Stems exist (shipping set)

```bash
ICON_ROOT=assets/icons/phosphor/regular
for s in paint-brush eraser selection circle-dashed lasso polygon \
  arrows-out-cardinal arrows-out crop paint-bucket gradient eyedropper \
  text-t shapes pen-nib hand magnifying-glass dots-three \
  file-plus folder-open floppy-disk export \
  arrow-counter-clockwise arrow-clockwise info corners-in; do
  test -f "$ICON_ROOT/$s.svg" && echo "OK $s" || echo "MISSING $s"
done
```

### Descriptor sync

After changing tools/actions in Rust, confirm QML still resolves stems (filesystem `PHOTOTUX_QML` or qrc) and Accessible names remain on icon-only buttons.

---

*Aligned with handbook + shipping descriptors: 2026-07-18.*
