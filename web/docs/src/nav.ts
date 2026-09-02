/**
 * The order a reader is meant to meet the documentation in.
 *
 * Explicit rather than derived from the folder tree: the sidebar order, the
 * previous/next links at the foot of each page and the sitemap all come from
 * this one list, and "install before tour before first edit" is an editorial
 * decision that a directory listing would get alphabetically wrong.
 *
 * Every `slug` here must match a file under `src/content/docs`; the guard in
 * `src/pages/[...slug].astro` fails the build if one does not.
 */

export interface NavItem {
  slug: string;
  label: string;
}

export interface NavGroup {
  title: string;
  icon: string;
  items: NavItem[];
}

export const NAV: NavGroup[] = [
  {
    title: "Getting started",
    icon: "rocket-launch",
    items: [
      { slug: "index", label: "Introduction" },
      { slug: "guides/installation", label: "Installing PhotoTux" },
      { slug: "guides/tour", label: "A tour of the workspace" },
      { slug: "guides/first-edit", label: "Your first edit" },
    ],
  },
  {
    title: "Guides",
    icon: "book-open-text",
    items: [
      { slug: "guides/layers", label: "Working with layers" },
      { slug: "guides/selections", label: "Selections and masks" },
      { slug: "guides/adjustments", label: "Adjustments and filters" },
      { slug: "guides/text-shapes", label: "Text, shapes and smart objects" },
      { slug: "guides/transform", label: "Transform, crop and canvas" },
      { slug: "guides/files", label: "Opening, saving and exporting" },
      { slug: "guides/workspace", label: "Panels and preferences" },
    ],
  },
  {
    title: "Reference",
    icon: "article",
    items: [
      { slug: "reference/tools", label: "Tools" },
      { slug: "reference/shortcuts", label: "Keyboard shortcuts" },
      { slug: "reference/blend-modes", label: "Blend modes" },
      { slug: "reference/file-formats", label: "File formats" },
    ],
  },
  {
    title: "Help",
    icon: "lifebuoy",
    items: [
      { slug: "troubleshooting", label: "Troubleshooting" },
      { slug: "contributing", label: "Contributing" },
    ],
  },
];

/** Flat reading order, for previous/next links. */
export const NAV_FLAT: NavItem[] = NAV.flatMap((group) => group.items);

/** `index` is the site root; everything else keeps a trailing slash. */
export function hrefFor(slug: string): string {
  return slug === "index" ? "/" : `/${slug}/`;
}

/** The group a slug belongs to, for the eyebrow above a page title. */
export function groupFor(slug: string): NavGroup | undefined {
  return NAV.find((group) => group.items.some((item) => item.slug === slug));
}
