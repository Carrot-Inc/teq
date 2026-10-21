// The dev server's swap and reload over the plugin, with vite's real server and module graph and
// teq replaced by test-dev/teq.js (registered by test-dev/loader.mjs): the page's modules enter
// the graph as a browser's requests put them there, a source change rebuilds, and the rewritten
// module is what the server answers next. On Windows the plugin's paths meet vite's in its form.
import { register } from "node:module"
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { basename, join } from "node:path"
import { pathToFileURL } from "node:url"

register("./test-dev/loader.mjs", import.meta.url)
const { createServer } = await import("vite")
const { default: teq } = await import("./index.js")

let failed = 0
const check = (ok, what) => {
  console.log(`${ok ? "ok" : "FAIL"} ${what}`)
  if (!ok) failed++
}

const PORT = 57311
const page = (name, text) => `export const page = ${JSON.stringify(text)}\nconsole.log(${JSON.stringify(name)})\n`
const first = {
  "main.mjs": 'import { page } from "./app.page.mjs"\nimport "./shared.mjs"\nconsole.log("main", page)\n',
  "app.page.mjs": page("page", "first"),
  "shared.mjs": "export const shared = 1\n",
  "hot-build.mjs": "export const listing = 1\n",
}

async function session(accepting) {
  const root = mkdtempSync(join(tmpdir(), "teq-vite-dev-"))
  mkdirSync(join(root, "src"), { recursive: true })
  writeFileSync(join(root, "src", "A.scala"), "object A\n")
  writeFileSync(join(root, "index.html"), '<script type="module" src="/main.js"></script>\n')
  writeFileSync(join(root, "main.js"), `import "scalajs:main.js"\n${accepting ? "import.meta.hot?.accept()\n" : ""}`)
  globalThis.teqUnderTest = { first, next: { "app.page.mjs": page("page", "second") } }
  const lines = []
  const logger = { info: (m) => lines.push(m), warn: (m) => lines.push(m), error: (m) => lines.push(m), warnOnce: (m) => lines.push(m), clearScreen() {}, hasErrorLogged: () => false, hasWarned: false }
  const server = await createServer({ root, configFile: false, logLevel: "silent", customLogger: logger, server: { port: PORT, strictPort: false, host: "127.0.0.1" }, plugins: [teq({ project: "web" })] })
  const sent = []
  const hot = server.environments?.client?.hot ?? server.hot ?? server.ws
  const send = hot.send.bind(hot)
  hot.send = (payload) => {
    sent.push(payload)
    return send(payload)
  }
  await server.listen()
  const outName = `${basename(root)}-teq-${PORT}-staged`
  const url = (name) => `/target/teq/${outName}/${name}`
  try {
    await server.transformRequest("/main.js")
    const entry = await server.transformRequest(url("main.mjs"))
    check(entry?.code.includes("app.page.mjs"), `${accepting ? "accepting" : "plain"}: the entry is served through the plugin`)
    const before = await server.transformRequest(url("app.page.mjs"))
    check(before?.code.includes('"first"'), `${accepting ? "accepting" : "plain"}: the per-file module is served as first written`)
    const swapped = new Promise((resolve) => {
      const started = lines.length
      const look = () => {
        const line = lines.slice(started).find((l) => l.includes("module changed"))
        line ? resolve(line) : setTimeout(look, 20)
      }
      look()
    })
    server.watcher.emit("change", join(root, "src", "A.scala"))
    const line = await Promise.race([swapped, new Promise((r) => setTimeout(() => r("TIMEOUT"), 10000))])
    const after = await server.transformRequest(url("app.page.mjs"))
    const stale = !after?.code.includes('"second"')
    if (accepting) {
      check(line.includes("hot swap, 1 module re-executed"), `accepting: a change is a hot swap (${line})`)
      const update = sent.find((p) => p.type === "update")
      check(update?.updates.some((u) => u.path === "/main.js"), "accepting: the update names the accepting importer")
    } else {
      check(line.includes("page reload: no importer"), `plain: a change without an accepting importer reloads (${line})`)
      check(sent.some((p) => p.type === "full-reload"), "plain: the reload is sent")
    }
    check(!stale, `${accepting ? "accepting" : "plain"}: the rewritten module is what the server answers next`)
  } finally {
    await server.close()
    rmSync(root, { recursive: true, force: true })
  }
}

await session(true)
await session(false)
console.log(failed ? `vite-plugin-teq dev: ${failed} failed` : "vite-plugin-teq dev: all passed")
process.exit(failed ? 1 : 0)
