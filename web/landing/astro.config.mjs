// @ts-check
import { defineConfig } from "astro/config";

// https://astro.build/config
export default defineConfig({
  site: "https://phototux.xyz",
  // A single page of hand-written HTML and CSS. Nothing here needs a
  // framework island, so the output is HTML plus about 4 kB of vanilla JS.
  build: { inlineStylesheets: "auto" },
  devToolbar: { enabled: false },
});
