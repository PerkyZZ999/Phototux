# Icons — PhotoTux

Vendored **Phosphor Icons** for desktop GUI chrome (tool strip, menus, docks, toolbar).

**Normative product docs:** [`internal_docs/`](../../internal_docs/README.md) (Engineering Handbook).  
**Stack lock:** [DR-023](../../internal_docs/Appendix/Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase) (Phosphor under `assets/icons/phosphor/`).  
**Presentation tokens:** [`qml/Theme.qml`](../../qml/Theme.qml) + [25 — Themes](../../internal_docs/25-Themes.md).  
**Toolbar / tools contracts:** [06 — Toolbar System](../../internal_docs/06-Toolbar-System.md).  
**Action / command IDs:** [Command-Taxonomy](../../internal_docs/Appendix/Command-Taxonomy.md) + `ActionDescriptor` / `ToolDescriptor` in `phototux_engine`.

This folder documents **glyph wiring only**. Do not invent a second icon family or a parallel action taxonomy here.

## Pack: Phosphor Icons

| | |
|--|--|
| **Name** | Phosphor Icons |
| **Upstream** | [phosphor-icons/core](https://github.com/phosphor-icons/core) (`@phosphor-icons/core` **2.1.1**) |
| **Homepage** | [phosphoricons.com](https://phosphoricons.com) |
| **License** | **MIT** — see `phosphor/LICENSE` (OK with GPL-3.0-or-later app bundling) |
| **Vendored path** | `assets/icons/phosphor/` |
| **Format** | SVG (`viewBox="0 0 256 256"`, `fill="currentColor"`) |
| **Weights** | `thin`, `light`, `regular`, `bold`, `fill`, `duotone` |

**Default weight for dense editor chrome:** `regular` (or `bold` for 16–20px glyphs if regular reads thin).  
**Selected / emphasis:** `fill` when a filled variant helps; otherwise keep `regular` and use Theme selection chrome.

## Layout

```
assets/icons/
├── README.md          # this file
├── ICON_MAP.md        # stem ↔ tool/action ID tables
└── phosphor/
    ├── LICENSE
    ├── VERSION        # 2.1.1
    ├── UPSTREAM
    ├── HOMEPAGE
    ├── regular/*.svg
    ├── bold/*.svg
    ├── fill/*.svg
    ├── light/*.svg
    ├── thin/*.svg
    └── duotone/*.svg
```

## How QML resolves icons

1. Descriptors expose a Phosphor **stem** as `icon_key` (not a filesystem path).
2. QML calls `Theme.iconUrl(AppSession.iconRoot, stem)` →  
   `{iconRoot}/regular/{stem}.svg` (filesystem or `qrc` depending on build).
3. Tool strip may also map legacy `tool.*` keys through `toolIconStemMap` in `qml/Main.qml`; prefer stems already on `ToolDescriptor.icon_key`.

**Canonical stem tables:** [ICON_MAP.md](./ICON_MAP.md).

## Usage notes (Qt Quick)

- Prefer SVG + runtime tint (`currentColor` / Theme colors) so dark / high-contrast themes work without duplicate assets ([25 — Themes](../../internal_docs/25-Themes.md)).
- Hit targets follow Theme density (`Theme.toolHit` ≈ 40px; glyph ≈ 18–20px on the tool strip).
- Icon-only controls **MUST** still expose Accessible names (handbook [29 — Accessibility](../../internal_docs/29-Accessibility.md)).
- Do **not** mix a second icon family for tools without a Decision Register amendment (DR-023).
- Do **not** recolor by editing thousands of SVGs — tint at runtime.

## Updating the pack

1. Fetch a newer `@phosphor-icons/core` release / git tag.
2. Replace `phosphor/{weights}/` SVGs; update `VERSION` + `LICENSE` if needed.
3. Note the bump in root `CHANGELOG.md`.
4. Re-run the stem existence check in [ICON_MAP.md](./ICON_MAP.md).

## App brand

Launcher / about art may stay custom (`assets/logo-ui.png`). **Tool and chrome glyphs** come from this Phosphor pack.
