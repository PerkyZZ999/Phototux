import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";

/**
 * Every documentation page.
 *
 * The id is the path under `src/content/docs`, so `guides/layers.md` becomes
 * `/guides/layers/`. Ordering and grouping are *not* derived from the folder
 * tree — a reader's route through documentation is an editorial decision, and
 * it lives in `src/nav.ts`.
 */
const docs = defineCollection({
  loader: glob({ pattern: "**/*.md", base: "./src/content/docs" }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    /** Shown above the title; the group the page belongs to. */
    section: z.string().optional(),
    /** Set on pages that describe behaviour still in flux. */
    draft: z.boolean().default(false),
  }),
});

export const collections = { docs };
