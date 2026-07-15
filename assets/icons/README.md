# Icons — PhotoTux

Bundled icon family for the **desktop GUI** chrome (tool strip, menus, docks). ADR-013 G15.

## Pack: Phosphor Icons

| | |
|--|--|
| **Name** | Phosphor Icons |
| **Upstream assets** | [phosphor-icons/core](https://github.com/phosphor-icons/core) (`@phosphor-icons/core` **2.1.1**) |
| **Homepage** | [phosphor-icons/homepage](https://github.com/phosphor-icons/homepage) · [phosphoricons.com](https://phosphoricons.com) |
| **License** | **MIT** — see `phosphor/LICENSE` (compatible with GPL-3.0-or-later app linking/bundling) |
| **Vendored path** | `assets/icons/phosphor/` |
| **Format** | SVG (`viewBox="0 0 256 256"`, `fill="currentColor"`) |
| **Weights** | `thin`, `light`, `regular`, `bold`, `fill`, `duotone` — **1512** icons each |

**Default weight for dense editor chrome:** `regular` (or `bold` for 16–20px tool glyphs if regular is too thin).  
**Selected / emphasis:** `fill` where a filled variant reads better.

## Layout

```
assets/icons/
├── README.md                 # this file
└── phosphor/
    ├── LICENSE
    ├── VERSION               # 2.1.1
    ├── UPSTREAM
    ├── HOMEPAGE
    ├── regular/*.svg
    ├── bold/*.svg
    ├── fill/*.svg
    ├── light/*.svg
    ├── thin/*.svg
    └── duotone/*.svg
```

## Suggested mapping (PhotoTux tools / chrome)

Use these filenames under `phosphor/regular/` (swap weight folder as needed). Names follow Phosphor’s kebab-case SVG files.

| UI role | Phosphor SVG (regular) | Notes |
|---------|------------------------|--------|
| Brush | `paint-brush.svg` | Primary paint tool |
| Pencil | `pencil-simple.svg` | Hard edge / sketch |
| Eraser | `eraser.svg` | |
| Eyedropper | `eyedropper.svg` | Color pick |
| Selection (rect) | `selection.svg` | Or `rectangle` / `bounding-box` |
| Lasso-ish free select | `lasso.svg` | If present; else `polygon` |
| Move / pan | `hand-grabbing.svg` / `hand.svg` | Space-pan cursor |
| Transform | `arrows-out-cardinal.svg` | Scale/move handles context |
| Crop | `crop.svg` | |
| Text | `text-t.svg` | |
| Shape | `shapes.svg` | |
| Fill / bucket | `paint-bucket.svg` | |
| Zoom | `magnifying-glass.svg` | |
| Zoom in / out | `magnifying-glass-plus.svg` / `magnifying-glass-minus.svg` | |
| Layers | `stack.svg` | Layers panel |
| Layer visible | `eye.svg` | |
| Layer hidden | `eye-slash.svg` | |
| Lock | `lock.svg` / `lock-open.svg` | |
| New document | `file-plus.svg` | |
| Open | `folder-open.svg` | |
| Save | `floppy-disk.svg` | |
| Export | `export.svg` | |
| Undo | `arrow-counter-clockwise.svg` | |
| Redo | `arrow-clockwise.svg` | |
| Copy / paste | `copy.svg` / `clipboard.svg` | |
| Trash / delete | `trash.svg` | |
| Settings | `gear.svg` | |
| Properties / sliders | `sliders-horizontal.svg` | |
| Close | `x.svg` | |
| Menu overflow | `dots-three.svg` | |
| Info | `info.svg` | |
| Warning | `warning.svg` | |
| Image | `image.svg` | Document type |
| Palette | `palette.svg` | Color UI |
| Desktop / window | `desktop.svg` | About / platform |

Exact presence: verify with `ls assets/icons/phosphor/regular/<name>.svg` before wiring QML `Image` / `icon.source`.

## Usage notes (Qt Quick)

- Prefer **SVG + `currentColor`-style tinting** via Qt (e.g. colorized `Image` / icon engines) so dark theme works without duplicate assets.
- Keep tool icons on a **consistent size grid** (e.g. 20–24px glyph in 36px hit target per `DESIGN.md`).
- Do not mix a second icon family for tools without an ADR amendment.
- **Do not** recolor by editing thousands of SVGs — tint at runtime.

## Updating the pack

1. Fetch a newer `@phosphor-icons/core` release / git tag.
2. Replace `phosphor/{weights}/` SVGs and update `VERSION` + `LICENSE` if changed.
3. Note the bump in `CHANGELOG.md` / journal.

## App brand icon

Application launcher icon may remain custom later; **tool and chrome glyphs** come from this pack.
