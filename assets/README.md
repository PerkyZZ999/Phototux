# Assets

| Path | What it is |
|---|---|
| `logo-ui.png` | The PhotoTux mark — Tux with a paintbrush and a trail of blue pixels. Shown on the editor's welcome screen and used as the mark on both websites. The plainer glyph in `packaging/linux/` is the packaged desktop icon, not the brand mark. |
| `og-card.png` | The link preview card, 1200×630 as Open Graph expects. Composed from `logo-ui.png` and `screenshots/workspace.webp` on the websites' own background. |
| `screenshots/` | Captures from a real build. See [`screenshots/README.md`](screenshots/README.md). |
| `icons/phosphor/` | Phosphor Icons 2.1.1 (MIT), the icon set the editor and both websites draw from. |

Each website keeps its own committed copies of the logo, the card and the
screenshots under `web/<site>/public/` so it can be built and deployed on its
own. When anything here changes, copy it into both.
