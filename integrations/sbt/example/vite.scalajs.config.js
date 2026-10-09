import { defineConfig } from "vite"
import scalajs from "@scala-js/vite-plugin-scalajs"

// The same frontend as vite.config.js (vite-plugin-teq), served instead through the stock Scala.js
// plugin: with TEQ_COMPILER=1 in sbt's environment, sbt-teq answers browserdemo's fastLinkJS and
// fullLinkJS from teq's output (docs/TARGETS.md, "The sbt plugin"), so the plugin runs unmodified:
// `sbt ~browserdemo/fastLinkJS` in dev, `vite build --config vite.scalajs.config.js` running
// fullLinkJS itself through the plugin.
export default defineConfig({
  plugins: [scalajs({ projectID: "browserdemo" })],
  build: { outDir: "dist-scalajs" },
})
