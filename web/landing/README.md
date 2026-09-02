# phototux.xyz

The PhotoTux product site: one page presenting the editor, its features, its
screenshots, and how to build it.

```bash
pnpm install          # from web/, once
pnpm dev              # http://localhost:4321
pnpm build            # static output in dist/
pnpm preview          # serve dist/
pnpm check            # astro check
```

Astro 7, no framework islands. The output is HTML, one stylesheet and about
4 kB of vanilla JavaScript for the theme toggle, the screenshot tabs, the
distribution picker and the copy buttons.

## Design

Tokens, base styles and brand facts come from `@phototux/design`, the workspace
package shared with the documentation site. Colours are the editor's own —
`qml/Theme.qml` — so a screenshot of the application looks like it belongs on
the page it sits on. Do not add a second palette here.

Icons are Phosphor, inlined from `@phototux/design/icons`, which is generated
from the same SVGs the editor ships. To use one that is not in the bundle, add
its stem to `WANTED` in `web/packages/design/build-icons.mjs` and run that
script.

## Assets

`public/brand/logo.png` and `public/screenshots/` are **committed copies** of
`assets/logo-ui.png` and `assets/screenshots/` at the repository root. They
live here rather than being synced at build time so the site is
self-contained: it builds, previews and deploys without reaching outside its
own directory.

When the root copies change, copy them here and into
`web/docs/public/` as well. `workspace.png` is kept alongside the WebP because
some crawlers will not read WebP for an Open Graph image.

## Deploying

Static files. Point any host at `dist/` after `pnpm build`, with
`phototux.xyz` resolving to it.
