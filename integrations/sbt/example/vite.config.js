import { defineConfig } from "vite"
import teq from "vite-plugin-teq"

// `vite` serves main.js with the frontend's modules under `scalajs:`; `vite build` bundles and minifies
// it into dist/frontend.js, which node runs. The project is the export's, teq.lock.
export default defineConfig({
  plugins: [teq({ project: "frontend" })],
  build: { lib: { entry: "main.js", formats: ["es"], fileName: "frontend" } },
})
