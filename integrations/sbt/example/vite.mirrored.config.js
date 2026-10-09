import { spawnSync } from "node:child_process"
import { existsSync } from "node:fs"
import { defineConfig } from "vite"

// The page served the way the reference application serves its own: a plugin of the build's own
// prints a task of the build's own, `servedOutput`, the mirror of the linker's directory
// (build.sbt, project `mirrored`), and resolves `scalajs:` imports in it. Nothing here or in the
// entry names teq: with TEQ_COMPILER=1 in sbt's environment the mirror holds teq's output, the
// stub `main.js` among it, and without it the Scala.js linker's. `sbt ~mirrored/fastLinkJS`
// keeps the mirror current in dev; `vite build` runs fullLinkJS through the plugin.
function mirrored({ projectID, prefix = "scalajs:" }) {
  let dev = true
  let directory
  return {
    name: "example-mirrored",
    configResolved(config) {
      dev = config.command === "serve"
    },
    buildStart() {
      const task = `${projectID}/Compile/${dev ? "fastLinkJS" : "fullLinkJS"}/servedOutput`
      const sbt = spawnSync("sbt", ["--batch", "-no-colors", "-Dsbt.supershell=false", `print ${task}`], { encoding: "utf-8", stdio: ["ignore", "pipe", "inherit"] })
      if (sbt.status !== 0) throw new Error(`sbt print ${task} exited ${sbt.status}:\n${sbt.stdout}`)
      // sbt 2 follows the value with a success line and a terminal escape
      directory = sbt.stdout.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, "").split("\n").map((line) => line.trim()).filter((line) => line && !line.startsWith("[")).at(-1)
      if (!directory || !existsSync(directory)) throw new Error(`sbt print ${task} named no directory:\n${sbt.stdout}`)
    },
    resolveId(source) {
      if (source.startsWith(prefix)) return `${directory}/${source.slice(prefix.length)}`
    },
  }
}

export default defineConfig({
  plugins: [mirrored({ projectID: "mirrored" })],
  build: { outDir: "dist-mirrored" },
})
