// @ts-check
import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";

export default defineConfig({
  site: "https://teq.build",
  trailingSlash: "always",
  integrations: [sitemap()],
  // The page's one script stays a file under /_astro/ so the CSP in public/_headers can name 'self' alone.
  vite: { build: { assetsInlineLimit: 0 } },
  markdown: {
    // Shiki with one dark theme: code blocks are dark on both of the page's themes.
    shikiConfig: { theme: "vitesse-dark" },
  },
});
