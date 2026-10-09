// The page of browserdemo-src/ under headless Chrome through sbt's own link tasks, with
// TEQ_COMPILER=1 in sbt's environment so that sbt-teq answers them from teq's output, written
// where the linker would write its own. Three modes, each a process of its own. A check that
// fails prints why as a line that starts with FAIL, and the mode exits 1.
//
// A mode has 300 s (5 to 10 times what it takes on an idle machine), and every part in it a
// bound of its own or what is left of the mode's time, whichever is less. A mode that runs out
// of its time prints a line that starts with TIMEOUT and exits 3: that is a verdict on the
// machine's load and none on the behaviour, which a part that fails inside its own bound is.
//
// `node check-scalajs-dev.mjs stock`: the stock Scala.js vite plugin over `browserdemo`
// (vite.scalajs.config.js), a link per edit by a batch client.
// - The refresh runtime is in place ahead of React DOM on a first load with nothing pre-bundled
//   and after the optimizer's reload.
// - A body edit of a widget reaches the DOM without a reload, the hook state kept and the
//   `Theme` held in it still matching, with the modules of the chain run again and no others;
//   a module that was not run again calls the swapped one's new code; two files are swapped in
//   turn; a page load after the swaps fetches every module under one address, and a swap after
//   it works.
// - An edit of a per-file module whose class shared code tests reloads the page; a value of an
//   enum of the swapped file itself, held in hook state, matches nothing after the swap
//   (`MatchError`, which the check pins as what happens today).
// - The files of a build published one by one, a shared module before the per-file module it
//   takes a renamed member of, and two shared modules one after the other, leave the page as
//   it is until the listing is published, which loads it again on the whole build; main.mjs
//   published before the shared module it takes a new export of fails to link, which the
//   stub tells the runtime, and the listing loads the page again without main's say. Nothing
//   is pending at the end of any scenario, and the page's listing is the directory's.
// - An edit of a module that is not per file reloads the page.
// - Two links of teq's into one directory in one command leave one build, whole.
// - `vite build` bundles the one file of fullLinkJS, without the runtime.
//
// `node check-scalajs-dev.mjs toggle`: the same project and plugin, with the Scala.js linker.
// - The toggle turned off leaves the linker's files alone in the directory, an edit then
//   reloads the page, and the toggle turned on again leaves teq's alone, with a swap after it.
// - A link of teq's and the linker's into one directory in one command leave one writer's
//   build, whole, under that writer's marker.
// - The linker's turn is over with a command that is cancelled, and a second sbt process links
//   into the directory after it.
//
// `node check-scalajs-dev.mjs mirrored`: the build shaped like the reference application's
// (`mirrored`, vite.mirrored.config.js) under `sbt ~mirrored/fastLinkJS`.
// - The same swaps through the watch and the mirror; an edit of a generator's input
//   retriggers; a type error leaves the page as it was, waiting for nothing, and its fix lands.
// - A directory removed is written again by a link that changed nothing; a second sbt process
//   is refused while the first writes the directory.
// - The resident ends with the watch, the server staying, and an idle one outside a watch
//   after `teqLinkIdle`; a watch of another task that links holds the resident while it lasts
//   and no longer.
// - `vite build` through the mirror.
import { spawn, spawnSync } from "node:child_process"
import { copyFileSync, cpSync, existsSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs"
import { join, resolve } from "node:path"
import puppeteer from "puppeteer-core"
import { chromePath } from "./chrome.mjs"

const mode = process.argv[2] ?? "stock"
const PORTS = { stock: 0, mirrored: 2, toggle: 4 }
if (!(mode in PORTS)) throw new Error("usage: node check-scalajs-dev.mjs stock|toggle|mirrored")
const project = mode === "mirrored" ? "mirrored" : "browserdemo"
const config = mode === "mirrored" ? "vite.mirrored.config.js" : "vite.scalajs.config.js"
const dist = mode === "mirrored" ? "dist-mirrored" : "dist-scalajs"
const port = String(Number(process.env.VITE_SCALAJS_PORT ?? "5393") + PORTS[mode])
const directory = mode === "mirrored" ? "mirrored/target/page-fastopt" : "target/out/sjs1/scala-3.8.4/browserdemo/browserdemo-fastopt"
const served = mode === "mirrored" ? `${directory}-staged` : directory
const widgetsFile = "browserdemo-src/demo/widgets/Widgets.scala"
const badgeFile = "browserdemo-src/demo/widgets/Badge.scala"
const chipFile = "browserdemo-src/demo/widgets/Chip.scala"
const themeFile = "browserdemo-src/demo/model/Theme.scala"
const mainFile = "browserdemo-src/demo/Main.scala"
// What teq writes for the page: five modules of the program, main.mjs, rt.mjs, std.mjs,
// hot-refresh.mjs and the listing, hot-build.mjs; sbt-teq's stub main.js beside them.
const MODULES = 10
const labelsFile = "browserdemo-labels.txt"
const widgets = readFileSync(widgetsFile, "utf-8")
const badge = readFileSync(badgeFile, "utf-8")
const chip = readFileSync(chipFile, "utf-8")
const labels = readFileSync(labelsFile, "utf-8")
const greeting = (text) => widgets.replace("Hello from teq", text)
if (greeting("x") === widgets) throw new Error(`${widgetsFile}: "Hello from teq" not found, cannot edit it`)
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

const LIMIT = 300_000
const started = Date.now()
const left = () => LIMIT - (Date.now() - started)
class OutOfTime extends Error {}
/** A part's bound: its own, or what is left of the mode's time where that is less. */
function within(bound, what) {
  if (left() <= 0) throw new OutOfTime(what)
  return Math.min(bound, left())
}

let failed = 0
let passed = 0
const ok = (what) => { passed++; console.log(`${mode}: ${what}`) }
function fail(what) {
  console.error(`FAIL ${mode}: ${what}`)
  failed++
  process.exitCode = 1
}
const check = (condition, what, found) => (condition ? ok(what) : fail(`${what}${found === undefined ? "" : `: found ${JSON.stringify(found)}`}`))

// bin/sbt keeps the value of a `print` the last line of sbt 2's output, as the stock plugin reads it.
const sbtReal = spawnSync("which", ["sbt"], { encoding: "utf-8" }).stdout.trim()
const env = { ...process.env, TEQ_COMPILER: "1", SBT_REAL: sbtReal, PATH: `${process.cwd()}/bin:${process.env.PATH}` }
const children = []
function restore() {
  writeFileSync(widgetsFile, widgets)
  writeFileSync(badgeFile, badge)
  writeFileSync(chipFile, chip)
  writeFileSync(labelsFile, labels)
}
function stop() {
  restore()
  for (const child of children) child.kill()
}
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => { stop(); process.exit(1) })
const exited = (child) => child.exitCode !== null || child.signalCode !== null

/** One sbt command by a batch client of the session's server, with its output. */
function sbt(command, timeout = 200_000) {
  const run = spawnSync(sbtReal, ["--batch", "-no-colors", "-Dsbt.supershell=false", command], { encoding: "utf-8", env, timeout: within(timeout, `sbt ${command}`) })
  if (run.error?.code === "ETIMEDOUT") within(0, `sbt ${command}`)
  return { ok: run.status === 0, out: `${run.stdout}\n${run.stderr}` }
}

/** The processes' commands as teq reads them, split on blanks, which the example's paths hold
  * none of: the plugin starts teq as `teq @<file>` (docs/TARGETS.md, "Argument files"), the file's
  * lines standing for the rest. */
function commands() {
  const ps = spawnSync("ps", ["-ww", "-eo", "pid=,args="], { encoding: "utf-8" }).stdout
  return ps.split("\n").filter((line) => line.trim()).map((line) => {
    const [pid, ...args] = line.trim().split(/\s+/)
    const lines = (file) => readFileSync(file, "utf-8").split("\n").slice(0, -1).map((l) => l.replace(/\r$/, ""))
    const read = (a) => {
      if (!a.startsWith("@") || !a.endsWith(".args")) return [a]
      try { return lines(a.slice(1)) } catch { return [a] }
    }
    return { pid: Number(pid), args: args.flatMap(read) }
  })
}
const writes = (args, of) => args[1] === "compiler" && args[2] === "watch" && args.some((a, i) => a === "--split" && args[i + 1] === of)

/** The `teq compiler watch` processes writing the directory, by their commands. */
function residents(of = directory) {
  return commands().filter(({ args }) => writes(args, of)).map(({ pid }) => pid)
}
const servers = () => spawnSync("ps", ["-eo", "pid=,args="], { encoding: "utf-8" }).stdout.split("\n").filter((line) => /java/.test(line) && /sbt-launch|xsbt\.boot|sbt\.script/.test(line)).map((line) => Number(line.trim().split(/\s+/)[0]))

async function until(what, condition, ms = 15_000, step = 100) {
  const end = Date.now() + within(ms, what)
  for (;;) {
    const value = await condition()
    if (value) return value
    if (Date.now() > end) return void within(0, what)
    await sleep(step)
  }
}

function startVite() {
  const vite = spawn("./node_modules/.bin/vite", ["--config", config, "--port", port, "--strictPort", "--host", "127.0.0.1"], { stdio: ["ignore", "pipe", "pipe"], env })
  children.push(vite)
  vite.log = ""
  vite.stdout.on("data", (d) => (vite.log += d))
  vite.stderr.on("data", (d) => (vite.log += d))
  return vite
}
async function waitForServer(vite, at = port) {
  for (const end = Date.now() + within(120_000, "vite's start"); Date.now() < end; ) {
    if (exited(vite)) throw new Error(`vite exited (${vite.exitCode ?? vite.signalCode}):\n${vite.log}`)
    try {
      if ((await fetch(`http://127.0.0.1:${at}/`)).ok) return
    } catch {}
    await sleep(500)
  }
  within(0, "vite's start")
  throw new Error(`vite did not come up on port ${at}:\n${vite.log}`)
}

/** The page with what the checks read of it: its loads, its errors, the modules it fetched anew. */
async function open(browser) {
  const page = await browser.newPage()
  page.loads = 0
  page.errors = []
  page.again = []
  page.on("pageerror", (e) => page.errors.push(e.message))
  page.fetched = new Map()
  page.on("framenavigated", (frame) => {
    if (frame !== page.mainFrame()) return
    page.loads++
    page.fetched = new Map()
  })
  page.on("request", (request) => {
    const [address, name] = /([^/?]+\.m?js)(\?[^/]*)?$/.exec(request.url()) ?? []
    if (name === undefined) return
    page.fetched.set(name, (page.fetched.get(name) ?? new Set()).add(address))
    if (/[?&]t=\d/.test(address)) page.again.push(name)
  })
  // The modules this load fetched under more than one address: each would have run twice.
  page.twice = () => [...page.fetched].filter(([, addresses]) => addresses.size > 1).map(([, addresses]) => [...addresses].join(" + "))
  page.text = (selector) => page.$eval(selector, (el) => el.textContent).catch(() => undefined)
  page.runtime = () => page.evaluate(() => (typeof __teqHotRefresh === "undefined" ? "absent" : `${__teqHotRefresh.renderers.size} renderer, ${__teqHotRefresh.mountedRoots.size} root`))
  page.shows = (selector, text, ms) => until(text, async () => (await page.text(selector)) === text, ms)
  // What the page waits for to be loaded again, and the listing it has (docs/TARGETS.md,
  // "Module splitting").
  page.pending = () => page.evaluate(() => globalThis.__teqHot?.pending()).catch(() => undefined)
  page.listing = () => page.evaluate(() => globalThis.__teqHot?.build).catch(() => undefined)
  return page
}

/** The end of a scenario: the page waits for nothing, and the listing it has is the one of the
  * directory it is served from. */
async function settled(page, scenario) {
  const pending = await page.pending()
  const onDisk = Object.fromEntries([...readFileSync(`${served}/hot-build.mjs`, "utf-8").matchAll(/^    "(.*)": \["(.*)", "(.*)"\],$/gm)].map(([, name, hash, writer]) => [name, [hash, writer]]))
  const listing = await until("listing", async () => JSON.stringify(await page.listing()) === JSON.stringify(onDisk), 5_000)
  check(pending === undefined && listing, `${scenario}: nothing is pending, and the page has the directory's listing`, [pending, !!listing])
}

const same = (a, b) => {
  const names = (dir) => (existsSync(dir) ? readdirSync(dir).sort() : [])
  return names(a).join() === names(b).join() && names(a).every((name) => readFileSync(join(a, name)).equals(readFileSync(join(b, name))))
}
const teqs = (dir) => readdirSync(dir).filter((name) => name.endsWith(".mjs")).length

/** The writer whose build the directory holds whole, under that writer's marker and with no
  * file of another's; `linkers` are the names of the Scala.js linker's files. */
function writer(linkers = "") {
  const names = existsSync(directory) ? readdirSync(directory).sort() : []
  const marker = existsSync(`${directory}.backend`) ? readFileSync(`${directory}.backend`, "utf-8").trim() : ""
  const main = names.includes("main.js") ? readFileSync(`${directory}/main.js`, "utf-8") : ""
  if (marker === "") return names.join() === linkers && main.includes("$c_O") ? "the Scala.js linker" : undefined
  if (!marker.startsWith("teq ") || main.includes("$c_O")) return undefined
  if (marker.endsWith(" full")) return names.join() === "main.js" && !main.includes("./main.mjs") ? "teq's full link" : undefined
  return teqs(directory) === MODULES && names.length === MODULES + 1 && main.includes("./main.mjs") ? "teq's dev link" : undefined
}

/** The state three clicks on the counter and two on the theme leave, and what a swap has to keep. */
async function click(page, theme = "theme: dark 2") {
  for (let i = 0; i < 3; i++) await page.click("#counter")
  for (let i = 0; i < 2; i++) await page.click("#theme")
  check((await page.text("#counter")) === "count: 3" && (await page.text("#theme")) === theme, `the clicks are in hook state (count: 3, ${theme})`, [await page.text("#counter"), await page.text("#theme")])
}
async function swapped(page, text, loads, { at = "#root h1", theme = "theme: dark 2", chain = "demo.widgets.Widgets.mjs,main.js,main.mjs" } = {}) {
  const reached = await page.shows(at, text)
  check(reached, `the edit reached the DOM (${text})`, await page.text(at))
  await sleep(300)
  check((await page.text("#counter")) === "count: 3", "the counter's hook state was kept", await page.text("#counter"))
  check((await page.text("#theme")) === theme, `the Theme held in hook state still matches (${theme})`, await page.text("#theme"))
  check(page.loads === loads, "no page reload", page.loads - loads)
  check(page.errors.length === 0, "no page error", page.errors.slice(0, 2))
  const again = [...new Set(page.again)].filter((name) => name !== "hot-build.mjs").sort()
  check(again.join() === chain, `the page ran the chain again and nothing else (${chain})`, again)
  await settled(page, "after the swap")
  page.again.length = 0
}

/** The command of the directory's writer, the sbt link's resident `teq compiler watch`, read
  * while it runs (a link has just answered), its argument file's lines in their place. */
function linkCommand() {
  const found = commands().find(({ args }) => writes(args, directory))
  if (!found) throw new Error(`no resident writes ${directory}`)
  return found.args
}

/** A build of other sources than the directory's, made by teq with the writer's own command line
  * over a copy of the directory (so that the modules the edit does not change keep their writer,
  * as under a session): the check publishes its files into the served directory one by one, as a build's renames do,
  * in the order it wants the page to meet them. An edit is `[file, from, to]`, or `[file,
  * text]` for a file added with that text. The sources are as they were when it returns. */
function buildOf(name, edits, command) {
  const before = edits.map(([file]) => [file, existsSync(file) ? readFileSync(file, "utf-8") : undefined])
  for (const [file, from, to] of edits) writeFileSync(file, to === undefined ? from : readFileSync(file, "utf-8").replaceAll(from, to))
  const out = resolve(`target-build-${name}`)
  rmSync(out, { recursive: true, force: true })
  cpSync(directory, out, { recursive: true })
  const split = command.indexOf("--split")
  const args = ["compiler", "build", ...command.slice(3, split), "--split", out, ...command.slice(split + 2)]
  const run = spawnSync(command[0], args, { encoding: "utf-8", timeout: within(60_000, `teq compiler build of ${name}`) })
  for (const [file, text] of before) if (text === undefined) rmSync(file, { force: true }); else writeFileSync(file, text)
  if (run.error?.code === "ETIMEDOUT") within(0, `teq compiler build of ${name}`)
  if (run.status !== 0) fail(`teq compiler build of ${name} exited ${run.status}:\n${run.stdout}${run.stderr}`)
  return out
}
function publish(from, ...names) {
  for (const name of names) {
    copyFileSync(join(from, name), join(directory, `${name}.tmp`))
    renameSync(join(directory, `${name}.tmp`), join(directory, name))
  }
}
const waits = async (page) => ((await page.pending())?.waits ?? []).map((w) => w.module).sort().join()

/** The order in which a build reaches the page. The page is loaded again when the listing has
  * the text of every module the page ran, and not before. */
async function interleaved(page) {
  const linked = sbt(`${project}/fastLinkJS`)
  if (!linked.ok) throw new Error(`sbt ${project}/fastLinkJS failed:\n${linked.out}`)
  const command = linkCommand()
  const first = resolve("target-build-first")
  rmSync(first, { recursive: true, force: true })
  cpSync(directory, first, { recursive: true })
  const back = async (...names) => {
    const loads = page.loads
    publish(first, ...names, "hot-build.mjs")
    await until("reload", () => page.loads > loads)
    await page.shows("#theme", "theme: light")
    page.errors.length = 0
  }

  // A shared module takes a member of a per-file module under another name: published and run
  // before the per-file module is, it would find nothing under the name.
  const renamed = buildOf("renamed", [[widgetsFile, "themePrefix", "themeLabel"], [widgetsFile, '"theme: "', '"look: "'], [themeFile, "themePrefix", "themeLabel"]], command)
  let loads = page.loads
  publish(renamed, "demo.model.mjs")
  check((await until("wish", async () => (await waits(page)) === "demo.model")) && page.loads === loads, "a shared module published before the per-file module it takes from leaves a wish, and the page as it is", [await page.pending(), page.loads - loads])
  publish(renamed, "demo.widgets.Widgets.mjs")
  check((await until("wish", async () => (await waits(page)) === "demo.model,demo.widgets.Widgets")) && page.loads === loads, "the per-file module published after it is swapped, and the wish stays", [await page.pending(), page.loads - loads])
  publish(renamed, "hot-build.mjs")
  check((await until("reload", () => page.loads === loads + 1)) && (await page.shows("#theme", "look: light")), "the listing, published last, loads the page again on the whole build", [page.loads - loads, await page.text("#theme")])
  page.errors.length = 0
  await settled(page, "after a build published consumer first")
  await back("demo.model.mjs", "demo.widgets.Widgets.mjs")
  await settled(page, "after the build of before published the same way")

  // Two shared modules of one build: the first runs while the second has not landed.
  const two = buildOf("two", [[themeFile, '"light"', '"bright"'], [mainFile, '"one"', '"two"']], command)
  loads = page.loads
  publish(two, "demo.model.mjs")
  check((await until("wish", async () => (await waits(page)) === "demo.model")) && page.loads === loads, "the first of two shared modules of a build leaves a wish, and the page as it is", [await page.pending(), page.loads - loads])
  publish(two, "demo.mjs")
  check((await until("wish", async () => (await waits(page)) === "demo,demo.model")) && page.loads === loads, "the second joins it", [await page.pending(), page.loads - loads])
  publish(two, "hot-build.mjs")
  check((await until("reload", () => page.loads === loads + 1)) && (await page.shows("#theme", "theme: bright")) && (await page.evaluate(() => window.__demoStamp)) === "two", "the listing loads the page again on both", [page.loads - loads, await page.text("#theme")])
  check(page.errors.length === 0, "with no page error", page.errors.slice(0, 2))
  await settled(page, "after a build of two shared modules")
  await back("demo.model.mjs", "demo.mjs")
  await settled(page, "after the build of before")

  // main.mjs published before the shared module it takes a new export of (an enum in Main.scala's
  // package, in a file after every other so that no class number shifts, gives demo.mjs an
  // `$enums`, which main.mjs imports): the swap fails to link, the stub tells the runtime, and
  // the page is loaded again once the listing is there.
  const needs = buildOf("needs", [[mainFile, '"one"', "Stamp.Two.toString"], ["browserdemo-src/demo/zed.scala", "package demo\n\nenum Stamp:\n  case Two\n"]], command)
  check(readdirSync(needs).filter((name) => !readFileSync(join(needs, name)).equals(readFileSync(join(directory, name)))).sort().join() === "demo.mjs,hot-build.mjs,main.mjs", "the build rewrote main.mjs, demo.mjs and the listing, and no other module", readdirSync(needs).filter((name) => !readFileSync(join(needs, name)).equals(readFileSync(join(directory, name)))))
  loads = page.loads
  publish(needs, "main.mjs")
  const failure = await until("failure", async () => { const p = await page.pending(); return p?.failed?.join() === "main" ? p : undefined })
  check(failure !== undefined && page.loads === loads && failure.wish === "main's update failed" && failure.waits.length === 0, "main published before the shared module it needs: its update fails, the page keeps what it has and waits for nothing of main's", [failure, page.loads - loads])
  publish(needs, "demo.mjs")
  check((await until("wish", async () => (await page.pending())?.waits.some((w) => w.module === "demo"))) && page.loads === loads, "the shared module's update runs, and the page stays as it is", [await page.pending(), page.loads - loads])
  publish(needs, "hot-build.mjs")
  check((await until("reload", () => page.loads === loads + 1)) && (await until("stamp", async () => (await page.evaluate(() => window.__demoStamp)) === "Two")), "the listing loads the page again on the whole build, without main's say", [page.loads - loads, await page.evaluate(() => window.__demoStamp)])
  check(page.errors.length === 0, "with no page error", page.errors.slice(0, 2))
  await settled(page, "after the build published main first")
  await back("main.mjs", "demo.mjs")
  await settled(page, "after the build of before")
  for (const name of ["first", "renamed", "two", "needs"]) rmSync(resolve(`target-build-${name}`), { recursive: true, force: true })
  await click(page)
  page.again.length = 0
  return page.loads
}

/** What cannot be swapped. An edit of Chip.scala, whose class shared code tests, reloads the
  * page. A value of `Size`, an enum of Widgets.scala itself, held in hook state: after a swap of
  * that file the per-file code run again matches it against the enum the swap made, and the
  * match fails. That is what happens today, pinned here; the page is loaded anew after it. */
async function held(page, save) {
  let loads = page.loads
  await save(chipFile, chip.replace("chip: one", "chip: two"))
  check(await until("reload", () => page.loads > loads) && (await page.shows("#chip", "chip: two")), "an edit of a per-file module whose class shared code tests reloads the page", [page.loads - loads, await page.text("#chip")])
  check((await page.text("#counter")) === "count: 0" && page.errors.length === 0, "with the page's state gone and no page error", [await page.text("#counter"), page.errors[0]])
  await settled(page, "after the reload")
  await save(chipFile, chip)
  await page.shows("#chip", "chip: one")
  loads = page.loads
  await page.click("#size")
  check((await page.text("#size")) === "size: large", "a value of the file's own enum is in hook state (size: large)", await page.text("#size"))
  await save(widgetsFile, greeting("Hello from the hot swap").replace("Hello from the hot swap", "Hello from the own enum"))
  const failed = await until("error", () => page.errors.some((e) => e.includes("MatchError")), 10_000)
  check(failed && page.loads === loads, "after a swap of its file the held value matches no case of the enum the swap made: MatchError, no reload", [page.errors.slice(0, 2), page.loads - loads])
  page.errors.length = 0
  await save(widgetsFile, greeting("Hello from the hot swap"))
  await page.reload({ waitUntil: "networkidle0" })
  check((await page.shows("#root h1", "Hello from the hot swap")) && (await page.text("#size")) === "size: none" && page.errors.length === 0, "a page load after it runs the page again", [await page.text("#root h1"), await page.text("#size"), page.errors[0]])
  await settled(page, "after the page load")
  await click(page)
  page.again.length = 0
  return page.loads
}

/** Two files swapped in turn, a module that was not run again calling the new code, and a page
  * load after the swaps: `save` writes a file and has it linked. */
async function turns(page, save, loads) {
  await save(badgeFile, badge.replace("badge: plain", "badge: swapped"))
  await swapped(page, "badge: swapped", loads, { at: "#badge", chain: "demo.widgets.Badge.mjs,demo.widgets.Widgets.mjs,main.js,main.mjs" })
  // demo.model's Themes.label, which the theme's button shows, reads Widgets.themePrefix.
  await save(widgetsFile, greeting("Hello from the hot swap").replace('"theme: "', '"mode: "'))
  await swapped(page, "mode: dark 2", loads, { at: "#theme", theme: "mode: dark 2" })
  check(true, "a module that was not run again (demo.model) calls the swapped module's new code")
  await page.reload({ waitUntil: "networkidle0" })
  check((await page.shows("#badge", "badge: swapped")) && (await page.text("#theme")) === "mode: light", "a page load after the swaps shows what they made", [await page.text("#badge"), await page.text("#theme")])
  check(page.twice().length === 0 && page.fetched.size >= MODULES, `and fetched each of its ${page.fetched.size} modules under one address`, page.twice())
  check((await page.runtime()) === "1 renderer, 1 root" && page.errors.length === 0, "with the refresh runtime in place and no page error", [await page.runtime(), page.errors[0]])
  await settled(page, "after the page load")
  await click(page, "mode: dark 2")
  page.again.length = 0
  await save(badgeFile, badge)
  await swapped(page, "badge: plain", page.loads, { at: "#badge", theme: "mode: dark 2", chain: "demo.widgets.Badge.mjs,demo.widgets.Widgets.mjs,main.js,main.mjs" })
  await save(widgetsFile, greeting("Hello from the hot swap"))
  await swapped(page, "theme: dark 2", page.loads, { at: "#theme" })
  return page.loads
}

async function bundle(browser) {
  rmSync(dist, { recursive: true, force: true })
  const build = spawnSync("./node_modules/.bin/vite", ["build", "--config", config], { encoding: "utf-8", env, timeout: within(200_000, "vite build") })
  if (build.error?.code === "ETIMEDOUT") within(0, "vite build")
  if (build.status !== 0) return fail(`vite build --config ${config} exited ${build.status}:\n${build.stdout}\n${build.stderr}`)
  const full = directory.replace("-fastopt", "-opt")
  check(existsSync(`${full}/main.js`) && teqs(full) === 0, "fullLinkJS wrote one file, main.js", readdirSync(full))
  if (mode === "mirrored") check(same(full, `${full}-staged`), "the mirror of the full link holds the directory's file")
  const assets = readdirSync(`${dist}/assets`).filter((name) => name.endsWith(".js")).map((name) => readFileSync(`${dist}/assets/${name}`, "utf-8")).join("\n")
  check(assets.includes("Hello from teq") && !/__teqHotRefresh|teqHotObject|hot-refresh/.test(assets), "the bundle holds the program and nothing of the refresh runtime")
  const at = String(Number(port) + 1)
  const preview = spawn("./node_modules/.bin/vite", ["preview", "--config", config, "--port", at, "--strictPort", "--host", "127.0.0.1"], { stdio: ["ignore", "pipe", "pipe"], env })
  children.push(preview)
  preview.log = ""
  try {
    await waitForServer(preview, at)
    const page = await open(browser)
    await page.goto(`http://127.0.0.1:${at}/`, { waitUntil: "networkidle0" })
    check((await page.text("#root h1")) === "Hello from teq" && page.errors.length === 0, "vite build: the bundle renders the page", [await page.text("#root h1"), page.errors[0]])
    await page.close()
  } finally {
    preview.kill()
  }
}

async function stock(browser) {
  // Nothing pre-bundled: the first load finds react and react-dom, and vite reloads the page
  // once it has bundled them.
  rmSync("node_modules/.vite", { recursive: true, force: true })
  const vite = startVite()
  await waitForServer(vite)
  const page = await open(browser)
  await page.goto(`http://127.0.0.1:${port}/`, { waitUntil: "networkidle0" })
  check(await page.shows("#root h1", "Hello from teq"), "the page renders", await page.text("#root h1"))
  check((await page.runtime()) === "1 renderer, 1 root", "the refresh runtime holds React DOM's renderer and its root on a first load", await page.runtime())
  await sleep(2500)
  await page.shows("#root h1", "Hello from teq")
  check((await page.runtime()) === "1 renderer, 1 root", `and after the optimizer has settled (${page.loads} load${page.loads === 1 ? "" : "s"})`, await page.runtime())
  await page.reload({ waitUntil: "networkidle0" })
  check((await page.runtime()) === "1 renderer, 1 root", "and after a reload with the dependencies pre-bundled", await page.runtime())
  await click(page)
  page.again.length = 0
  let loads = page.loads

  writeFileSync(widgetsFile, greeting("Hello from the hot swap"))
  let link = sbt(`${project}/fastLinkJS`)
  if (!link.ok) return fail(`sbt ${project}/fastLinkJS failed:\n${link.out}`)
  check(link.out.includes('changed ["demo.widgets.Widgets.mjs","hot-build.mjs"]'), "the link rewrote the edited module and the listing, and no other", link.out.match(/changed .*/)?.[0])
  await swapped(page, "Hello from the hot swap", loads)
  const save = async (file, text) => {
    writeFileSync(file, text)
    const linked = sbt(`${project}/fastLinkJS`)
    if (!linked.ok) fail(`sbt ${project}/fastLinkJS failed:\n${linked.out}`)
  }
  loads = await turns(page, save, loads)
  loads = await held(page, save)
  loads = await interleaved(page)

  // demo.Labels is generated into the package's module, which is not per file.
  writeFileSync(labelsFile, "made by the generator\n")
  link = sbt(`${project}/fastLinkJS`)
  if (!link.ok) return fail(`sbt ${project}/fastLinkJS failed after the labels' edit:\n${link.out}`)
  check(await page.shows("#footer", "made by the generator"), "an edit of a generator's input reaches the DOM", await page.text("#footer"))
  check(page.loads === loads + 1 && (await page.text("#counter")) === "count: 0" && page.errors.length === 0, "an edit of a module that is not per file reloads the page, with no page error on the way", [page.loads - loads, await page.text("#counter"), page.errors[0]])
  await settled(page, "after the reload")
  writeFileSync(labelsFile, labels)

  // Two links of teq's into one directory in one command: they take turns, and the directory
  // ends as one of the two builds, whole. Nothing reads the directory for a copy here: a turn
  // is between writers.
  link = sbt(`set ${project} / teqFullServedOutput := (${project} / Compile / fastLinkJS / scalaJSLinkerOutputDirectory).value; all ${project}/fastLinkJS ${project}/teqFullLinkJS`)
  check(link.ok && ["teq's dev link", "teq's full link"].includes(writer()), `two links of teq's into one directory in one command leave one build, whole (${writer()})`, existsSync(directory) ? readdirSync(directory) : "no directory")
  link = sbt(`session clear-all; ${project}/fastLinkJS`)
  check(link.ok && writer() === "teq's dev link", "the dev link after them writes the directory afresh", readdirSync(directory))

  restore()
  vite.kill()
  await page.close()
  await bundle(browser)
}

async function toggle(browser) {
  const vite = startVite()
  await waitForServer(vite)
  const page = await open(browser)
  await page.goto(`http://127.0.0.1:${port}/`, { waitUntil: "networkidle0" })
  await sleep(2500)
  check(await page.shows("#root h1", "Hello from teq"), "the page renders teq's output", await page.text("#root h1"))
  check((await page.runtime()) === "1 renderer, 1 root", "with the refresh runtime in place", await page.runtime())
  let loads = page.loads

  // The toggle off: the Scala.js linker writes the directory, and nothing of teq's stays in it.
  let link = sbt(`set every teqCompiler := false; ${project}/fastLinkJS`, 300_000)
  if (!link.ok) return fail(`the Scala.js link failed:\n${link.out.slice(-3000)}`)
  check(teqs(directory) === 0 && existsSync(`${directory}/main.js`) && !existsSync(`${directory}.backend`), "toggle off: the directory holds the Scala.js linker's files alone", readdirSync(directory))
  check(residents().length === 0, "toggle off: the directory's resident is gone", residents())
  // The dev server read the directory while it changed hands: the page is loaded anew.
  await page.reload({ waitUntil: "networkidle0" })
  check(await page.shows("#root h1", "Hello from teq"), "toggle off: the page runs the linker's output", await page.text("#root h1"))
  check((await page.runtime()) === "absent", "toggle off: the page has no refresh runtime", await page.runtime())
  loads = page.loads
  await page.click("#counter")
  writeFileSync(widgetsFile, greeting("Hello from Scala.js"))
  link = sbt(`${project}/fastLinkJS`, 300_000)
  if (!link.ok) return fail(`the Scala.js link failed after an edit:\n${link.out.slice(-3000)}`)
  check(await page.shows("#root h1", "Hello from Scala.js", 30_000), "toggle off: an edit reaches the DOM", [await page.text("#root h1"), vite.log.slice(-1500)])
  check(page.loads > loads && (await page.text("#counter")) === "count: 0", "toggle off: by a page reload, nothing accepting the update", [page.loads - loads, await page.text("#counter")])
  link = sbt(`${project}/fastLinkJS`, 300_000)
  check(link.ok && teqs(directory) === 0 && existsSync(`${directory}/main.js`), "toggle off: a link that changed nothing leaves the linker's files", readdirSync(directory))

  // Two links into one directory in one command, one of them the Scala.js linker's, which
  // links in a task of its own after the one that takes the directory for it: teq's link is
  // refused where it meets the linker's turn, and the linker waits where it meets teq's.
  const linkers = readdirSync(directory).sort().join()
  check(writer(linkers) === "the Scala.js linker", "toggle off: the directory is the Scala.js linker's, whole", [writer(linkers), linkers])
  writeFileSync(widgetsFile, greeting("Hello from two links"))
  link = sbt(`set ${project} / teqFullServedOutput := (${project} / Compile / fastLinkJS / scalaJSLinkerOutputDirectory).value; all ${project}/fastLinkJS ${project}/teqFullLinkJS`, 300_000)
  const refused = /teq: the Scala\.js linker \(browserdemo\/fastLinkJS\) has .* in this command until its link has ended/.test(link.out)
  const whole = writer(linkers)
  check(link.ok !== refused && whole !== undefined && (!refused || whole === "the Scala.js linker"), `the Scala.js linker's link and a link of teq's into one directory in one command leave one writer's build, whole (${whole}${refused ? ", teq's link refused" : ""})`, [readdirSync(directory), existsSync(`${directory}.backend`) ? readFileSync(`${directory}.backend`, "utf-8") : "no marker", link.out.slice(-600)])
  writeFileSync(widgetsFile, greeting("Hello from Scala.js"))
  link = sbt(`session clear-all; set every teqCompiler := false; ${project}/fastLinkJS`, 300_000)
  check(link.ok && writer(linkers) === "the Scala.js linker", "the Scala.js linker's link after them writes the directory afresh", [readdirSync(directory), link.out.slice(-600)])

  // A command cancelled while scalac compiles for the linker (an interactive client's Ctrl+C;
  // the server ends with a cancelled client of one command): sbt runs none of its tasks that
  // are left, the one that ends the linker's turn among them. The turn is over with the command.
  // A text of this run's own, so that scalac has the file to compile and the command a while
  // to be cancelled in (sbt's disk cache would answer a text compiled before at once).
  writeFileSync(widgetsFile, greeting(`Hello from a cancelled link ${Date.now()}`))
  const server = servers()
  const shell = spawn(sbtReal, ["--client", "-no-colors", "-Dsbt.supershell=false"], { stdio: ["pipe", "pipe", "pipe"], env, detached: true })
  children.push(shell)
  shell.log = ""
  shell.stdout.on("data", (d) => (shell.log += d))
  shell.stderr.on("data", (d) => (shell.log += d))
  await until("prompt", () => /terminate the server with/.test(shell.log), 60_000)
  shell.stdin.write(`${project}/fastLinkJS\n`)
  const compiling = await until("compiling", () => /compiling 1 Scala source/.test(shell.log), 100_000)
  shell.kill("SIGINT")
  await sleep(2000)
  check(compiling && !/Fast optimizing/.test(shell.log) && server.join() === servers().join(), "a command is cancelled before the Scala.js linker links, the server staying", [shell.log.slice(-600), server, servers()])
  shell.kill()
  // A second sbt process after it: the cancelled evaluation gave its claim back at its end,
  // the lock with it.
  const after = spawnSync(sbtReal, ["--server", "--no-server", "--batch", "-no-colors", "-Dsbt.supershell=false", `set every teqLinkWait := scala.concurrent.duration.DurationInt(3).seconds; set every teqCompiler := false; ${project}/fastLinkJS`], { encoding: "utf-8", env, timeout: within(300_000, "a second sbt process after the cancel") })
  if (after.error?.code === "ETIMEDOUT") within(0, "a second sbt process after the cancel")
  check(after.status === 0 && !/is written by another sbt process/.test(`${after.stdout}${after.stderr}`), "a second sbt process links into the directory after the cancelled command: the lock went with the evaluation", [after.status, after.signal, `${after.stdout}${after.stderr}`.replace(/^WARNING: .*\n?/gm, "").slice(-2500)])
  writeFileSync(widgetsFile, greeting("Hello from Scala.js"))

  // And on again: teq writes the directory, and nothing of the linker's stays.
  loads = page.loads
  link = sbt(`set every teqCompiler := true; ${project}/fastLinkJS`)
  if (!link.ok) return fail(`the link failed with the toggle on again:\n${link.out.slice(-3000)}`)
  check(!/in this command until its link has ended/.test(link.out), "the cancelled command's turn is over with it: the next command's link has the directory", link.out.slice(-400))
  const left = readdirSync(directory).filter((name) => !name.endsWith(".mjs") && name !== "main.js")
  check(teqs(directory) === MODULES && left.length === 0 && readFileSync(`${directory}.backend`, "utf-8").startsWith("teq "), "toggle on again: the directory holds teq's modules and stub alone", readdirSync(directory))
  await page.reload({ waitUntil: "networkidle0" })
  check(await page.shows("#root h1", "Hello from Scala.js"), "toggle on again: the page runs teq's output", await page.text("#root h1"))
  check((await page.runtime()) === "1 renderer, 1 root", "toggle on again: the refresh runtime is back", await page.runtime())
  await click(page)
  page.again.length = 0
  loads = page.loads
  writeFileSync(widgetsFile, greeting("Hello from teq again"))
  link = sbt(`${project}/fastLinkJS`)
  if (!link.ok) return fail(`sbt ${project}/fastLinkJS failed:\n${link.out}`)
  await swapped(page, "Hello from teq again", loads)
  link = sbt(`set every teqCompiler := false; ${project}/fastLinkJS; set every teqCompiler := true; ${project}/fastLinkJS`, 300_000)
  check(link.ok && teqs(directory) === MODULES && readdirSync(directory).length === MODULES + 1, "a round trip of the toggle with nothing changed ends on teq's files", readdirSync(directory))


  restore()
  vite.kill()
  await page.close()
}

async function mirrored(browser) {
  const watch = spawn(sbtReal, ["-no-colors", "-Dsbt.supershell=false", `~${project}/fastLinkJS`], { stdio: ["pipe", "pipe", "pipe"], env })
  children.push(watch)
  watch.log = ""
  watch.stdout.on("data", (d) => (watch.log += d))
  watch.stderr.on("data", (d) => (watch.log += d))
  // sbt numbers the watch's builds: "3. Monitoring source files" once the third has ended.
  const builds = () => Math.max(0, ...[...watch.log.matchAll(/(\d+)\. Monitoring source files/g)].map((m) => Number(m[1])))
  const built = async (n, ms = 60_000) => until("build", () => builds() >= n, ms)
  // sbt's watch ignores a file's events for a while after a build that the file triggered
  // (watchAntiEntropy, 500 ms): the saves here are further apart, as a developer's are.
  const save = async (file, text) => { await sleep(1200); writeFileSync(file, text) }
  if (!(await built(1, 200_000))) return fail(`sbt ~${project}/fastLinkJS did not reach its watch:\n${watch.log.slice(-3000)}`)
  check(same(directory, served) && teqs(served) === MODULES, "the mirror holds the directory's files", readdirSync(served))

  const vite = startVite()
  await waitForServer(vite)
  check(residents().length === 1, "vite's own print ran in the watch's server and started no second resident", residents())
  const page = await open(browser)
  await page.goto(`http://127.0.0.1:${port}/`, { waitUntil: "networkidle0" })
  await sleep(2500)
  check(await page.shows("#root h1", "Hello from teq"), "the page renders from the mirror", await page.text("#root h1"))
  check((await page.runtime()) === "1 renderer, 1 root", "the refresh runtime holds React DOM's renderer and its root", await page.runtime())
  await click(page)
  page.again.length = 0
  let loads = page.loads
  let n = builds()

  await save(widgetsFile, greeting("Hello from the watch"))
  await swapped(page, "Hello from the watch", loads)
  await built(++n)
  check(same(directory, served), "the mirror holds the directory's files after the edit")
  loads = await turns(page, save, loads)
  n = builds()

  // A type error: sbt's log has it, the page keeps its state, and the fix lands.
  await save(widgetsFile, greeting("Hello from the hot swap").replace('val themePrefix: String = "theme: "', "val themePrefix: String = 42"))
  await built(++n)
  check(/Widgets\.scala:\d+:\d+: error: /.test(watch.log.slice(-4000)), "a type error is in sbt's log, as teq prints it", watch.log.slice(-300))
  check((await page.text("#root h1")) === "Hello from the hot swap" && (await page.text("#counter")) === "count: 3" && page.loads === loads, "the page keeps its last good state", [await page.text("#root h1"), await page.text("#counter")])
  check((await page.pending()) === undefined, "and waits for nothing", await page.pending())
  page.again.length = 0
  await save(widgetsFile, greeting("Hello from the fix"))
  await swapped(page, "Hello from the fix", loads)
  await built(++n)

  // The generator's input is known to sbt through `fileInputs` alone.
  await save(labelsFile, "made by the generator\n")
  check(await page.shows("#footer", "made by the generator", 30_000), "an edit of a generator's input retriggers the watch and reaches the DOM", await page.text("#footer"))
  await built(++n)
  await save(labelsFile, labels)
  await built(++n)
  await page.shows("#footer", labels.trim(), 30_000)

  // The directory gone (a `clean`) and nothing changed: the link writes it again, and the mirror.
  rmSync(directory, { recursive: true, force: true })
  let link = sbt(`${project}/fastLinkJS`)
  check(link.ok && teqs(directory) === MODULES && existsSync(`${directory}/main.js`) && same(directory, served), "a link that changed nothing writes a removed directory again, and the mirror holds it", existsSync(directory) ? readdirSync(directory) : "no directory")
  rmSync(`${directory}/main.mjs`)
  link = sbt(`${project}/fastLinkJS`)
  check(link.ok && existsSync(`${directory}/main.mjs`) && same(directory, served), "and a removed main.mjs")

  // A second sbt process while this one's resident writes the directory.
  const before = readdirSync(directory).map((name) => `${name} ${readFileSync(join(directory, name)).length}`).join()
  const second = spawnSync(sbtReal, ["--server", "--no-server", "--batch", "-no-colors", "-Dsbt.supershell=false", `set every teqLinkWait := scala.concurrent.duration.DurationInt(3).seconds; ${project}/fastLinkJS`], { encoding: "utf-8", env, timeout: within(300_000, "a second sbt process") })
  if (second.error?.code === "ETIMEDOUT") within(0, "a second sbt process")
  check(second.status !== 0 && /is written by another sbt process \(pid \d+\)/.test(`${second.stdout}${second.stderr}`), "a second sbt process is refused the directory, naming the first", `${second.stdout}${second.stderr}`.slice(-600))
  const after = readdirSync(directory).map((name) => `${name} ${readFileSync(join(directory, name)).length}`).join()
  check(before === after && same(directory, served), "and leaves the directory and the mirror as they were")

  // The watch left with Enter: its resident ends, the server stays, and the next link starts another.
  n = builds()
  await save(widgetsFile, greeting("Hello from the last edit"))
  await built(++n)
  const held = residents()
  watch.stdin.write("\n")
  const gone = await until("resident gone", () => residents().length === 0, 8_000)
  check(held.length === 1 && gone, "the resident ends with the watch, within 5 s", [held, residents()])
  await until("client gone", () => exited(watch), 10_000)
  check(servers().length >= 1, "the sbt server stays", servers())
  link = sbt(`${project}/fastLinkJS`)
  const next = residents()
  check(link.ok && next.length === 1 && next[0] !== held[0], "the next link starts another resident", next)
  const idle = await until("idle resident gone", () => residents().length === 0, 20_000, 500)
  check(idle && servers().length >= 1, "which ends once idle for teqLinkIdle (8 s here), the server staying", residents())

  // A watch of another task that links on its way, `fastLinkJSOutput`: sbt's hook for the end
  // of a watch is the watched task's, not the link's. The resident stays while the watch
  // lasts, longer than teqLinkIdle, and ends by that bound once the watch is over.
  const other = spawn(sbtReal, ["-no-colors", "-Dsbt.supershell=false", `~${project}/fastLinkJSOutput`], { stdio: ["pipe", "pipe", "pipe"], env })
  children.push(other)
  other.log = ""
  other.stdout.on("data", (d) => (other.log += d))
  other.stderr.on("data", (d) => (other.log += d))
  if (!(await until("watch", () => /Monitoring source files/.test(other.log), 100_000))) return fail(`sbt ~${project}/fastLinkJSOutput did not reach its watch:\n${other.log.slice(-2000)}`)
  const kept = residents()
  await sleep(12_000)
  check(kept.length === 1 && residents().join() === kept.join(), "a watch of another task holds the link's resident while it lasts, longer than teqLinkIdle", [kept, residents()])
  other.stdin.write("\n")
  const over = Date.now()
  const lapsed = await until("resident gone", () => residents().length === 0, 20_000, 250)
  check(lapsed && servers().length >= 1, `and the resident ends by teqLinkIdle once that watch is over (${((Date.now() - over) / 1000).toFixed(1)} s), the server staying`, residents())

  restore()
  vite.kill()
  await page.close()
  await bundle(browser)
}

spawnSync(sbtReal, ["--client", "shutdown"], { encoding: "utf-8", env, timeout: within(120_000, "the server's end") })
const browser = await puppeteer.launch({ executablePath: chromePath, headless: true, args: ["--no-sandbox", "--disable-gpu"] })
let out
try {
  await { stock, toggle, mirrored }[mode](browser)
} catch (e) {
  if (e instanceof OutOfTime || left() <= 0) out = e.message
  else fail(String(e.stack ?? e))
} finally {
  stop()
  await browser.close()
  spawnSync(sbtReal, ["--client", "shutdown"], { encoding: "utf-8", env, timeout: 30_000 })
}
if (out !== undefined) {
  console.error(`TIMEOUT ${mode}: the mode's ${LIMIT / 1000} s were used up at "${out}", after ${passed} checks passed and ${failed} failed: the machine was too loaded for the check, which says nothing about the behaviour`)
  process.exitCode = 3
} else console.log(failed ? `${mode}: ${failed} FAILED` : `${mode}: all checks passed`)
