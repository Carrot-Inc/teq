// The dev loop with no sbt alive: `teq dev browserdemo` over the export, teq.lock
// (src/task/dev.rs), which runs the project's generator, the install, and the export's dev
// command, vite with vite-plugin-teq (vite.config.browserdemo.js) running `teq watch
// browserdemo`; the page under headless Chrome. A check that fails prints why as a line that
// starts with FAIL, and the run exits 1. It has 300 s, and every part in it a bound of its own or
// what is left, whichever is less; out of its time it prints a line that starts with TIMEOUT
// and exits 3, a verdict on the machine's load and none on the behaviour.
// - The page is served, and no process of the loop is a JVM; the time from the command to the
//   page's heading is printed.
// - A body edit reaches the DOM without a reload, the hook state kept; its time is printed.
// - An edit of the generator's input has the loop run the generator, and the label reaches the
//   DOM.
// - A change of the export's description of the project restarts the dev command, and the page
//   comes back; one of another project's block does not.
// - The binary's file changing restarts the dev command.
// - The export pinning another compiler ends the loop with code 75, its processes gone.
// - SIGINT ends the loop with code 130, its processes gone; under a terminal (python's pty), Ctrl-C
//   reaches the dev command, which holds the terminal, and ends the loop with it, and so does
//   vite's own `q`, which the dev command reads only while it holds the terminal.
// Needs TEQ (the binary), npm and Chrome (CHROME overrides its path); TASK_DEV_PORT the page's
// port (default 5399).
import { spawn, spawnSync } from "node:child_process"
import { copyFileSync, mkdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs"
import { resolve } from "node:path"
import puppeteer from "puppeteer-core"
import { parse } from "../../vite/lock.js"
import { chromePath } from "./chrome.mjs"

if (!process.env.TEQ) throw new Error("TEQ names the teq binary the loop runs")
const port = process.env.TASK_DEV_PORT ?? "5399"
const widgetsFile = "browserdemo-src/demo/widgets/Widgets.scala"
const labelsFile = "browserdemo-labels.txt"
const exportFile = "teq.lock"
const widgets = readFileSync(widgetsFile, "utf-8")
const labels = readFileSync(labelsFile, "utf-8")
const exported = readFileSync(exportFile, "utf-8")
const greeting = (text) => widgets.replace("Hello from teq", text)
if (greeting("x") === widgets) throw new Error(`${widgetsFile}: "Hello from teq" not found, cannot edit it`)
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
/** A tree as a teq.lock, every key and string quoted, which the lock's subset allows. */
function lockText(tree) {
  const q = (s) => JSON.stringify(s)
  const nested = (v) => v !== null && typeof v === "object" && Object.keys(v).length > 0
  const scalar = (v) => (typeof v === "string" ? q(v) : Array.isArray(v) ? "[]" : v !== null && typeof v === "object" ? "{}" : String(v))
  const lines = (v, depth) => {
    const pad = "  ".repeat(depth)
    const out = []
    for (const [k, x] of Array.isArray(v) ? v.map((x) => [undefined, x]) : Object.entries(v)) {
      const head = k === undefined ? `${pad}-` : `${pad}${q(k)}:`
      if (!nested(x)) out.push(`${head} ${scalar(x)}`)
      else if (k !== undefined) out.push(head, ...lines(x, depth + 1))
      else {
        const inner = lines(x, depth + 1)
        out.push(`${pad}- ${inner[0].slice(pad.length + 2)}`, ...inner.slice(1))
      }
    }
    return out
  }
  return lines(tree, 0).join("\n") + "\n"
}

const LIMIT = 300_000
const started = Date.now()
const left = () => LIMIT - (Date.now() - started)
class OutOfTime extends Error {}
function within(bound, what) {
  if (left() <= 0) throw new OutOfTime(what)
  return Math.min(bound, left())
}

let failed = 0
let passed = 0
const ok = (what) => { passed++; console.log(`task dev: ${what}`) }
function fail(what) {
  console.error(`FAIL task dev: ${what}`)
  failed++
  process.exitCode = 1
}
const check = (condition, what, found) => (condition ? ok(what) : fail(`${what}${found === undefined ? "" : `: found ${JSON.stringify(found)}`}`))

// A copy of the binary, whose modification time the check moves.
const scratch = resolve("target-task-dev")
rmSync(scratch, { recursive: true, force: true })
mkdirSync(scratch, { recursive: true })
const teq = resolve(scratch, "teq")
copyFileSync(process.env.TEQ, teq)

const loops = []
function restore() {
  writeFileSync(widgetsFile, widgets)
  writeFileSync(labelsFile, labels)
  writeFileSync(exportFile, exported)
}
function stop() {
  restore()
  for (const loop of loops) if (loop.exitCode === null && loop.signalCode === null) loop.kill("SIGINT")
}
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => { stop(); process.exit(1) })

/** `teq dev browserdemo` on the check's port, its output kept; under a terminal of python's
  * `pty.spawn`, which copies what the check writes to its input into the terminal. */
function startLoop({ terminal = false } = {}) {
  const command = [teq, "dev", "browserdemo", "--", "--port", port, "--strictPort", "--host", "127.0.0.1"]
  const [program, args] = terminal ? ["python3", ["-c", "import pty, sys; sys.exit(pty.spawn(sys.argv[1:]) >> 8)", ...command]] : [command[0], command.slice(1)]
  const loop = spawn(program, args, { stdio: ["pipe", "pipe", "pipe"], detached: true })
  loops.push(loop)
  loop.log = ""
  loop.stdout.on("data", (d) => (loop.log += d))
  loop.stderr.on("data", (d) => (loop.log += d))
  loop.ended = new Promise((resolve) => loop.on("exit", (code, signal) => resolve(code ?? signal)))
  return loop
}

async function until(what, condition, ms = 15_000, step = 100) {
  const end = Date.now() + within(ms, what)
  for (;;) {
    const value = await condition()
    if (value) return value
    if (Date.now() > end) return void within(0, what)
    await sleep(step)
  }
}

/** The processes whose command line names the copy of the binary, with their commands. */
function processes() {
  const ps = spawnSync("ps", ["-eo", "pid=,comm=,args="], { encoding: "utf-8" }).stdout
  return ps.split("\n").filter((line) => line.includes(teq) || line.includes(`--port ${port} `))
}
/** The processes of the loop's tree: the dev command's group, which leads with its own pid. */
function tree() {
  const ps = spawnSync("ps", ["-eo", "pid=,ppid=,pgid=,comm="], { encoding: "utf-8" }).stdout
  const rows = ps.split("\n").map((l) => l.trim().split(/\s+/)).filter((r) => r.length >= 4).map(([pid, ppid, pgid, ...comm]) => ({ pid: Number(pid), ppid: Number(ppid), pgid: Number(pgid), comm: comm.join(" ") }))
  return (root) => {
    const out = []
    const queue = [root]
    while (queue.length) {
      const pid = queue.pop()
      for (const row of rows) if (row.ppid === pid) { out.push(row); queue.push(row.pid) }
    }
    return out
  }
}
/** Of the processes of a tree taken before its end, those still running (a zombie is over). */
function survivors(rows) {
  if (rows.length === 0) return []
  const ps = spawnSync("ps", ["-o", "pid=,stat=", "-p", rows.map((r) => r.pid).join(",")], { encoding: "utf-8" }).stdout
  const running = new Set(ps.split("\n").map((l) => l.trim().split(/\s+/)).filter(([, stat]) => stat && !stat.startsWith("Z")).map(([pid]) => Number(pid)))
  return rows.filter((r) => running.has(r.pid))
}
const portFree = async () => {
  try {
    await fetch(`http://127.0.0.1:${port}/`, { signal: AbortSignal.timeout(1000) })
    return false
  } catch {
    return true
  }
}
async function served() {
  try {
    return (await fetch(`http://127.0.0.1:${port}/`, { signal: AbortSignal.timeout(2000) })).ok
  } catch {
    return false
  }
}

async function open(browser) {
  const page = await browser.newPage()
  page.loads = 0
  page.errors = []
  page.on("pageerror", (e) => page.errors.push(e.message))
  page.on("framenavigated", (frame) => frame === page.mainFrame() && page.loads++)
  page.text = (selector) => page.$eval(selector, (el) => el.textContent).catch(() => undefined)
  page.shows = (selector, text, ms) => until(text, async () => (await page.text(selector)) === text, ms, 20)
  return page
}
async function save(file, text) {
  writeFileSync(file, text)
  await sleep(50)
}

const browser = await puppeteer.launch({ executablePath: chromePath, headless: true, args: ["--no-sandbox"] })
try {
  // The command to the page.
  const t0 = Date.now()
  let loop = startLoop()
  const up = await until("the page served", served, 180_000, 50)
  if (!up) throw new Error(`the page was not served:\n${loop.log}`)
  let page = await open(browser)
  await page.goto(`http://127.0.0.1:${port}/`)
  const shown = await page.shows("#root h1", "Hello from teq", 60_000)
  check(shown, `the page shows its heading, ${Date.now() - t0} ms from teq dev`, await page.text("#root h1"))
  const members = tree()(loop.pid)
  check(members.length > 0 && !members.some((p) => /java/.test(p.comm)), "the loop's processes hold no JVM", members.map((p) => p.comm))

  // An edit of a body reaches the DOM, the hook state kept.
  for (let i = 0; i < 3; i++) await page.click("#counter")
  const loads = page.loads
  const t1 = Date.now()
  await save(widgetsFile, greeting("Hello from the loop"))
  const reached = await page.shows("#root h1", "Hello from the loop", 30_000)
  const ms = Date.now() - t1
  check(reached && page.loads === loads && (await page.text("#counter")) === "count: 3", `an edit reaches the DOM in ${ms} ms, without a reload and with the count kept`, [await page.text("#root h1"), page.loads - loads, await page.text("#counter")])
  await save(widgetsFile, widgets)
  await page.shows("#root h1", "Hello from teq", 30_000)

  // The generator's input.
  await save(labelsFile, "made by the generator\n")
  check(await page.shows("#footer", "made by the generator", 30_000), "an edit of the generator's input reaches the DOM", await page.text("#footer"))
  await save(labelsFile, labels)
  await page.shows("#footer", labels.trim(), 30_000)

  // The export: another project's block changes nothing, the project's restarts the dev command.
  const restarts = () => (loop.log.match(/: restarting /g) ?? []).length
  const restartLines = () => loop.log.split("\n").filter((line) => line.includes(": restarting "))
  const rewrite = (edit) => {
    const copy = parse(exported)
    edit(copy)
    writeFileSync(exportFile, lockText(copy))
  }
  rewrite((e) => (e.projects.api.description.keys = { unrelated: "1" }))
  await sleep(2500)
  check(restarts() === 0, "a change to another project's block leaves the dev command running", loop.log.slice(-400))
  const comesBack = async (before) => (await served()) && page.loads > before && (await page.text("#root h1")) === "Hello from teq"
  let before = page.loads
  rewrite((e) => (e.projects.browserdemo.description.keys = { hotSwapUpTo: "50" }))
  check(await until("restart", () => restarts() === 1, 10_000), "a change to the project's block restarts the dev command", restartLines())
  check(await until("the page back", () => comesBack(before), 60_000), "and the page comes back", [await page.text("#root h1"), page.loads - before])
  before = page.loads
  writeFileSync(exportFile, exported)
  check(await until("restart", () => restarts() === 2, 10_000), "the project's block restored restarts it again", restartLines())
  await until("the page back", () => comesBack(before), 60_000)

  // The binary's file.
  const now = new Date()
  utimesSync(teq, now, now)
  before = page.loads
  check(await until("restart", () => restarts() === 3, 10_000), "the binary's file changing restarts the dev command", restartLines())
  await until("the page back", () => comesBack(before), 60_000)

  // Another compiler pinned: the loop ends, with its processes.
  const ending = tree()(loop.pid)
  rewrite((e) => (e.teq = `${e.teq}-other`))
  const code = await Promise.race([loop.ended, sleep(within(15_000, "the loop's end")).then(() => "running")])
  check(code === 75 && /pins another compiler/.test(loop.log), "the export pinning another compiler ends the loop with code 75", [code, loop.log.slice(-300)])
  writeFileSync(exportFile, exported)
  const gone = await until("gone", async () => survivors(ending).length === 0 && processes().length === 0 && (await portFree()), 10_000)
  check(gone && ending.length > 0, `and its ${ending.length} processes are gone, the port free`, [survivors(ending), processes()])

  // SIGINT, then Ctrl-C under a terminal.
  for (const [terminal, key] of [[false, undefined], [true, "\x03"], [true, "q\n"]]) {
    loop = startLoop({ terminal })
    const how = key === "q\n" ? "vite's q under a terminal" : terminal ? "Ctrl-C under a terminal" : "SIGINT"
    if (!(await until("the page served", served, 120_000, 50))) {
      fail(`${how}: the page was not served: ${loop.log.slice(-300)}`)
      continue
    }
    const ending = tree()(loop.pid)
    if (terminal) loop.stdin.write(key)
    else loop.kill("SIGINT")
    const code = await Promise.race([loop.ended, sleep(within(15_000, "the loop's end")).then(() => "running")])
    const ends = key === "q\n" ? `${how} reaches vite, which holds the terminal, and its end ends the loop with code 0` : `${how} ends the loop${terminal ? "" : " with code 130"}`
    check(key === "q\n" ? code === 0 : terminal ? code !== "running" : code === 130, ends, [code, loop.log.slice(-300)])
    const gone = await until("gone", async () => survivors(ending).length === 0 && processes().length === 0 && (await portFree()), 10_000)
    check(gone && ending.length > 0, `and its ${ending.length} processes are gone after ${how}, the port free`, [survivors(ending), processes()])
  }
  await page.close()
} catch (err) {
  if (!(err instanceof OutOfTime)) throw err
  console.error(`TIMEOUT task dev: out of its ${LIMIT / 1000} s at ${err.message}`)
  process.exitCode = 3
} finally {
  stop()
  await browser.close()
  await sleep(500)
  rmSync(scratch, { recursive: true, force: true })
}
console.log(`task dev: ${passed} passed, ${failed} failed`)
process.exit(process.exitCode ?? 0)
