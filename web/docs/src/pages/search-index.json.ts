/**
 * The search index, built at compile time.
 *
 * Titles, descriptions and headings only — not body text. Seventeen pages of
 * prose would be several hundred kilobytes for a reader to download before
 * their first keystroke, and on a documentation site of this size the heading
 * a thing lives under is nearly always the thing being looked for.
 */
import type { APIRoute } from "astro";
import { getCollection, render } from "astro:content";
import { NAV_FLAT, groupFor, hrefFor } from "../nav";

export const GET: APIRoute = async () => {
  const docs = await getCollection("docs");
  const byId = new Map(docs.map((doc) => [doc.id, doc]));

  const index = [];
  for (const item of NAV_FLAT) {
    const doc = byId.get(item.slug);
    if (!doc) continue;
    const { headings } = await render(doc);
    index.push({
      slug: doc.id,
      href: hrefFor(doc.id),
      title: doc.data.title,
      description: doc.data.description,
      section: groupFor(doc.id)?.title ?? "Documentation",
      headings: headings
        .filter((heading) => heading.depth === 2 || heading.depth === 3)
        .map((heading) => ({ text: heading.text, id: heading.slug })),
    });
  }

  return new Response(JSON.stringify(index), {
    headers: { "content-type": "application/json; charset=utf-8" },
  });
};
