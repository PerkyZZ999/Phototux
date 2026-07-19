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

## Icon → action mapping

**Full tables (Action ID, UI label, SVG stem, phases):** see **[ICON_MAP.md](./ICON_MAP.md)**.

That file is the source of truth for wiring QML `iconSource(actionId)`. Examples: brush → `paint-brush`, layers → `stack`, undo → `arrow-counter-clockwise`.

## Usage notes (Qt Quick)

- Prefer **SVG + `currentColor`-style tinting** via Qt (e.g. colorized `Image` / icon engines) so dark theme works without duplicate assets.
- Keep tool icons on a **consistent size grid** (e.g. 20–24px glyph in 36px hit target per `qml/Theme.qml` / handbook Themes).
- Do not mix a second icon family for tools without an ADR amendment.
- **Do not** recolor by editing thousands of SVGs — tint at runtime.

## Updating the pack

1. Fetch a newer `@phosphor-icons/core` release / git tag.
2. Replace `phosphor/{weights}/` SVGs and update `VERSION` + `LICENSE` if changed.
3. Note the bump in `CHANGELOG.md` / journal.

## App brand icon

Application launcher icon may remain custom later; **tool and chrome glyphs** come from this pack.
