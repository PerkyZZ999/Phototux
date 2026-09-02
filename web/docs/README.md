# docs.phototux.xyz

The PhotoTux user documentation.

```bash
pnpm install          # from web/, once
pnpm dev              # http://localhost:4322
pnpm build            # static output in dist/
pnpm preview          # serve dist/
pnpm check            # astro check
```

## Adding a page

1. Write the Markdown under `src/content/docs/`. The file path is the URL:
   `guides/layers.md` becomes `/guides/layers/`.
2. Give it `title` and `description` frontmatter. Both are required, and the
   description is what appears under the heading and in search results.
3. **Add it to `src/nav.ts`.** The sidebar, the previous/next links and the
   search index all come from that file, and the build fails on a page that is
   in one and not the other — an orphaned page nobody can reach is a page
   nobody reads.

Headings get ids and anchor links automatically. Tables wider than the measure
need wrapping in `<div class="table-wrap">` so the page itself never scrolls
sideways.

Callouts are plain HTML:

```html
<div class="callout callout-note">

**Title.** Body text, in Markdown.

</div>
```

`callout-note`, `callout-tip`, `callout-warning` and `callout-danger`.

## Design

Shared with the product site through `@phototux/design`. See
`web/landing/README.md` for the rules — same tokens, same icon set, no second
palette.

## Assets

`public/brand/logo.png` and `public/screenshots/` are committed copies of the
root `assets/`, for the same reason they are in the landing site: each site is
self-contained.

## Search

`src/pages/search-index.json.ts` emits titles, descriptions and headings at
build time, and `src/components/Search.astro` filters it in the browser. Body
text is deliberately not indexed — it would be several hundred kilobytes for a
reader to download before their first keystroke.

## Deploying

Static files. Point any host at `dist/` after `pnpm build`, with
`docs.phototux.xyz` resolving to it.
