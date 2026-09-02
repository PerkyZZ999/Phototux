// @ts-check
import { defineConfig } from "astro/config";
import { satteri } from "@astrojs/markdown-satteri";
import GithubSlugger from "github-slugger";

/*
 * The two hast plugins below take `any` for their visitor parameters. Their
 * real types live in `satteri`, which reaches this project only as a
 * transitive dependency of Astro — importing the types would mean pinning a
 * second version of Astro's own Markdown compiler in this package.json just to
 * annotate two callbacks in a build config.
 */

/**
 * The Phosphor `hash` glyph, as raw HTML.
 *
 * Inlined rather than imported from `@phototux/design/icons` because this file
 * runs in the config, before the module graph the sites use exists.
 */
const HASH_ICON =
  '<svg class="icon icon-sm" viewBox="0 0 256 256" aria-hidden="true" focusable="false">' +
  '<path d="M224,88a8,8,0,0,1-8,8H183.83l-7.6,64H208a8,8,0,0,1,0,16H174.33l-6.39,53.94a8,8,0,0,1-7.94,7.06,9,9,0,0,1-.95-.06,8,8,0,0,1-7-8.88L158.22,176H94.33l-6.39,53.94a8,8,0,0,1-7.94,7.06,9,9,0,0,1-.95-.06,8,8,0,0,1-7-8.88L78.22,176H40a8,8,0,0,1,0-16H80.12l7.6-64H56a8,8,0,0,1,0-16H89.62l6.44-54.94a8,8,0,0,1,15.88,1.88L105.73,80h63.89l6.44-54.94a8,8,0,0,1,15.88,1.88L185.73,80H216A8,8,0,0,1,224,88ZM167.83,96H103.94l-7.6,64h63.89Z"/>' +
  "</svg>";

/**
 * Give every heading an id and a link to itself.
 *
 * The id is set here rather than left to Astro so the anchor's `href` and the
 * heading's `id` cannot disagree; Astro preserves an id a plugin has already
 * written. A slugger is created per document, so two headings with the same
 * words on one page get `-1` rather than colliding.
 *
 * The anchor is a real focusable link, not a hover-only affordance: CSS fades
 * it in on hover and shows it outright on focus and on touch.
 */
function headingAnchors() {
  let slugger = new GithubSlugger();
  return {
    name: "phototux-heading-anchors",
    before() {
      slugger = new GithubSlugger();
    },
    element: {
      filter: ["h2", "h3", "h4"],
      /** @param {any} node @param {any} ctx */
      visit(node, ctx) {
        const existing = node.properties?.id;
        const id =
          typeof existing === "string" && existing.length > 0
            ? existing
            : slugger.slug(ctx.textContent(node));
        ctx.setProperty(node, "id", id);
        ctx.appendChild(node, {
          type: "element",
          tagName: "a",
          properties: {
            className: ["heading-anchor"],
            href: `#${id}`,
            "aria-label": "Link to this section",
          },
          children: [{ type: "raw", value: HASH_ICON }],
        });
      },
    },
  };
}

/**
 * Put every table in its own horizontal scroller.
 *
 * A reference table is wider than the prose measure more often than not, and a
 * page that scrolls sideways as a whole is a page that is broken on a phone.
 * Done here rather than by asking each writer to remember a wrapper `div`,
 * because the one they forget is the one that breaks.
 */
function tableWrap() {
  return {
    name: "phototux-table-wrap",
    element: {
      filter: ["table"],
      /** @param {any} node @param {any} ctx */
      visit(node, ctx) {
        ctx.wrapNode(node, {
          type: "element",
          tagName: "div",
          properties: { className: ["table-wrap"] },
          children: [],
        });
      },
    },
  };
}

// https://astro.build/config
export default defineConfig({
  site: "https://docs.phototux.xyz",
  build: { inlineStylesheets: "auto" },
  devToolbar: { enabled: false },
  markdown: {
    processor: satteri({ hastPlugins: [headingAnchors(), tableWrap()] }),
    shikiConfig: {
      themes: { light: "github-light", dark: "github-dark-default" },
      wrap: false,
    },
  },
});
