// The run check of a reference project built by teq (bench/mill-reference.scala's `run`): its page served
// from a directory by node's own HTTP server and opened in headless Chrome over the DevTools protocol on a
// pipe (`--remote-debugging-pipe`), nothing to install but node and Chrome. A scenario drives the page and
// checks what it shows; the check fails on an exception the page throws, on a console error, on a request
// that fails and on its time running out. It prints a line per step, then a line per named assertion of the scenario,
// `assert <name>: ok`, `assert <name>: FAIL <reason>` (the first one a failure leaves unchecked) or `assert <name>: not
// reached` (those after it), which the run lines hold to their record (bench/reference/records.scala, run.json: a
// known violation identified by its assertion's name), and one starting with `smoke:` or `FAIL smoke:` at the end,
// and exits 0 or 1. With --assertions it prints the scenario's assertions, `assert <name>`, a line each, and exits 0:
// what a run whose build failed did not check.
//
//   node bench/smoke.mjs <tyrian|indigo|webgl> <directory> [--seconds n] [--screenshots <dir>]
//   node bench/smoke.mjs <tyrian|indigo|webgl> --assertions
//
// tyrian   the sandbox's page 2: "insert" makes a counter (Counter.scala), whose "+" adds one to the number
//          it shows (Counter.scala 17-19)
// indigo   the sandbox game: a frame drawn on its canvas (pixels not all one colour), then the scene-switch
//          key (SandboxGame.scala 213-215, Page Up the next scene) changing what is drawn
// webgl    no page: Chrome makes a WebGL2 context, the preflight of the indigo line
//
// CHROME names the browser (default: google-chrome, then chromium); the directory is served at the root,
// an unknown path answering index.html, as the projects' dev servers do for their routes.
import { spawn } from "node:child_process"
import { createServer } from "node:http"
import { mkdtempSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync, existsSync } from "node:fs"
import { tmpdir } from "node:os"
import { extname, join, normalize } from "node:path"
import { inflateSync } from "node:zlib"

const [scenario, root, ...rest] = process.argv.slice(2)
const option = (name, fallback) => {
  const i = rest.indexOf(name)
  return i >= 0 && i + 1 < rest.length ? rest[i + 1] : fallback
}
const seconds = Number(option("--seconds", "60"))
const shots = option("--screenshots", null)
// Each scenario's named assertions, in the order its steps check them.
const assertions = {
  tyrian: ["the app's page 2", "a counter after insert", "the counter one more after +", "no exception, console error or failed request"],
  indigo: ["the game's canvas", "a frame drawn", "another frame after the scene-switch key", "no exception, console error or failed request"],
  webgl: ["a WebGL2 context"],
}[scenario]
if (assertions && root === "--assertions") {
  assertions.forEach((a) => console.log(`assert ${a}`))
  process.exit(0)
}
let checked = 0
// The next assertion holds: its step is reached and checked.
const held = () => { checked++ }
if (!["tyrian", "indigo", "webgl"].includes(scenario) || (scenario !== "webgl" && !root)) {
  console.error("usage: node bench/smoke.mjs <tyrian|indigo|webgl> <directory> [--seconds n] [--screenshots <dir>]")
  process.exit(2)
}

const started = Date.now()
const problems = []
let finished = false
// A text of the page's (a message, a value, a failure's reason) on one line of the checker's, every line separator
// escaped (LF, CR, CR LF, NEL, LS and PS, all that a reader's lines or a regex's `.` end at): the checker's own lines,
// `assert <name>: <outcome>` and the verdict, which the records read (bench/reference/records.scala `Assertions`), are
// never shaped by the page's text.
const separators = { "\r\n": "\\r\\n", "\n": "\\n", "\r": "\\r", "\u0085": "\\u0085", "\u2028": "\\u2028", "\u2029": "\\u2029" }
const oneLine = (s) => String(s).replace(/\r\n|[\n\r\u0085\u2028\u2029]/g, (c) => separators[c])
function finish(ok, reason) {
  if (finished) return
  finished = true
  const line = oneLine(reason)
  assertions.forEach((a, i) => console.log(`assert ${a}: ${i < checked ? "ok" : i === checked && !ok ? `FAIL ${line}` : "not reached"}`))
  console.log(`${ok ? "" : "FAIL "}smoke: ${scenario} ${line} (${((Date.now() - started) / 1000).toFixed(1)} s)`)
  cleanup()
  process.exit(ok ? 0 : 1)
}
const deadline = setTimeout(() => finish(false, `ran out of its ${seconds} s`), seconds * 1000)

// ---- the server ----
const types = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".json": "application/json", ".png": "image/png", ".css": "text/css", ".map": "application/json", ".ttf": "font/ttf", ".wav": "audio/wav", ".mp3": "audio/mpeg", ".glsl": "text/plain", ".txt": "text/plain", ".svg": "image/svg+xml", ".jpg": "image/jpeg" }
const server = root ? createServer((req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, "http://localhost").pathname)).replace(/^(\.\.[/\\])+/, "")
  let file = join(root, path)
  if (!existsSync(file) || statSync(file).isDirectory()) {
    const index = join(file, "index.html")
    file = existsSync(index) ? index : extname(path) ? null : join(root, "index.html")
  }
  // The browser asks for an icon of its own accord, which the projects do not ship.
  if ((!file || !existsSync(file)) && path === "/favicon.ico") {
    res.writeHead(204)
    res.end()
    return
  }
  if (!file || !existsSync(file)) {
    res.writeHead(404)
    res.end()
    return
  }
  res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream", "cache-control": "no-store" })
  res.end(readFileSync(file))
}) : null

// ---- the browser, over the DevTools protocol on fds 3 and 4 ----
const profile = mkdtempSync(join(tmpdir(), "smoke-chrome-"))
const chromes = process.env.CHROME ? [process.env.CHROME] : ["google-chrome", "chromium", "chromium-browser"]
let chrome = null
function cleanup() {
  clearTimeout(deadline)
  try { chrome?.kill("SIGKILL") } catch {}
  try { server?.close() } catch {}
  try { rmSync(profile, { recursive: true, force: true }) } catch {}
}
process.on("exit", cleanup)

function launch(i) {
  if (i >= chromes.length) finish(false, `no browser: ${chromes.join(", ")} not found (CHROME names one)`)
  const args = ["--headless=new", "--remote-debugging-pipe", `--user-data-dir=${profile}`, "--no-first-run", "--no-default-browser-check",
    "--disable-gpu-sandbox", "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist",
    "--window-size=1024,768", "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "about:blank"]
  if (process.getuid?.() === 0) args.unshift("--no-sandbox")
  const p = spawn(chromes[i], args, { stdio: ["ignore", "ignore", "pipe", "pipe", "pipe"] })
  p.on("error", () => launch(i + 1))
  p.on("spawn", () => { chrome = p; connect(p) })
  let errors = ""
  p.stdio[2].on("data", (d) => { errors = (errors + d).slice(-2000) })
  p.on("exit", (code) => { if (!finished) finish(false, `the browser exited (${code}): ${errors.trim().split("\n").slice(-3).join(" | ")}`) })
}

let nextId = 1
const pending = new Map()
const listeners = []
let send = null
function connect(p) {
  const out = p.stdio[3]
  let buffer = Buffer.alloc(0)
  p.stdio[4].on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk])
    let end
    while ((end = buffer.indexOf(0)) >= 0) {
      const message = JSON.parse(buffer.subarray(0, end).toString("utf8"))
      buffer = buffer.subarray(end + 1)
      if (message.id !== undefined && pending.has(message.id)) {
        const { resolve, reject } = pending.get(message.id)
        pending.delete(message.id)
        message.error ? reject(new Error(`${message.error.message} ${message.error.data ?? ""}`)) : resolve(message.result)
      } else {
        for (const l of listeners) l(message)
      }
    }
  })
  send = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = nextId++
    pending.set(id, { resolve, reject })
    out.write(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }) + "\0")
  })
  run().catch((e) => finish(false, `${e.message}`))
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
async function page(url) {
  const { targetId } = await send("Target.createTarget", { url: "about:blank" })
  const { sessionId } = await send("Target.attachToTarget", { targetId, flatten: true })
  const call = (method, params) => send(method, params, sessionId)
  listeners.push((m) => {
    if (m.sessionId !== sessionId) return
    if (m.method === "Runtime.exceptionThrown") {
      const d = m.params.exceptionDetails
      problems.push(`exception: ${d.exception?.description?.split("\n").slice(0, 4).join(" | ") ?? d.text}`)
    } else if (m.method === "Runtime.consoleAPICalled" && m.params.type === "error") {
      problems.push(`console error: ${m.params.args.map((a) => a.value ?? a.description).join(" ").slice(0, 3000)}`)
    } else if (m.method === "Runtime.consoleAPICalled") {
      console.log(`  page: ${oneLine(m.params.args.map((a) => a.value ?? a.description).join(" ").slice(0, 200))}`)
    } else if (m.method === "Log.entryAdded" && m.params.entry.level === "error") {
      problems.push(`log error: ${m.params.entry.text.slice(0, 300)}${m.params.entry.url ? " " + m.params.entry.url : ""}`)
    } else if (m.method === "Network.loadingFailed" && !m.params.canceled) {
      problems.push(`request failed: ${m.params.errorText}`)
    }
  })
  await call("Runtime.enable")
  await call("Log.enable")
  await call("Page.enable")
  await call("Network.enable")
  const loaded = new Promise((resolve) => listeners.push((m) => { if (m.sessionId === sessionId && m.method === "Page.loadEventFired") resolve() }))
  await call("Page.navigate", { url })
  await loaded
  const evaluate = async (expression) => {
    const r = await call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })
    if (r.exceptionDetails) throw new Error(`evaluating ${expression.slice(0, 80)}: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`)
    return r.result.value
  }
  return { call, evaluate }
}

// Waits until `expression` evaluates to a truthy value, failing on a problem the page reported.
async function until(p, what, expression, ms = 20000) {
  const end = Date.now() + ms
  for (;;) {
    if (problems.length) throw new Error(`${problems[0]} (waiting for ${what})`)
    const v = await p.evaluate(expression)
    if (v) {
      console.log(`  ${what}: ${typeof v === "string" ? oneLine(v) : "yes"}`)
      return v
    }
    if (Date.now() > end) throw new Error(`no ${what} within ${ms / 1000} s`)
    await sleep(100)
  }
}

// A screenshot's pixels: the PNG decoded (8-bit RGB or RGBA, as Chrome writes them).
function pixels(png) {
  let pos = 8, width = 0, height = 0, channels = 0
  const data = []
  while (pos < png.length) {
    const len = png.readUInt32BE(pos), type = png.toString("latin1", pos + 4, pos + 8)
    const body = png.subarray(pos + 8, pos + 8 + len)
    if (type === "IHDR") { width = body.readUInt32BE(0); height = body.readUInt32BE(4); channels = body[9] === 6 ? 4 : 3 }
    if (type === "IDAT") data.push(body)
    pos += 12 + len
  }
  const raw = inflateSync(Buffer.concat(data)), stride = width * channels, out = Buffer.alloc(height * stride)
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)], line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1))
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? out[y * stride + x - channels] : 0, b = y > 0 ? out[(y - 1) * stride + x] : 0
      const c = x >= channels && y > 0 ? out[(y - 1) * stride + x - channels] : 0
      const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c)
      const pred = [0, a, b, (a + b) >> 1, pa <= pb && pa <= pc ? a : pb <= pc ? b : c][filter]
      out[y * stride + x] = (line[x] + pred) & 255
    }
  }
  return { width, height, channels, data: out }
}

async function shot(p, name) {
  const { data } = await p.call("Page.captureScreenshot", { format: "png" })
  const png = Buffer.from(data, "base64")
  if (shots) { mkdirSync(shots, { recursive: true }); writeFileSync(join(shots, `${scenario}-${name}.png`), png) }
  return pixels(png)
}

// The colours of the region of the page the canvas covers, counted, and a digest of its pixels.
function canvasPicture(img, box) {
  const colours = new Set()
  let digest = 0
  for (let y = Math.max(0, Math.floor(box.y)); y < Math.min(img.height, box.y + box.height); y++) {
    for (let x = Math.max(0, Math.floor(box.x)); x < Math.min(img.width, box.x + box.width); x++) {
      const i = (y * img.width + x) * img.channels
      const rgb = (img.data[i] << 16) | (img.data[i + 1] << 8) | img.data[i + 2]
      colours.add(rgb)
      digest = (Math.imul(digest, 31) + rgb) | 0
    }
  }
  return { colours: colours.size, digest }
}

async function tyrian() {
  const p = await page(`http://127.0.0.1:${server.address().port}/page2`)
  await until(p, "the app's page 2", `[...document.querySelectorAll("button")].some(b => b.textContent === "insert") && "insert and remove shown"`)
  held()
  await p.evaluate(`[...document.querySelectorAll("button")].find(b => b.textContent === "insert").click()`)
  const counter = `(() => { const plus = [...document.querySelectorAll("button")].find(b => b.textContent === "+"); return plus && plus.previousElementSibling ? plus.previousElementSibling.textContent : null })()`
  const before = await until(p, "a counter after insert", counter)
  held()
  await p.evaluate(`[...document.querySelectorAll("button")].find(b => b.textContent === "+").click()`)
  const after = await until(p, "the counter after +", `(() => { const v = ${counter}; return v !== ${JSON.stringify(before)} ? v : null })()`)
  if (Number(after) !== Number(before) + 1) throw new Error(`the counter went from ${before} to ${after}, not ${Number(before) + 1}`)
  held()
  if (problems.length) throw new Error(problems[0])
  held()
  finish(true, `counter ${before} -> ${after} after a click on +`)
}

async function indigo() {
  const p = await page(`http://127.0.0.1:${server.address().port}/`)
  const box = JSON.parse(await until(p, "the game's canvas", `(() => { const c = document.querySelector("canvas"); if (!c || !c.width) return null; const r = c.getBoundingClientRect(); return JSON.stringify({ x: r.x, y: r.y, width: r.width, height: r.height }) })()`))
  held()
  // A frame: the canvas shows more than one colour.
  let first = null
  const end = Date.now() + 20000
  while (!first) {
    if (problems.length) throw new Error(`${problems[0]} (waiting for a frame)`)
    const picture = canvasPicture(await shot(p, "frame"), box)
    if (picture.colours > 1) first = picture
    else if (Date.now() > end) throw new Error("the canvas shows one colour after 20 s: no frame drawn")
    else await sleep(250)
  }
  console.log(`  a frame: ${first.colours} colours on the canvas`)
  held()
  await p.evaluate(`document.querySelector("canvas").focus(); window.focus(); true`)
  for (const type of ["keyDown", "keyUp"]) {
    await p.call("Input.dispatchKeyEvent", { type, key: "PageUp", code: "PageUp", windowsVirtualKeyCode: 33, nativeVirtualKeyCode: 33 })
  }
  let switched = null
  const end2 = Date.now() + 20000
  while (!switched) {
    if (problems.length) throw new Error(`${problems[0]} (waiting for the next scene)`)
    const picture = canvasPicture(await shot(p, "scene"), box)
    if (picture.digest !== first.digest && picture.colours > 1) switched = picture
    else if (Date.now() > end2) throw new Error("the canvas did not change within 20 s of the scene-switch key")
    else await sleep(250)
  }
  held()
  if (problems.length) throw new Error(problems[0])
  held()
  finish(true, `a frame of ${first.colours} colours, another (${switched.colours} colours) after the scene-switch key`)
}

async function webgl() {
  const { targetId } = await send("Target.createTarget", { url: "about:blank" })
  const { sessionId } = await send("Target.attachToTarget", { targetId, flatten: true })
  const r = await send("Runtime.evaluate", { expression: `(() => { const g = document.createElement("canvas").getContext("webgl2"); return g ? g.getParameter(g.VERSION) : null })()`, returnByValue: true }, sessionId)
  if (!r.result.value) finish(false, "the browser makes no WebGL2 context")
  held()
  finish(true, `the browser makes a WebGL2 context: ${r.result.value}`)
}

async function run() {
  if (scenario === "tyrian") await tyrian()
  else if (scenario === "indigo") await indigo()
  else await webgl()
}

if (server) server.listen(0, "127.0.0.1", () => launch(0))
else launch(0)
