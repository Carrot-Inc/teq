import { defineConfig } from "vite"
import teq from "vite-plugin-teq"

// The same browserdemo page as vite.scalajs.config.js, served instead through vite-plugin-teq: its
// own resident `teq compiler watch` and its own hot-swap invalidation, for the dev-loop measurement of
// docs/TOOLING.md ("The vite plugin") against the sbt-driven path.
// vite-plugin-teq serves teq's own `main.mjs` in the place of sbt-teq's stub, so the entry that
// accepts its own updates is main.js, not the page's main.scalajs.js.
export default defineConfig({
  plugins: [
    teq({ project: "browserdemo" }),
    { name: "accepting-entry", transformIndexHtml: (html) => html.replace("/main.scalajs.js", "/main.js") },
  ],
})
