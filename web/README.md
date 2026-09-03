# PhotoTux on the web

Two static sites, one design system.

| Directory | Domain | What it is |
|---|---|---|
| [`landing/`](landing/) | `phototux.xyz` | The product site — one page presenting the editor. |
| [`docs/`](docs/) | `docs.phototux.xyz` | The user documentation. |
| [`packages/design/`](packages/design/) | — | Design tokens, base styles, the icon bundle and the brand facts both sites share. |

## Getting started

```bash
cd web
pnpm install

pnpm dev:landing      # http://localhost:4321
pnpm dev:docs         # http://localhost:4322

pnpm build            # both sites, into landing/dist and docs/dist
pnpm check            # astro check on both
```

Requires Node 22.12 or newer and pnpm. Both sites are Astro 7 with no
framework islands, so the output is HTML, CSS and a few kilobytes of vanilla
JavaScript.

`./scripts/check-web.sh` from the repository root runs the same build, and
installs dependencies first on a cold checkout. Run it after touching anything
under `web/`: neither `rust-tc doctor` nor `check-docs-links.py` compiles Astro,
so a template that does not parse is invisible to every other gate. The landing
page shipped for a day with two malformed `<img>` tags for exactly that reason.

## Why one design package

The sites share `@phototux/design`: colour, type, space and shape tokens, the
base element styles, the long-form prose styles, the Phosphor icon bundle, and
the facts — version, repository URL, requirements, build command — that would
otherwise be written down twice and go stale in one of them.

The colours are the **editor's own**, taken from `qml/Theme.qml`. That is
deliberate: a screenshot of PhotoTux should look like it belongs on the page
it is sitting on. Do not introduce a second palette in either site.

Two departures from the application palette:

1. The page ground is darker than the application's, so a screenshot reads as
   an object placed on the page rather than dissolving into it.
2. There is a light theme. The editor is dark-only; a website is read in
   daylight, on other people's machines, under other people's settings.

## Icons

Phosphor, the same set the editor ships, generated into
`packages/design/src/icons.js` from `assets/icons/phosphor/regular` by
`packages/design/build-icons.mjs`. To use an icon that is not in the bundle,
add its stem to `WANTED` in that script and run it:

```bash
node packages/design/build-icons.mjs
```

The icons are inlined into the HTML at build time, so only the ones a page
actually renders reach the browser.

## Assets

Each site keeps **its own committed copies** of the logo and the screenshots
under `public/`. They are not synced from the repository root at build time —
each site has to build, preview and deploy on its own without reaching outside
its directory.

The sources are `assets/logo-ui.png`, `assets/og-card.png` and
`assets/screenshots/` at the repository root. When any of them changes, copy it
into both `landing/public/` and `docs/public/`.

`og-card.png` is the link preview, 1200×630 as Open Graph expects — a different
shape from the screenshots, and composed rather than captured.

## Deploying

Both are static. Build, then point a host at `dist/`:

| Site | Build | Serve | DNS |
|---|---|---|---|
| Landing | `pnpm build:landing` | `landing/dist` | `phototux.xyz` and `www.phototux.xyz` |
| Docs | `pnpm build:docs` | `docs/dist` | `docs.phototux.xyz` |

Neither needs a server runtime, a database or an API. Any static host —
GitHub Pages, Cloudflare Pages, Netlify, Vercel, or nginx on a box — will do.

Two things to configure wherever they land:

- **Trailing slashes.** Pages are emitted as `path/index.html`, so
  `/guides/layers/` is the canonical form. Redirect `/guides/layers` to it.
- **404s.** Each site emits its own `404.html`.

## Conventions

- Author in Markdown for the docs, `.astro` for chrome. See
  [`docs/README.md`](docs/README.md) for how to add a page.
- Colours, spacing and type come from `@phototux/design`. No literals.
- Every image gets a real `alt`, explicit `width` and `height`, and
  `loading="lazy"` unless it is above the fold.
- Anything wide — a table, a code block, a tab row — gets its own
  `overflow-x` container. The page body never scrolls sideways.
- Focus is never removed, only replaced.
- Colour is never the only indicator of a state.
