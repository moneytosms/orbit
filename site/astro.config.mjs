// @ts-check
import { defineConfig } from "astro/config";
import { unified } from "@astrojs/markdown-remark";
import { rewriteRepoLinks } from "./src/lib/remark-repo-links.mjs";
import cfg from "./site.config.mjs";
import { GUIDE_REDIRECTS } from "./src/lib/slug.mjs";

export default defineConfig({
  site: cfg.site,
  base: cfg.base,
  redirects: GUIDE_REDIRECTS,
  trailingSlash: "ignore",
  // Preserve spaces around inline expressions and links under Astro 7.
  compressHTML: true,
  markdown: {
    processor: unified({ remarkPlugins: [rewriteRepoLinks] }),
    shikiConfig: {
      themes: { light: "github-light", dark: "github-dark-dimmed" },
      defaultColor: false,
    },
  },
});
