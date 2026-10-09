// Times the edit-to-DOM loop of the dev paths on browserdemo's page, five edits each, printing the
// per-edit timings and the average. `vite`: vite-plugin-teq's own loop (a resident `teq compiler watch` the
// plugin drives from its file watcher), the server started here. `sbt`: `sbt --batch
// browserdemo/fastLinkJS` run per edit against a warm server and its resident watch
// (TEQ_COMPILER=1 in sbt's environment), the server of vite.scalajs.config.js started here, which
// isolates sbt's task-graph evaluation from teq's own build time. `watch`: the developer's loop,
// `TEQ_COMPILER=1 sbt ~browserdemo/fastLinkJS` in a terminal of its own (sbt's `~` wants one) and
// that server on VITE_SCALAJS_PORT (5393), both already running.
// `node measure-loop.mjs vite|sbt|watch`.
import { spawn, spawnSync } from "node:child_process"
import { readFileSync, writeFileSync } from "node:fs"
import puppeteer from "puppeteer-core"
import { chromePath } from "./chrome.mjs"

const widgetsFile = "browserdemo-src/demo/widgets/Widgets.scala"
const original = readFileSync(widgetsFile, "utf-8")
const mode = process.argv[2]
if (mode !== "sbt" && mode !== "vite" && mode !== "watch") throw new Error("usage: node measure-loop.mjs vite|sbt|watch")

const label = (n) => `Hello from teq (${n})`
// bin/sbt keeps the value of a `print` the last line of sbt 2's output, as the stock plugin reads it.
const sbtReal = spawnSync("which", ["sbt"], { encoding: "utf-8" }).stdout.trim()
const env = { ...process.env, TEQ_COMPILER: "1", SBT_REAL: sbtReal, PATH: `${process.cwd()}/bin:${process.env.PATH}` }
let vite
let sbtChild
const exited = (child) => child.exitCode !== null || child.signalCode !== null
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => { writeFileSync(widgetsFile, original); vite?.kill(); sbtChild?.kill(); process.exit(1) })

/** Runs sbt to completion, with its stdout and its wall time. */
function runSbt(args) {
  const start = Date.now()
  return new Promise((resolve, reject) => {
    const child = spawn("sbt", args, { stdio: ["ignore", "pipe", "inherit"], env })
    sbtChild = child
    let stdout = ""
    child.stdout.setEncoding("utf-8")
    child.stdout.on("data", (d) => (stdout += d))
    child.on("error", reject)
    child.on("close", (code) => resolve({ code, stdout, ms: Date.now() - start }))
  })
}
const editN = (n) => original.replace("Hello from teq", label(n))

function waitForDomText(page, text, timeoutMs = 15000) {
  const start = Date.now()
  return new Promise((resolve, reject) => {
    const poll = async () => {
      const h1 = await page.$eval("#root h1", (el) => el.textContent).catch(() => undefined)
      if (h1 === text) return resolve(Date.now())
      if (Date.now() - start > timeoutMs) return reject(new Error(`timed out waiting for "${text}", last saw ${JSON.stringify(h1)}`))
      setTimeout(poll, 20)
    }
    poll()
  })
}

async function withBrowserPage(url) {
  const browser = await puppeteer.launch({ executablePath: chromePath, headless: true, args: ["--no-sandbox", "--disable-gpu"] })
  const page = await browser.newPage()
  await page.goto(url, { waitUntil: "networkidle0" })
  await new Promise((r) => setTimeout(r, 2000)) // let vite's dependency optimizer settle once
  return { browser, page }
}

function report(results, label) {
  console.log(`\n${label}:`)
  for (const [i, r] of results.entries()) console.log(`  edit ${i + 1}: ${r.domMs} ms to the DOM${r.extra ?? ""}`)
  const avg = (key) => results.reduce((s, r) => s + r[key], 0) / results.length
  console.log(`  average: ${avg("domMs").toFixed(1)} ms to the DOM`)
  if (results[0].sbtMs !== undefined) console.log(`  average sbt trigger (sbt --batch cost - teq's own build time): ${avg("sbtOverheadMs").toFixed(1)} ms`)
}

async function measureVite() {
  const port = 5395
  vite = spawn("./node_modules/.bin/vite", ["--config", "vite.config.browserdemo.js", "--port", String(port), "--strictPort", "--host", "127.0.0.1"])
  try {
    for (let i = 0; i < 120; i++) {
      if (exited(vite)) throw new Error(`vite exited (${vite.exitCode ?? vite.signalCode})`)
      try { if ((await fetch(`http://127.0.0.1:${port}/`)).ok) break } catch {}
      await new Promise((r) => setTimeout(r, 500))
    }
    const { browser, page } = await withBrowserPage(`http://127.0.0.1:${port}/`)
    try {
      const results = []
      for (let i = 1; i <= 5; i++) {
        if (i > 1) await new Promise((r) => setTimeout(r, 800)) // a pause between edits, as a developer would take
        const t0 = Date.now()
        writeFileSync(widgetsFile, editN(i))
        const at = await waitForDomText(page, label(i))
        results.push({ domMs: at - t0 })
      }
      report(results, "vite-plugin-teq (its own resident teq compiler watch)")
    } finally {
      await browser.close()
    }
  } finally {
    vite.kill()
    writeFileSync(widgetsFile, original)
  }
}

// `sbt --batch browserdemo/fastLinkJS` once per edit, against the server and resident `teq compiler watch`
// the run before it warmed (sbt 2's launcher is a thin client of a server that stays up): sbt's
// own task-graph evaluation apart from teq's reported build time, plus a client start that `~`'s
// own trigger (`watch` below) does not pay.
async function measureSbt() {
  const port = 5396
  vite = spawn("./node_modules/.bin/vite", ["--config", "vite.scalajs.config.js", "--port", String(port), "--strictPort", "--host", "127.0.0.1"], { env })
  try {
    for (let i = 0; i < 120; i++) {
      if (exited(vite)) throw new Error(`vite exited (${vite.exitCode ?? vite.signalCode})`)
      try { if ((await fetch(`http://127.0.0.1:${port}/`)).ok) break } catch {}
      await new Promise((r) => setTimeout(r, 500))
    }
    const { browser, page } = await withBrowserPage(`http://127.0.0.1:${port}/`)
    try {
      const results = []
      for (let i = 1; i <= 5; i++) {
        if (i > 1) await new Promise((r) => setTimeout(r, 800))
        const t0 = Date.now()
        writeFileSync(widgetsFile, editN(i))
        // the DOM is watched while sbt runs: the update lands before the client exits
        const domAt = waitForDomText(page, label(i)).catch((e) => e)
        const build = await runSbt(["-Dsbt.supershell=false", "--batch", "browserdemo/fastLinkJS"])
        const at = await domAt
        if (at instanceof Error) throw at
        const m = build.stdout.match(/teq: built in ([\d.]+)ms/)
        const teqMs = m ? Number(m[1]) : undefined
        results.push({ domMs: at - t0, sbtMs: build.ms, sbtOverheadMs: teqMs === undefined ? undefined : build.ms - teqMs, teqMs })
      }
      report(results, "sbt --batch fastLinkJS (through the stock Scala.js vite plugin)")
    } finally {
      await browser.close()
    }
  } finally {
    vite.kill()
    writeFileSync(widgetsFile, original)
  }
}

async function measureWatch() {
  const port = process.env.VITE_SCALAJS_PORT ?? "5393"
  const { browser, page } = await withBrowserPage(`http://127.0.0.1:${port}/`)
  try {
    const results = []
    for (let i = 1; i <= 5; i++) {
      if (i > 1) await new Promise((r) => setTimeout(r, 800))
      const t0 = Date.now()
      writeFileSync(widgetsFile, editN(i))
      const at = await waitForDomText(page, label(i))
      results.push({ domMs: at - t0 })
    }
    report(results, "sbt ~fastLinkJS (through the stock Scala.js vite plugin)")
  } finally {
    await browser.close()
    writeFileSync(widgetsFile, original)
  }
}

if (mode === "vite") await measureVite()
else if (mode === "sbt") await measureSbt()
else await measureWatch()
