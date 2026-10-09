// Drives `teq lsp` over its stdin and stdout through the scenarios of tests/lsp.sh, every
// expected answer written from the sources of tests/lsp/ws and tests/lsp/bare: a location is
// named by the text it points at (`at(file, "def area", 1, 4, 4)`: the second `def area`, four
// characters in, four long), so that the test says where the answer must point, not what the
// server said last time. Usage: node driver.mjs <teq> <work dir> [<jar to put on the JVM class
// path> [<another version of it>]]; the work dir holds copies of the workspaces, which the
// scenarios edit.
import { spawn, execSync } from "node:child_process"
import { createHash } from "node:crypto"
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync, copyFileSync, existsSync, utimesSync, symlinkSync, realpathSync, appendFileSync, chmodSync, readdirSync, statSync, lstatSync } from "node:fs"
import { join, dirname } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { parse as parseLock } from "../../integrations/vite/lock.js"

const [teq, work, jar, jar2] = process.argv.slice(2)
const here = dirname(fileURLToPath(import.meta.url))
let passed = 0
let failed = 0
function check(name, ok, detail) {
  if (ok) {
    passed++
  } else {
    failed++
    console.log(`FAIL ${name}${detail === undefined ? "" : `: ${typeof detail === "string" ? detail : JSON.stringify(detail)}`}`)
  }
}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
/** Whether `pred` holds within `timeout`, asked every 20 ms. */
async function until(pred, timeout) {
  const deadline = Date.now() + timeout
  while (!pred()) {
    if (Date.now() > deadline) return false
    await sleep(20)
  }
  return true
}

// --- The client ------------------------------------------------------------------------------

class Client {
  constructor(cwd, env = {}, program = teq, args = ["lsp"]) {
    this.proc = spawn(program, args, { cwd, env: { ...process.env, ...env }, stdio: ["pipe", "pipe", "pipe"] })
    this.buffer = Buffer.alloc(0)
    this.nextId = 1
    this.waiting = new Map()
    this.notifications = []
    this.requests = []
    this.orphanErrors = []
    this.stderr = ""
    this.exited = new Promise((resolve) => this.proc.on("exit", (code) => resolve(code)))
    this.proc.stderr.on("data", (d) => (this.stderr += d))
    this.proc.stdout.on("data", (d) => {
      this.buffer = Buffer.concat([this.buffer, d])
      for (;;) {
        const end = this.buffer.indexOf("\r\n\r\n")
        if (end < 0) return
        const header = this.buffer.subarray(0, end).toString()
        const length = Number(/Content-Length: *(\d+)/i.exec(header)?.[1])
        if (this.buffer.length < end + 4 + length) return
        const body = JSON.parse(this.buffer.subarray(end + 4, end + 4 + length).toString())
        this.buffer = this.buffer.subarray(end + 4 + length)
        this.receive(body)
      }
    })
  }
  receive(msg) {
    if (msg.id === null && msg.error) {
      this.orphanErrors.push(msg.error)
    } else if (msg.id !== undefined && msg.method === undefined) {
      const w = this.waiting.get(msg.id)
      if (w) {
        this.waiting.delete(msg.id)
        w(msg)
      }
    } else if (msg.id !== undefined) {
      this.requests.push(msg)
      this.write({ jsonrpc: "2.0", id: msg.id, result: null })
    } else {
      this.notifications.push(msg)
    }
  }
  write(msg, pieces = 1) {
    const body = Buffer.from(JSON.stringify(msg))
    const frame = Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`), body])
    if (pieces === 1) return this.proc.stdin.write(frame)
    // One frame in several writes, cut inside the header and inside the body.
    const cuts = [7, frame.length - 5]
    let from = 0
    const parts = []
    for (const c of [...cuts, frame.length]) {
      parts.push(frame.subarray(from, c))
      from = c
    }
    return (async () => {
      for (const p of parts) {
        this.proc.stdin.write(p)
        await sleep(30)
      }
    })()
  }
  writeRaw(text) {
    this.proc.stdin.write(`Content-Length: ${Buffer.byteLength(text)}\r\n\r\n${text}`)
  }
  async request(method, params, { timeout = 120000, pieces = 1 } = {}) {
    const id = this.nextId++
    const answer = new Promise((resolve) => this.waiting.set(id, resolve))
    await this.write({ jsonrpc: "2.0", id, method, params }, pieces)
    const timer = sleep(timeout).then(() => ({ timeout: true }))
    const msg = await Promise.race([answer, timer])
    if (msg.timeout) throw new Error(`${method}: no answer in ${timeout} ms (stderr: ${this.stderr.slice(-400)})`)
    return msg
  }
  async result(method, params, opts) {
    const msg = await this.request(method, params, opts)
    if (msg.error) throw new Error(`${method}: ${JSON.stringify(msg.error)}`)
    return msg.result
  }
  notify(method, params) {
    this.write({ jsonrpc: "2.0", method, params })
  }
  mark() {
    return this.notifications.length
  }
  diagnosticsSince(mark) {
    return this.notifications.slice(mark).filter((n) => n.method === "textDocument/publishDiagnostics").map((n) => n.params)
  }
  /** The first publication for `uri` after `mark` that `pred` accepts, waiting up to `timeout`. */
  async waitDiagnostics(mark, uri, pred = () => true, timeout = 120000) {
    const until = Date.now() + timeout
    while (Date.now() < until) {
      const found = this.diagnosticsSince(mark).find((p) => p.uri === uri && pred(p.diagnostics))
      if (found) return found.diagnostics
      await sleep(50)
    }
    return undefined
  }
  latestDiagnostics(uri) {
    const all = this.diagnosticsSince(0).filter((p) => p.uri === uri)
    return all.length ? all[all.length - 1].diagnostics : []
  }
}

// --- Positions ---------------------------------------------------------------------------------

let encoding = "utf-16"
const uriOf = (file) => pathToFileURL(file).href
function positionAt(text, offset) {
  const before = text.slice(0, offset)
  const line = before.split("\n").length - 1
  const lineText = before.slice(before.lastIndexOf("\n") + 1)
  const character = encoding === "utf-8" ? Buffer.byteLength(lineText) : lineText.length
  return { line, character }
}
/** The offset of the `n`th occurrence of `needle`, plus `delta` characters. */
function offsetIn(file, needle, n = 0, delta = 0) {
  const text = readFileSync(file, "utf-8")
  let i = -1
  for (let k = 0; k <= n; k++) {
    i = text.indexOf(needle, i + 1)
    if (i < 0) throw new Error(`${needle} (${n}) not in ${file}`)
  }
  return [text, i + delta]
}
function pos(file, needle, n = 0, delta = 0) {
  const [text, offset] = offsetIn(file, needle, n, delta)
  return positionAt(text, offset)
}
function at(file, needle, n = 0, delta = 0, length = needle.length - delta) {
  const [text, offset] = offsetIn(file, needle, n, delta)
  return { uri: uriOf(file), range: { start: positionAt(text, offset), end: positionAt(text, offset + length) } }
}
const doc = (file) => ({ textDocument: { uri: uriOf(file) } })
const at_ = (file, needle, n = 0, delta = 0) => ({ ...doc(file), position: pos(file, needle, n, delta) })
const sortLocs = (locs) => [...(locs ?? [])].sort((a, b) => (a.uri + JSON.stringify(a.range)).localeCompare(b.uri + JSON.stringify(b.range)))
const sameLocs = (a, b) => same(sortLocs(a), sortLocs(b))

// --- The workspaces ---------------------------------------------------------------------------

rmSync(work, { recursive: true, force: true })
mkdirSync(join(work, "real"), { recursive: true })
// The workspaces are opened through a symbolic link, as macOS hands out its temporary
// directories (`/var` for `/private/var`): the answers must name files as the client does.
symlinkSync(join(work, "real"), join(work, "link"))
const ws = join(work, "link/ws")
const bare = join(work, "link/bare")
cpSync(join(here, "ws"), ws, { recursive: true })
cpSync(join(here, "bare"), bare, { recursive: true })
const shapes = join(ws, "shared/src/shapes/Shapes.scala")
const page = join(ws, "js/src/web/Page.scala")
const calls = join(ws, "js/src/web/Calls.scala")
const other = join(ws, "js/src/web/Other.scala")
const webMain = join(ws, "js/src/web/Main.scala")
const appMain = join(ws, "jvm/src/app/Main.scala")
const crlf = join(ws, "js/src/web/Crlf.scala")
const astral = join(ws, "js/src/web/Astral.scala")
const stray = join(ws, "outside/Stray.scala")
const findings = join(ws, "js/src/web/Findings.scala")
const vars = join(ws, "js/src/web/Vars.scala")
const hello = join(bare, "Hello.scala")
const broken = join(bare, "Broken.scala")
writeFileSync(crlf, "package web\r\n\r\nobject Crlf:\r\n  def one: Int = 1\r\n  def two: Int = one + one\r\n")
writeFileSync(astral, 'package web\n\nobject Astral:\n  val emoji = "😀"; val after = emoji\n')
const completing = join(ws, "js/src/web/Complete.scala")
const completingText = `package web

import shapes.*
import scala.collection.mutable.ListBuffer
import scala.language.implicitConversions

class Base:
  protected def guarded: Int = 1
  def inherited: Int = 2

class Holder extends Base:
  private def secretly: Int = 3
  def own(x: Int): Int = x
  def own(x: String): Int = 0
  val \`my value\`: Int = 4
  def foobar: Int = 5

object Holder:
  def fromCompanion: Holder = Holder()

object Complete:
  def enclosing: Int = 6
  def run(param: Int): Int =
    val holder = Holder()
    val point = Point(1, 2)
    holder.own(1) + param
`
writeFileSync(completing, completingText)
// What auto-import imports: a class of another package, and a file whose texts the scenario
// varies around it.
const thing = join(ws, "js/src/lib/Thing.scala")
mkdirSync(dirname(thing), { recursive: true })
writeFileSync(thing, "package lib\n\nclass Thing\n")
const importing = join(ws, "js/src/web/Importing.scala")
writeFileSync(importing, "package web\n\nobject Importing\n")
// The applied function values of js/src/web/Applied.scala and the macro it applies, copied into
// the JVM project and into the JavaScript project on scala-library.
const appliedIn = Object.fromEntries(["js/src/web", "jvm/src/app", "jsstd/src/web"].map((dir) => [dir.split("/")[0], join(ws, dir, "Applied.scala")]))
for (const project of ["jvm", "jsstd"]) {
  mkdirSync(dirname(appliedIn[project]), { recursive: true })
  for (const name of ["Applied.scala", "AppliedMacro.scala"]) {
    const text = readFileSync(join(dirname(appliedIn.js), name), "utf-8")
    writeFileSync(join(dirname(appliedIn[project]), name), project === "jvm" ? text.replace("package web", "package app") : text)
  }
}
let exports = 0
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
/** Writes an export (teq.lock) of the projects, with a new modification time whatever the file
 * system's granularity. A project is `{ platform, sources, classpath, flags, description }`. */
function writeExport(file, projects) {
  const json = { teq: "0.1.2", format: 1, binaries: {}, inputs: { files: {} }, repositories: [{ id: "maven-central", url: "https://repo1.maven.org/maven2/" }], jars: {}, projects: {} }
  for (const [name, p] of Object.entries(projects)) {
    // A pinned jar (`pinned` below) goes into the table, its key onto the classpath.
    const classpath = (p.classpath ?? []).map((e) => (e.key ? ((json.jars[e.key] = e.record), e.key) : e))
    const configurations = { compile: { sources: p.sources, classpath, flags: p.flags ?? {}, generators: p.generators ?? [] } }
    if (p.test) configurations.test = { sources: p.test, classpath: [{ project: name, configuration: "compile" }, ...classpath], flags: p.flags ?? {} }
    json.projects[name] = { base: name, platform: p.platform ?? "jvm", configurations, description: p.description ?? {} }
  }
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, lockText(json))
  const t = new Date(Math.floor(Date.now() / 1000) * 1000 + ++exports * 1000)
  utimesSync(file, t, t)
}
const wsExport = join(ws, "teq.lock")
const jvmProject = (classpath = []) => ({ platform: "jvm", sources: ["shared/src", "jvm/src"], classpath })
const jsProject = (description = {}) => ({ platform: "js", sources: ["shared/src", "js/src"], flags: { maxInlines: 32 }, description })
// sbt's aggregate project exports no sources: no session for it.
const aggregate = { platform: "jvm", sources: [] }
// A JavaScript project on scala-library, which its description's flags select.
const jsStdProject = { platform: "js", sources: ["jsstd/src"], description: { keys: { flags: "--std=scala-library" } } }
const wsProjects = (more = {}) => ({ jvm: jvmProject(), js: jsProject(), jsstd: jsStdProject, aggregate, ...more })
writeExport(wsExport, wsProjects())

// --- The main workspace, UTF-16 ----------------------------------------------------------------

async function mainWorkspace() {
  const c = new Client(ws)
  try {
    const init = await c.result("initialize", {
      processId: null,
      rootUri: uriOf(ws),
      capabilities: { general: { positionEncodings: ["utf-16"] }, workspace: { didChangeWatchedFiles: { dynamicRegistration: true } }, window: { workDoneProgress: true } },
    })
    const caps = init.capabilities
    check(
      "initialize: capabilities",
      caps.positionEncoding === "utf-16" && caps.textDocumentSync?.change === 1 && caps.definitionProvider && caps.referencesProvider && caps.hoverProvider && caps.documentSymbolProvider && caps.workspaceSymbolProvider && caps.implementationProvider && caps.callHierarchyProvider,
      caps,
    )
    const before = await c.request("textDocument/definition", at_(page, "local +"))
    check("a request before initialized is answered", sameLocs(before.result, [at(page, "val local", 0, 4, 5)]), before)
    check("no registration before initialized", !c.requests.some((r) => r.method === "client/registerCapability"), c.requests)
    c.notify("initialized", {})
    await sleep(200)
    const registered = c.requests.filter((r) => r.method === "client/registerCapability").map((r) => JSON.stringify(r.params))
    check("the watched files are registered at initialized", registered.length === 1 && ["**/*.scala", "**/teq.lock", "**/*.sbt"].every((g) => registered[0].includes(g)), c.requests)
    const created = () => c.requests.filter((r) => r.method === "window/workDoneProgress/create").map((r) => r.params.token)
    const progress = (kind) => c.notifications.filter((n) => n.method === "$/progress" && n.params.value.kind === kind).map((n) => n.params)
    const childCount = () => execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
    // The JS session started for the request before `initialized`, nothing at `initialized`; a
    // request on the shared root, which both projects' own sources hold, is served by the
    // running session whose closure holds it; one on a file of the JVM project's starts its.
    await sleep(300)
    await c.result("textDocument/hover", at_(shapes, "def helper", 0, 4))
    const onlyJs = childCount() === 1 && !progress("begin").some((p) => p.value.message === "checking jvm/compile")
    await c.result("textDocument/hover", at_(appMain, "G.total", 0, 2))
    const startTokens = ["js/compile", "jvm/compile"].map((name) => progress("begin").find((p) => p.value.message === `checking ${name}`)?.token)
    check(
      "sessions start on demand: the JS one for a request on its file, the shared root's served by it, the JVM's for its file, a progress token each",
      onlyJs && startTokens.every((t) => t !== undefined && created().includes(t)) && progress("begin").every((p) => p.value.title === "teq"),
      { onlyJs, created: created(), begun: progress("begin") },
    )

    // workspace/symbol from the running sessions: both projects' programs answer.
    const main = await c.result("workspace/symbol", { query: "Main" })
    check("workspace/symbol: the JVM project's object", main?.some((s) => s.name === "Main" && same(s.location, at(appMain, "object Main", 0, 7, 4)) && s.kind === 2), main)
    const pageSym = await c.result("workspace/symbol", { query: "pag" })
    check("workspace/symbol: the JS project, prefix match", pageSym?.some((s) => s.name === "Page" && same(s.location, at(page, "object Page", 0, 7, 4)) && s.containerName === "web"), pageSym)
    const circ = await c.result("workspace/symbol", { query: "IRCLE" })
    check("workspace/symbol: case-insensitive, once for a file of both projects", circ?.filter((s) => s.name === "Circle").length === 1, circ)
    const ended = await until(() => startTokens.every((t) => progress("end").some((p) => p.token === t)), 5000)
    check("the tokens end once the first builds answered", ended, progress("end"))
    check("a project without sources starts no session", childCount() === 2 && !c.diagnosticsSince(0).some((p) => p.uri === uriOf(wsExport)), { children: childCount(), published: c.diagnosticsSince(0).map((p) => p.uri) })

    // The base protocol: a frame in pieces, a malformed message, an unknown method.
    const split = await c.request("textDocument/hover", at_(shapes, "def helper", 0, 4), { pieces: 3 })
    check("a frame written in pieces", split.result?.contents?.value?.includes("def shapes.Geometry.helper(x: Int): Int"), split)
    c.writeRaw("{not json")
    const unknown = await c.request("textDocument/formatting", doc(page))
    check("a malformed message: a parse error", c.orphanErrors.some((e) => e.code === -32700), c.orphanErrors)
    check("an unknown method: MethodNotFound", unknown.error?.code === -32601, unknown)

    // Diagnostics: an open document's text, an error, its fix.
    let m = c.mark()
    const otherText = readFileSync(other, "utf-8")
    const otherError = otherText.replace("def value: Int = 1", 'def value: Int = "one"')
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(other), languageId: "scala", version: 1, text: otherError } })
    let d = await c.waitDiagnostics(m, uriOf(other), (ds) => ds.length > 0)
    const one = otherError.indexOf('"one"')
    check(
      "didOpen: the open text's error",
      d?.length === 1 && d[0].severity === 1 && d[0].message.includes("type mismatch") && same(d[0].range, { start: positionAt(otherError, one), end: positionAt(otherError, one + 5) }),
      d,
    )
    m = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(other), version: 2 }, contentChanges: [{ text: otherText }] })
    d = await c.waitDiagnostics(m, uriOf(other))
    check("the fix: an empty list", same(d, []), d)
    m = c.mark()
    const otherError2 = otherText.replace("def value: Int = 1", "def value: Int = true")
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(other), version: 3 }, contentChanges: [{ text: otherError2 }] })
    d = await c.waitDiagnostics(m, uriOf(other), (ds) => ds.length > 0)
    const [otherErrText, otherErrOffset] = [otherError2, otherError2.indexOf("true")]
    check("an edit's error on its file", d?.length === 1 && same(d[0].range, { start: positionAt(otherErrText, otherErrOffset), end: positionAt(otherErrText, otherErrOffset + 4) }), d)
    await sleep(300)
    check("the edit publishes that file only", c.diagnosticsSince(m).every((p) => p.uri === uriOf(other)), c.diagnosticsSince(m))
    m = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(other), version: 4 }, contentChanges: [{ text: otherText }] })
    d = await c.waitDiagnostics(m, uriOf(other))
    check("the second fix: an empty list", same(d, []), d)

    // Definitions.
    const defs = [
      ["a local val", at_(page, "local +"), at(page, "val local", 0, 4, 5)],
      ["a parameter", at_(page, "shape.area"), at(page, "shape: Shape", 0, 0, 5)],
      ["a member", at_(page, "circle.area", 0, 7), at(shapes, "def area", 1, 4, 4)],
      ["an extension method", at_(page, "q.norm", 0, 2), at(shapes, "def norm", 0, 4, 4)],
      ["an extension supplied by a given", at_(page, "q.show", 0, 2), at(shapes, "def show", 1, 4, 4)],
      ["a member through an implicit conversion", at_(page, "5.x", 0, 2), at(shapes, "x: Int, y", 0, 0, 1)],
      ["an overloaded method's chosen alternative", at_(page, 'overloaded("x")'), at(shapes, "def overloaded", 1, 4, 10)],
      ["an infix operator", at_(page, "p + Point", 0, 2), at(shapes, "def +", 0, 4, 1)],
      ["a class", at_(page, "new Circle", 0, 4), at(shapes, "class Circle", 0, 6, 6)],
      ["a case class applied", at_(page, "Point(1, 2)"), at(shapes, "case class Point", 0, 11, 5)],
      ["a type alias", at_(page, "Area = 1.0"), at(shapes, "type Area", 0, 5, 4)],
      ["a type parameter", at_(shapes, "fallback: T", 0, 10), at(shapes, "first[T]", 0, 6, 1)],
      ["a type in a signature", at_(page, "shape: Shape", 0, 7), at(shapes, "trait Shape", 0, 6, 5)],
      ["an import's last segment", at_(page, "Geometry.{describe", 0, 0), at(shapes, "object Geometry", 0, 7, 8)],
      ["an import's renamed selector", at_(page, "describe as", 0, 0), at(shapes, "def describe", 0, 4, 8)],
      ["a renamed import used", at_(page, "describePoint(q)"), at(shapes, "def describe", 0, 4, 8)],
      ["an exported member", at_(page, "Exports.overloaded", 0, 8), at(shapes, "def overloaded", 0, 4, 10)],
      ["an export clause's selector", at_(shapes, "Geometry.overloaded", 0, 9), at(shapes, "def overloaded", 0, 4, 10)],
      ["an export clause's path", at_(shapes, "export Geometry", 0, 7), at(shapes, "object Geometry", 0, 7, 8)],
      ["a named argument", at_(page, "label = "), at(shapes, 'label: String = "p"', 0, 0, 5)],
      ["a case class in a pattern", at_(page, "case Point", 0, 5), at(shapes, "case class Point", 0, 11, 5)],
      ["a pattern binder", at_(page, "a + b"), at(page, "Point(a, b)", 0, 6, 1)],
      ["an enum case", at_(page, "Color.Red", 0, 6), at(shapes, "Red", 0, 0, 3)],
      ["an enum", at_(page, "Color.Red"), at(shapes, "enum Color", 0, 5, 5)],
      ["a given used by name", at_(page, "pointShow.show"), at(shapes, "given pointShow", 0, 6, 9)],
      ["an inline method expanded", at_(page, "twice(3)"), at(shapes, "def twice", 0, 4, 5)],
      ["a macro", at_(page, "plusOne(2)"), at(join(ws, "js/src/web/Macros.scala"), "def plusOne", 0, 4, 7)],
      ["a nested object", at_(page, "Geometry.Nested", 0, 9), at(shapes, "object Nested", 0, 7, 6)],
      ["a member imported through nested objects", at_(page, "= depth", 0, 2), at(shapes, "val depth", 0, 4, 5)],
      ["the nested object of an import's path", at_(page, "Nested.*", 0, 0), at(shapes, "object Nested", 0, 7, 6)],
      ["a lambda's parameter", at_(shapes, "acc + s", 0, 0), at(shapes, "(acc, s)", 0, 1, 3)],
      ["a declaration answers itself", at_(shapes, "def helper", 0, 4), at(shapes, "def helper", 0, 4, 6)],
      ["a CRLF file", at_(crlf, "one + one", 0, 0), at(crlf, "def one", 0, 4, 3)],
      ["an astral character before it on the line (UTF-16)", at_(astral, "= emoji", 0, 2), at(astral, "val emoji", 0, 4, 5)],
    ]
    for (const [name, params, expected] of defs) {
      const r = await c.result("textDocument/definition", params)
      check(`definition: ${name}`, sameLocs(r, [expected]), { found: r, expected })
    }
    const nowhere = await c.request("textDocument/definition", at_(page, "    val local", 0, 0))
    check("definition on no name: null", nowhere.result === null && !nowhere.error, nowhere)
    const outside = await c.request("textDocument/definition", at_(stray, "lonely"))
    check("a file outside every project: null, no error", outside.result === null && !outside.error, outside)
    const outsideRefs = await c.request("textDocument/references", { ...at_(stray, "lonely"), context: { includeDeclaration: true } })
    check("references outside every project: null", outsideRefs.result === null && !outsideRefs.error, outsideRefs)

    // References.
    const total = [at(shapes, "def total", 0, 4, 5), at(appMain, "G.total", 0, 2, 5), at(page, "Geometry.total", 0, 9, 5)]
    let r = await c.result("textDocument/references", { ...at_(shapes, "def total", 0, 4), context: { includeDeclaration: true } })
    check("references of a member across the projects and the shared root", sameLocs(r, total), { found: r, expected: total })
    r = await c.result("textDocument/references", { ...at_(page, "Geometry.total", 0, 9), context: { includeDeclaration: false } })
    check("references without the declaration, asked at a use", sameLocs(r, total.slice(1)), { found: r, expected: total.slice(1) })
    r = await c.result("textDocument/references", { ...at_(shapes, "val depth", 0, 4), context: { includeDeclaration: false } })
    const depthUses = [at(page, "= depth", 0, 2, 5), at(page, "Nested.depth", 0, 7, 5)]
    check("references of a member imported through nested objects", sameLocs(r, depthUses), { found: r, expected: depthUses })
    r = await c.result("textDocument/references", { ...at_(shapes, "def helper", 0, 4), context: { includeDeclaration: false } })
    const helperUses = [at(shapes, "helper(x) +", 0, 0, 6), at(shapes, "+ helper(x)", 0, 2, 6)]
    check("references of a helper called only from an inline body", sameLocs(r, helperUses), { found: r, expected: helperUses })
    r = await c.result("textDocument/references", { ...at_(shapes, "def area", 0, 4), context: { includeDeclaration: true } })
    const shapeArea = [at(shapes, "def area", 0, 4, 4), at(shapes, "s.area", 0, 2, 4), at(page, "shape.area", 0, 6, 4)]
    check("references of an abstract member: the exact symbol", sameLocs(r, shapeArea), { found: r, expected: shapeArea })
    r = await c.result("textDocument/references", { ...at_(vars, "var level", 0, 4), context: { includeDeclaration: true } })
    const levelUses = [at(vars, "var level", 0, 4, 5), at(vars, "level = level", 0, 0, 5), at(vars, "= level + 1", 0, 2, 5), at(vars, "s.level = 3", 0, 2, 5), at(vars, "s.level += 1", 0, 2, 5)]
    check("references of an abstract var: its assignments through the setter", sameLocs(r, levelUses), { found: r, expected: levelUses })
    r = await c.result("textDocument/references", { ...at_(shapes, "class Circle", 0, 6), context: { includeDeclaration: false } })
    const circleUses = [at(appMain, "{Circle", 0, 1, 6), at(appMain, "Circle(2.0)", 0, 0, 6), at(page, "new Circle", 0, 4, 6), at(webMain, "Circle(1.0)", 0, 0, 6)]
    check("references of a class: an import selector, an application, a new", sameLocs(r, circleUses), { found: r, expected: circleUses })

    // Hover.
    const hovers = [
      ["a def", at_(shapes, "def helper", 0, 4), "def shapes.Geometry.helper(x: Int): Int"],
      ["a val", at_(page, "local +"), "val local: Int"],
      ["a class", at_(page, "new Circle", 0, 4), "class shapes.Circle extends Shape"],
      ["a type alias", at_(page, "Area = 1.0"), "type Area = Double"],
    ]
    for (const [name, params, text] of hovers) {
      const h = await c.result("textDocument/hover", params)
      check(`hover: ${name}`, h?.contents?.kind === "markdown" && h.contents.value === "```scala\n" + text + "\n```", h)
    }
    const noHover = await c.request("textDocument/hover", at_(page, "    val local", 0, 0))
    check("hover on no name: null", noHover.result === null && !noHover.error, noHover)

    // Document symbols of a file with nested definitions.
    const syms = await c.result("textDocument/documentSymbol", doc(shapes))
    const geometry = syms?.find((s) => s.name === "Geometry")
    const nested = geometry?.children?.find((s) => s.name === "Nested")
    check(
      "documentSymbol: nested definitions",
      same(syms?.map((s) => s.name), ["Shape", "Circle", "Square", "Point", "Color", "Area", "Geometry", "norm", "Show", "pointShow", "intToPoint", "Exports"]) &&
        geometry.kind === 2 &&
        same(geometry.selectionRange, at(shapes, "object Geometry", 0, 7, 8).range) &&
        geometry.range.start.line === pos(shapes, "object Geometry").line &&
        geometry.range.end.line === pos(shapes, "val depth").line &&
        same(nested?.children?.map((s) => [s.name, s.kind, s.detail]), [["depth", 8, "Int"]]) &&
        same(geometry.children.map((s) => s.name), ["total", "overloaded", "overloaded", "describe", "twice", "helper", "first", "Nested"]) &&
        syms.find((s) => s.name === "Shape").kind === 11 &&
        syms.find((s) => s.name === "Color").children?.map((s) => [s.name, s.kind]).join() === "Red,22,Green,22",
      syms,
    )

    // Implementation.
    r = await c.result("textDocument/implementation", at_(shapes, "trait Shape", 0, 6))
    check("implementation of a trait", sameLocs(r, [at(shapes, "class Circle", 0, 6, 6), at(shapes, "class Square", 0, 6, 6)]), r)
    r = await c.result("textDocument/implementation", at_(shapes, "def area", 0, 4))
    check("implementation of an abstract method", sameLocs(r, [at(shapes, "def area", 1, 4, 4), at(shapes, "def area", 2, 4, 4)]), r)

    // The call hierarchy.
    const items = await c.result("textDocument/prepareCallHierarchy", at_(calls, "def leaf", 0, 4))
    const leaf = items?.[0]
    check("prepareCallHierarchy: a method", items?.length === 1 && leaf.name === "leaf" && same(leaf.selectionRange, at(calls, "def leaf", 0, 4, 4).range) && leaf.uri === uriOf(calls), items)
    const incoming = await c.result("callHierarchy/incomingCalls", { item: leaf })
    const callers = (incoming ?? []).map((i) => [i.from.name, i.fromRanges.map((x) => x.start)]).sort()
    const expectedCallers = [
      ["local", [pos(calls, "leaf() + 1")]],
      ["middle", [pos(calls, "leaf() + x")]],
      ["top", [pos(calls, "leaf()", 3)]],
    ]
    check("incomingCalls: grouped by the lexical owner, a lambda's by its method", same(callers, expectedCallers), { found: callers, expected: expectedCallers })
    const [middle] = await c.result("textDocument/prepareCallHierarchy", at_(calls, "def middle", 0, 4))
    const outgoing = await c.result("callHierarchy/outgoingCalls", { item: middle })
    const callees = (outgoing ?? []).map((o) => [o.to.name, o.fromRanges.map((x) => x.start)]).sort()
    const expectedCallees = [
      ["leaf", [pos(calls, "leaf() + x")]],
      ["local", [pos(calls, "local() +")]],
    ]
    check("outgoingCalls: the nested method's calls are its own", same(callees, expectedCallees), { found: callees, expected: expectedCallees })
    const enclosing = await c.result("textDocument/prepareCallHierarchy", at_(calls, "+ x", 0, 2))
    check("prepareCallHierarchy on no method: the enclosing definition", enclosing?.[0]?.name === "middle", enclosing)

    // The shapes of Findings.scala, each checked where it stands.
    r = await c.result("textDocument/definition", at_(findings, "Int = x\n", 0, 6))
    check("an inline method's body names its own parameter, whatever the calls pass", sameLocs(r, [at(findings, "id(x: Int)", 0, 3, 1)]), r)
    r = await c.result("textDocument/references", { ...at_(findings, "id(x: Int)", 0, 3), context: { includeDeclaration: true } })
    check("references of an inline method's parameter: its body's use", sameLocs(r, [at(findings, "id(x: Int)", 0, 3, 1), at(findings, "Int = x\n", 0, 6, 1)]), r)
    const innerY = at(findings, "inner(y: Int)", 0, 6, 1)
    r = await c.result("textDocument/definition", at_(findings, "inner(y: Int)", 0, 6))
    check("a nested inline method's parameter: its declaration", sameLocs(r, [innerY]), r)
    r = await c.result("textDocument/references", { ...at_(findings, "inner(y: Int)", 0, 6), context: { includeDeclaration: true } })
    check("a nested inline method's parameter: its references", sameLocs(r, [innerY, at(findings, "Int = y\n", 0, 6, 1)]), r)
    const hy = await c.result("textDocument/hover", at_(findings, "inner(y: Int)", 0, 6))
    check("a nested inline method's parameter: its hover", hy?.contents?.value?.includes("y: Int"), hy)
    r = await c.result("textDocument/implementation", at_(findings, "def f(x: Int)", 0, 4))
    check("implementation: an overload of a subclass is no override", same(r, []), r)
    r = await c.result("textDocument/implementation", at_(findings, "def g(x: Int)", 0, 4))
    check("implementation: the override", sameLocs(r, [at(findings, "def g(x: Int)", 1, 4, 1)]), r)
    const groupParam = [at(findings, "(c: Box)", 0, 1, 1), at(findings, "c.v", 0, 0, 1), at(findings, "c.v", 1, 0, 1)]
    r = await c.result("textDocument/references", { ...at_(findings, "(c: Box)", 0, 1), context: { includeDeclaration: true } })
    check("an extension group's parameter: every method's uses", sameLocs(r, groupParam), { found: r, expected: groupParam })
    r = await c.result("textDocument/references", { ...at_(findings, "c.v", 0, 0), context: { includeDeclaration: true } })
    check("an extension group's parameter asked at a use", sameLocs(r, groupParam), { found: r, expected: groupParam })

    // Edits of Findings.scala, open: a method moved by a body edit above it keeps its
    // parameters; a local alias typed again answers from its new text; the members of a local
    // class typed again do not pile up in workspace symbols.
    const findingsText = readFileSync(findings, "utf-8")
    const findingsMoved = join(work, "Findings.moved.scala")
    let fv = 1
    const setFindings = async (text) => {
      writeFileSync(findingsMoved, text)
      const mk = c.mark()
      c.notify(fv === 1 ? "textDocument/didOpen" : "textDocument/didChange", fv === 1 ? { textDocument: { uri: uriOf(findings), languageId: "scala", version: fv, text } } : { textDocument: { uri: uriOf(findings), version: fv }, contentChanges: [{ text }] })
      fv++
      await c.result("textDocument/documentSymbol", doc(findings))
      return mk
    }
    await setFindings(findingsText.replace("def first: Int = 1", "def first: Int =\n    1"))
    const movedAt = (needle, n, delta, len) => ({ uri: uriOf(findings), range: at(findingsMoved, needle, n, delta, len).range })
    const shiftedX = movedAt("shifted(x: Int)", 0, 8, 1)
    r = await c.result("textDocument/definition", { ...doc(findings), position: pos(findingsMoved, "shifted(x: Int): Int = x", 0, 23) })
    check("a moved method's parameter: definition from its body", sameLocs(r, [shiftedX]), r)
    r = await c.result("textDocument/definition", { ...doc(findings), position: pos(findingsMoved, "x = 2") })
    check("a moved method's parameter: definition from a named argument", sameLocs(r, [shiftedX]), r)
    r = await c.result("textDocument/references", { ...doc(findings), position: pos(findingsMoved, "shifted(x: Int)", 0, 8), context: { includeDeclaration: true } })
    check("a moved method's parameter: references", sameLocs(r, [shiftedX, movedAt("shifted(x: Int): Int = x", 0, 23, 1), movedAt("x = 2", 0, 0, 1)]), r)
    await setFindings(findingsText.replace("type T = Int\n    val t: T = 1\n    t", 'type T = String\n    val t: T = "a"\n    t.length'))
    const hoverT = await c.result("textDocument/hover", { ...doc(findings), position: pos(findingsMoved, "type T", 0, 5) })
    const hoverUse = await c.result("textDocument/hover", { ...doc(findings), position: pos(findingsMoved, "t: T", 0, 3) })
    check("a local alias typed again: its declaration and use answer the new text", hoverT?.contents?.value?.includes("type T = String") && hoverUse?.contents?.value?.includes("type T = String"), [hoverT, hoverUse])
    for (let i = 0; i < 6; i++) {
      await setFindings(findingsText.replace("def hello: Int = 1", `def hello: Int = ${i % 2 ? 1 : 2}`))
    }
    r = await c.result("workspace/symbol", { query: "hello" })
    check("workspace symbols after edits of a local class: no copies", same((r ?? []).map((x) => x.name).sort(), ["hello", "helloZ"]), r?.map((x) => x.name))
    await setFindings(findingsText)
    c.notify("textDocument/didClose", doc(findings))

    // A body-only edit inserting lines above a declaration: the declaration's own records and
    // its uses move with it.
    const pageText = readFileSync(page, "utf-8")
    const moved = pageText.replace("    val local = 40\n", "    // two lines\n    // inserted\n    val local = 40\n")
    m = c.mark()
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(page), languageId: "scala", version: 1, text: moved } })
    writeFileSync(page + ".moved", moved)
    const movedFile = page + ".moved"
    const shiftLoc = (l) => ({ uri: uriOf(page), range: l.range })
    r = await c.result("textDocument/definition", { ...doc(page), position: pos(movedFile, "shape: Shape", 0, 7) })
    check("after a body edit: definition from a moved signature", sameLocs(r, [at(shapes, "trait Shape", 0, 6, 5)]), r)
    r = await c.result("textDocument/references", { ...doc(page), position: pos(movedFile, "def area", 0, 4), context: { includeDeclaration: true } })
    const areaUses = [shiftLoc(at(movedFile, "def area", 0, 4, 4)), at(webMain, "Page.area", 0, 5, 4)]
    check("after a body edit: references of a moved declaration", sameLocs(r, areaUses), { found: r, expected: areaUses })
    r = await c.result("textDocument/definition", { ...doc(page), position: pos(movedFile, "local +") })
    check("after a body edit: definition in the edited body", sameLocs(r, [shiftLoc(at(movedFile, "val local", 0, 4, 5))]), r)

    // An overlay with a syntax error: its errors, and its recovered tree typed, so that
    // definition, hover and references answer in it, from it and into it (the header broken,
    // its body kept); then everything back.
    m = c.mark()
    const malformed = moved.replace("def area(shape: Shape): Double", "def area(shape: Shape: Double")
    const malformedFile = page + ".malformed"
    writeFileSync(malformedFile, malformed)
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(page), version: 2 }, contentChanges: [{ text: malformed }] })
    d = await c.waitDiagnostics(m, uriOf(page), (ds) => ds.length > 0)
    check("a text that fails to parse: its errors", d?.length > 0 && d.every((x) => x.severity === 1), d)
    r = await c.result("textDocument/definition", { ...doc(page), position: pos(malformedFile, "local +") })
    check("a text with a syntax error: definition in it", sameLocs(r, [shiftLoc(at(malformedFile, "val local", 0, 4, 5))]), r)
    const brokenHover = await c.result("textDocument/hover", { ...doc(page), position: pos(malformedFile, "local +") })
    check("a text with a syntax error: hover in it", brokenHover?.contents?.value === "```scala\nval local: Int\n```", brokenHover)
    r = await c.result("textDocument/references", { ...doc(page), position: pos(malformedFile, "Geometry.total", 0, 9), context: { includeDeclaration: true } })
    const totalFromBroken = [total[0], total[1], shiftLoc(at(malformedFile, "Geometry.total", 0, 9, 5))]
    check("a text with a syntax error: references from it", sameLocs(r, totalFromBroken), { found: r, expected: totalFromBroken })
    r = await c.result("textDocument/references", { ...at_(shapes, "def area", 0, 4), context: { includeDeclaration: true } })
    const areaIntoBroken = [shapeArea[0], shapeArea[1], shiftLoc(at(malformedFile, "shape.area", 0, 6, 4))]
    check("a text with a syntax error: references into it", sameLocs(r, areaIntoBroken), { found: r, expected: areaIntoBroken })
    rmSync(malformedFile)
    m = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(page), version: 3 }, contentChanges: [{ text: moved }] })
    d = await c.waitDiagnostics(m, uriOf(page))
    check("the text parses again: an empty list", same(d, []), d)
    r = await c.result("textDocument/definition", { ...doc(page), position: pos(movedFile, "local +") })
    check("the text parses again: navigation back", sameLocs(r, [shiftLoc(at(movedFile, "val local", 0, 4, 5))]), r)
    c.notify("textDocument/didClose", doc(page))
    rmSync(movedFile)

    // An empty overlay.
    m = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(other), version: 5 }, contentChanges: [{ text: "" }] })
    r = await c.result("textDocument/documentSymbol", doc(other))
    check("an empty overlay: no symbols", same(r, []), r)
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(other), version: 6 }, contentChanges: [{ text: otherText }] })
    r = await c.result("textDocument/documentSymbol", doc(other))
    check("the overlay back", r?.[0]?.name === "Other", r)

    // A file changed on disk: first without a notification (the idle reconciliation), then with.
    m = c.mark()
    const callsText = readFileSync(calls, "utf-8")
    writeFileSync(calls, callsText.replace("def leaf(): Int = 1", 'def leaf(): Int = "1"'))
    d = await c.waitDiagnostics(m, uriOf(calls), (ds) => ds.length > 0, 40000)
    check("a file changed on disk without a notification", d?.length === 1 && d[0].message.includes("type mismatch"), d)
    m = c.mark()
    writeFileSync(calls, callsText)
    const notified = Date.now()
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(calls), type: 2 }] })
    d = await c.waitDiagnostics(m, uriOf(calls), (ds) => ds.length === 0, 40000)
    check("a file changed on disk with a notification", same(d, []) && Date.now() - notified < 6000, { d, ms: Date.now() - notified })

    // An export regenerated with a jar removed, then restored: the session recreated, the
    // failure on the export, the overlays kept.
    if (jar && existsSync(jar)) {
      const extra = join(ws, "lib/extra.jar")
      mkdirSync(dirname(extra), { recursive: true })
      copyFileSync(jar, extra)
      const mainText = readFileSync(appMain, "utf-8")
      m = c.mark()
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(appMain), languageId: "scala", version: 1, text: mainText.replace("println(circle.area)", "println(circle.area.missing)") } })
      d = await c.waitDiagnostics(m, uriOf(appMain), (ds) => ds.length > 0)
      check("the JVM project: an open text's error", d?.length === 1 && d[0].message.includes("missing"), d)
      const withExtra = () => writeExport(wsExport, wsProjects({ jvm: jvmProject([{ file: "lib/extra.jar" }]) }))
      withExtra()
      await sleep(1500)
      m = c.mark()
      rmSync(extra)
      withExtra()
      d = await c.waitDiagnostics(m, uriOf(wsExport), (ds) => ds.length > 0, 30000)
      check("a jar removed: the failure on the export", d?.length === 1 && d[0].message.includes("extra.jar"), d)
      m = c.mark()
      copyFileSync(jar, extra)
      withExtra()
      d = await c.waitDiagnostics(m, uriOf(wsExport), (ds) => ds.length === 0, 60000)
      check("the jar restored: the failure gone", same(d, []), d)
      d = await c.waitDiagnostics(m, uriOf(appMain), (ds) => ds.length > 0, 60000)
      check("the jar restored: the overlay kept", d?.length === 1 && d[0].message.includes("missing"), d)
      r = await c.result("textDocument/definition", at_(appMain, "G.total", 0, 2))
      check("the recreated session answers", sameLocs(r, [at(shapes, "def total", 0, 4, 5)]), r)
      c.notify("textDocument/didClose", doc(appMain))
    } else {
      console.log("skip the export scenario: no jar given")
    }

    // An export declaring cacheable state of an object the program lacks: the
    // error on the export; then one it has, and the error gone.
    m = c.mark()
    writeExport(wsExport, wsProjects({ js: jsProject({ cacheableState: ["web.Nowhere"] }) }))
    d = await c.waitDiagnostics(m, uriOf(wsExport), (ds) => ds.length > 0, 30000)
    check("cacheable state naming no object: the error on the export", d?.length === 1 && d[0].message.includes("--cacheable-state web.Nowhere: no such object"), d)
    m = c.mark()
    writeExport(wsExport, wsProjects({ js: jsProject({ cacheableState: ["web.Macros"] }) }))
    d = await c.waitDiagnostics(m, uriOf(wsExport), (ds) => ds.length === 0, 30000)
    check("cacheable state naming an object of the program: the error gone", same(d, []), d)

    // A document opened under another spelling than its folder's is answered under its own.
    const crlfReal = pathToFileURL(realpathSync(crlf)).href
    c.notify("textDocument/didOpen", { textDocument: { uri: crlfReal, languageId: "scala", version: 1, text: readFileSync(crlf, "utf-8") } })
    r = await c.result("textDocument/definition", { textDocument: { uri: crlfReal }, position: pos(crlf, "one + one") })
    check("an open document answered under the URI it was opened with", r?.length === 1 && r[0].uri === crlfReal && same(r[0].range, at(crlf, "def one", 0, 4, 3).range), r)
    r = await c.result("textDocument/references", { ...at_(crlf, "def one", 0, 4), context: { includeDeclaration: false } })
    check("a file under the folder answered in the folder's spelling beside it", r?.every((l) => l.uri === crlfReal), r)
    c.notify("textDocument/didClose", { textDocument: { uri: crlfReal } })

    // A project the export gains while the server runs: a document of it opened is served.
    const strayProject = { platform: "js", sources: ["outside"] }
    const childrenNow = () => execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
    writeExport(wsExport, wsProjects({ outside: strayProject }))
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(wsExport), type: 2 }] })
    let m3 = c.mark()
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(stray), languageId: "scala", version: 1, text: 'object Stray:\n  def lonely: Int = "one"\n' } })
    const strayError = await c.waitDiagnostics(m3, uriOf(stray), (ds) => ds.length === 1)
    const straySym = await c.result("workspace/symbol", { query: "Stray" })
    check("a project the export gained: its open document served", strayError?.length === 1 && straySym?.some((s) => s.name === "Stray") && childrenNow() === 3, { strayError, straySym, children: childrenNow() })
    // Removed without a notification: the idle check finds it gone within seconds (the 10 s
    // search is too far), its session ends and its diagnostics are withdrawn.
    m3 = c.mark()
    writeExport(wsExport, wsProjects())
    const strayCleared = await c.waitDiagnostics(m3, uriOf(stray), (ds) => ds.length === 0, 5000)
    check("a project the export lost: its session ended, its diagnostics withdrawn", strayCleared?.length === 0 && childrenNow() === 2, { strayCleared, children: childrenNow() })
    // Gained again with its document still open: served again; then emptied: the child ends,
    // its exit is nobody's failure, and what it published is withdrawn.
    m3 = c.mark()
    writeExport(wsExport, wsProjects({ outside: strayProject }))
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(wsExport), type: 2 }] })
    const strayBack = await c.waitDiagnostics(m3, uriOf(stray), (ds) => ds.length === 1)
    m3 = c.mark()
    writeExport(wsExport, wsProjects({ outside: { platform: "js", sources: [] } }))
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(wsExport), type: 2 }] })
    const strayEmptied = await c.waitDiagnostics(m3, uriOf(stray), (ds) => ds.length === 0, 5000)
    await sleep(500)
    check(
      "a project emptied during the session: its child gone, its diagnostics withdrawn, nothing published on the export",
      strayBack?.length === 1 && strayEmptied?.length === 0 && childrenNow() === 2 && !c.diagnosticsSince(m3).some((p) => p.uri === uriOf(wsExport) && p.diagnostics.length > 0),
      { strayBack, strayEmptied, children: childrenNow(), published: c.diagnosticsSince(m3) },
    )
    c.notify("textDocument/didClose", { textDocument: { uri: uriOf(stray) } })
    writeExport(wsExport, wsProjects())
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(wsExport), type: 2 }] })
    const strayGone = await c.result("workspace/symbol", { query: "Stray" })
    check("a project removed while it ran nothing: gone from the symbols", !strayGone?.some((s) => s.name === "Stray"), strayGone)
    const shut = await c.request("shutdown", null)
    check("shutdown", shut.result === null && !shut.error, shut)
    c.notify("exit", null)
    const code = await Promise.race([c.exited, sleep(10000).then(() => "timeout")])
    check("exit after shutdown: code 0", code === 0, code)
  } catch (e) {
    check("the main workspace ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- An applied function value (docs/TARGETS.md, "The language server") ---------------------------

/** The names of Applied.scala a function value is applied through, beside the ones that answered
 * before (an unapplied use, a receiver, an object): each the needle of the use with its
 * occurrence, the name's offset in it and, where an import renames it, its length, and the
 * declaration it names with the name's length (and its file beside Applied.scala where it is
 * another), or `null` where the name answers nothing; several declarations where the definition
 * is several, the first the one the name refers to. */
const appliedRows = [
  ["a local val", ["h(1)", 0, 0], ["val h =", 0, 4, 1]],
  ["a parameter", ["g(2)", 0, 0], ["(g: Int", 0, 1, 1]],
  ["a Function0", ["zero()", 0, 0], ["val zero", 0, 4, 4]],
  ["a PartialFunction", ["partial(5)", 0, 0], ["val partial", 0, 4, 7]],
  ["a field through this", ["this.field(8)", 0, 5], ["val field", 0, 4, 5]],
  ["a field by its name", ["field(9)", 0, 0], ["val field", 0, 4, 5]],
  ["a field through its object", ["Applied.field(10)", 0, 8], ["val field", 0, 4, 5]],
  ["a field in a default argument", ["= field(10)", 0, 2], ["val field", 0, 4, 5]],
  ["a method returning a function", ["mk()(11)", 0, 0], ["def mk", 0, 4, 2]],
  ["a tuple whose apply gives the function", ["fs(0)(12)", 0, 0], ["val fs", 0, 4, 2]],
  ["an eta-expanded method", ["eta(13)", 0, 0], ["val eta", 0, 4, 3]],
  ["an extension's result", ["1.fun(14)", 0, 2], ["def fun", 0, 4, 3]],
  ["a given's member, the given's own as scalac names it", ["inst.member(15)", 0, 5], ["val member", 1, 4, 6]],
  ["a curried application", ["cur(1)(2)", 0, 0], ["val cur", 0, 4, 3]],
  ["a Function2", ["two(1, 2)", 0, 0], ["val two", 0, 4, 3]],
  ["a Function3 parameter", ["f3(1, 2, 3)", 0, 0], ["f3: (", 0, 0, 2]],
  ["a polymorphic function with a type argument", ["poly[Int](16)", 0, 0], ["val poly", 0, 4, 4]],
  ["a function of a Seq", ["repeated(Seq", 0, 0], ["val repeated", 0, 4, 8]],
  ["a varargs method eta-expanded", ["vari(Seq", 0, 0], ["val vari", 0, 4, 4]],
  ["a function a macro made", ["mf(20)", 0, 0], ["val mf", 0, 4, 2]],
  ["a constructor's val through a prefix", ["holder.hf(21)", 0, 7], ["val hf", 0, 4, 2]],
  ["a constructor's val through this", ["this.hf(x)", 0, 5], ["val hf", 0, 4, 2]],
  ["a constructor's val by its name", ["+ hf(x)", 0, 2], ["val hf", 0, 4, 2]],
  ["inside a colon block", ["h(23)", 0, 0], ["val h =", 0, 4, 1]],
  ["a val of a val", ["n(24)", 0, 0], ["val n", 0, 4, 1]],
  ["a using parameter's function member", ["ctx.f(2)", 0, 4], ["def f: Int", 0, 4, 1]],
  ["in a pattern guard", ["f(y)", 0, 0], ["(f: Int => Boolean", 0, 1, 1]],
  ["a by-name function parameter", ["bn(11)", 0, 0], ["(bn: =>", 0, 1, 2]],
  ["inside an interpolation", ["h(17)", 0, 0], ["(h: Int => Int): String", 0, 1, 1]],
  ["a parameter shadowing an object", ["same(18)", 0, 0], ["(same: Int", 0, 1, 4]],
  ["the report's val of a function alias in a colon block", ["runAction(1)", 0, 0], ["val runAction", 0, 4, 9]],
  ["the report's parameter of a generic function type", ["mkArgs(current)", 0, 0], ["mkArgs: CAT", 0, 0, 6]],
  ["an inline method returning a function", ["factory()(33)", 0, 0], ["def factory()", 0, 4, 7]],
  ["a tuple named as its element's field", ["_1(0)(32)", 0, 0], ["val _1 =", 0, 4, 2]],
  ["a tuple an import renames", ["twin(0)(40)", 0, 0, 4], ["val pair", 0, 4, 4]],
  ["a tuple named `_1` an import renames", ["first(1)(41)", 0, 0, 5], ["val _1:", 0, 4, 2]],
  ["a tuple of tuples", ["nested(0)(1)(42)", 0, 0], ["val nested", 0, 4, 6]],
  ["a tuple of tuples named as an element's field", ["_2(1)(0)(43)", 0, 0], ["val _2", 0, 4, 2]],
  ["(as before) an import's selector", ["{pair as twin", 0, 1], ["val pair", 0, 4, 4]],
  ["(as before) an import's rename", ["pair as twin", 0, 8, 4], ["val pair", 0, 4, 4]],
  ["(as before) an import's selector named `_1`", ["_1 as first", 0, 0], ["val _1:", 0, 4, 2]],
  ["(as before) an import's rename of `_1`", ["_1 as first", 0, 6, 5], ["val _1:", 0, 4, 2]],
  ["(as before) an unapplied use", ["val n = h", 0, 8], ["val h =", 0, 4, 1]],
  ["(as before) an unapplied parameter", ["val h = g", 0, 8], ["(g: Int", 0, 1, 1]],
  ["(as before) a tuple's element", ["(h, g)", 0, 1], ["val h =", 0, 4, 1]],
  ["(as before) a tuple's other element", ["(h, g)", 0, 4], ["(g: Int", 0, 1, 1]],
  ["(as before) a SAM's receiver", ["sam.run(6)", 0, 0], ["val sam", 0, 4, 3]],
  ["(as before) a SAM's method", ["sam.run(6)", 0, 4], ["def run(x: Int)", 0, 4, 3]],
  ["(as before) a SAM with apply", ["ap(7)", 0, 0], ["val ap", 0, 4, 2]],
  ["(as before) an apply with a default argument", ["dd(22)", 0, 0], ["dd: Defaulted", 0, 0, 2]],
  ["(as before) an explicit apply's receiver", ["h.apply(3)", 0, 0], ["val h =", 0, 4, 1]],
  ["(as before) an infix apply's receiver", ["h apply 4", 0, 0], ["val h =", 0, 4, 1]],
  ["(as before) an update's receiver", ["f(1) = 2", 0, 0], ["(f: Up", 0, 1, 1]],
  ["(as before) an object's path", ["Applied.field(10)", 0, 0], ["object Applied", 0, 7, 7]],
  ["(as before) a given", ["inst.member(15)", 0, 0], ["given inst", 0, 6, 4]],
  ["(as before) an unshadowed object applied", ["same(19)", 0, 0], ["object same", 0, 7, 4]],
  ["(as before) the receiver of an explicit apply, a val named `apply`", ["apply.apply(30)", 0, 0], ["val apply:", 0, 4, 5]],
  ["(as before) the receiver of a tuple's `head`, a val named `head`", ["head.head(31)", 0, 0], ["val head", 0, 4, 4]],
  ["(as before) the receiver of a tuple's explicit apply, a val named `apply`", ["apply.apply(0)(34)", 0, 0], ["val apply =", 0, 4, 5]],
  ["(as before, nothing) the `head` of a tuple named `head`", ["head.head(31)", 0, 5], null],
  ["(as before, nothing) the explicit apply of a tuple named `apply`", ["apply.apply(0)(34)", 0, 6], null],
  ["(as before) an ascribed receiver of an explicit apply", ["(apply: Int => Int).apply(44)", 0, 1], ["val apply: Int => Int = x => x + 2", 0, 4, 5]],
  ["(as before) a block's result before a tuple's `head`", ["{ head }.head(45)", 0, 2], ["val head = (f, (x", 0, 4, 4]],
  ["(as before, nothing) the `head` after a block's result", ["{ head }.head(45)", 0, 9], null],
  ["a tuple an import renames, beside a value of its original name", ["head(0)(54)", 0, 0, 4], ["val pair: (Int => Int, Int => Int) = (x => x, x => x)", 0, 4, 4]],
  ["a function value an import renames", ["apply(55)", 0, 0, 5], ["val g: Int => Int = x => x\n", 0, 4, 1]],
  ["(as before) an import's selector, `pair`", ["{pair as head", 0, 1], ["val pair: (Int => Int, Int => Int) = (x => x, x => x)", 0, 4, 4]],
  ["(as before) an import's rename, `head`", ["pair as head", 0, 8, 4], ["val pair: (Int => Int, Int => Int) = (x => x, x => x)", 0, 4, 4]],
  ["(as before) an import's selector, `g`", ["g as apply", 0, 0], ["val g: Int => Int = x => x\n", 0, 4, 1]],
  ["(as before) an import's rename, `apply`", ["g as apply", 0, 5, 5], ["val g: Int => Int = x => x\n", 0, 4, 1]],
  ["(as before) a tuple's `head` receiver, an import renaming another value to `head`", ["pair.head(50)", 0, 0], ["val pair = (f, f)", 0, 4, 4]],
  ["(as before, nothing) the tuple's `head` beside that import", ["pair.head(50)", 0, 5], null],
  ["(as before) an explicit apply's receiver, an import renaming another value to `apply`", ["g.apply(51)", 0, 0], ["val g: Int => Int = x => x + 1", 0, 4, 1]],
  ["(as before) a tuple of tuples, its written member's receiver", ["tuples._1(0)(52)", 0, 0], ["val tuples", 0, 4, 6]],
  ["(as before) a tuple of tuples, another member's receiver", ["tuples._2(1)(53)", 0, 0], ["val tuples", 0, 4, 6]],
  ["a value a given `Conversion` makes a function of", ["config(60)", 0, 0], ["(config: Config", 0, 1, 6]],
  ["a value an implicit method converts to a function", ["spec(61)", 0, 0], ["spec: Spec,", 0, 0, 4]],
  ["a tuple of partial functions", ["pfs(0)(62)", 0, 0], ["val pfs", 0, 4, 3]],
  ["a tuple of SAM values", ["aps(1)(63)", 0, 0], ["val aps", 0, 4, 3]],
  ["a tuple of SAM values an import renames", ["samFirst(0)(64)", 0, 0, 8], ["val _1: (Ap, Ap)", 0, 4, 2]],
  ["(as before) an import's selector, `_1` of SAM values", ["_1 as samFirst", 0, 0], ["val _1: (Ap, Ap)", 0, 4, 2]],
  ["(as before) an import's rename, `samFirst`", ["_1 as samFirst", 0, 6, 8], ["val _1: (Ap, Ap)", 0, 4, 2]],
  ["an object's inline `apply`: the object, as scalac names it and as for an object's own `apply`", ["handlers(\"a\")(65)", 0, 0, 8], ["object handlers", 0, 7, 8]],
  ["a function returning a SAM value", ["ff(66)(67)", 0, 0], ["ff: Int => Ap", 0, 0, 2]],
  ["a member spelled as its qualifier", ["handler.handler(68)", 0, 8], ["val handler:", 0, 4, 7]],
  ["(as before) the qualifier of a member spelled as it", ["handler.handler(68)", 0, 0], ["handler: Handler", 0, 0, 7]],
  ["a method spelled as its qualifier", ["maker.maker()(69)", 0, 6], ["def maker()", 0, 4, 5]],
  ["(as before) the qualifier of a method spelled as it", ["maker.maker()(69)", 0, 0], ["maker: Maker", 0, 0, 5]],
  ["a tuple of values an implicit def converts, spelled like it", ["spec(0)(71)", 0, 0], ["val spec = (s, s)", 0, 4, 4]],
  ["a tuple of values a given converts, spelled like it", ["cfg(1)(72)", 0, 0], ["val cfg = (c, c)", 0, 4, 3]],
  ["a method whose result an implicit def converts, spelled like it", ["spec()(73)", 0, 0], ["def spec(): Spec", 0, 4, 4]],
  ["a method whose result a given converts, spelled like it", ["cfg()(74)", 0, 0], ["def cfg(): Cfg", 0, 4, 3]],
  ["a parameter an implicit def converts, spelled like it", ["spec(75)", 0, 0], ["plain(spec: Spec", 0, 6, 4]],
  ["a parameter a given converts, spelled like it", ["cfg(76)", 0, 0], ["plain(spec: Spec, cfg", 0, 18, 3]],
  ["a field an implicit def converts", ["spec(77)", 0, 0], ["val spec: Spec, val cfg", 0, 4, 4]],
  ["a field an implicit def converts, through this", ["this.spec(78)", 0, 5], ["val spec: Spec, val cfg", 0, 4, 4]],
  ["a field a given converts", ["cfg(79)", 0, 0], ["val cfg: Cfg):", 0, 4, 3]],
  ["a field a given converts, through this", ["this.cfg(80)", 0, 5], ["val cfg: Cfg):", 0, 4, 3]],
  ["a parameter an implicit def converts, in its object's scope", ["spec(81)", 0, 0], ["go(spec: Spec", 0, 3, 4]],
  ["a parameter a given converts, in its object's scope", ["cfg(82)", 0, 0], ["go(spec: Spec, cfg", 0, 15, 3]],
  ["a parameter a given converts to a SAM value", ["ob(83)", 0, 0], ["ob: Ob,", 0, 0, 2]],
  ["a parameter an implicit class gives an apply", ["ri(84)", 0, 0], ["ri: Ri,", 0, 0, 2]],
  ["a parameter an extension gives an apply", ["ex(85)", 0, 0], ["ex: Ex)", 0, 0, 2]],
  ["an object's val an implicit def converts, spelled like it", ["UseC.spec(86)", 0, 5], ["val spec: Spec = Spec(1)", 0, 4, 4]],
  ["a tuple of values an implicit def converts", ["specs(0)(87)", 0, 0], ["val specs", 0, 4, 5]],
  ["a tuple of values a given converts", ["cfgs(1)(88)", 0, 0], ["val cfgs", 0, 4, 4]],
  ["an implicit def called on its object", ["Spec.spec(s)(89)", 0, 5], ["implicit def spec(s: Spec)", 0, 13, 4]],
  ["a given conversion called on its object", ["Cfg.cfg(c)(90)", 0, 4], ["given cfg:", 0, 6, 3]],
  ["a given conversion called on its object before an explicit apply", ["Cfg.cfg(c).apply(91)", 0, 4], ["given cfg:", 0, 6, 3]],
  ["an implicit def of using parameters alone", ["ord(92)", 0, 0], ["implicit def ord", 0, 13, 3]],
  ["a method whose result an implicit def converts", ["mkSpec()(93)", 0, 0], ["def mkSpec()", 0, 4, 6]],
  ["a val a given converts", ["cfgVal(94)", 0, 0], ["val cfgVal", 0, 4, 6]],
  ["a method whose result an implicit def converts, through its object", ["UseC.mkSpec()(95)", 0, 5], ["def mkSpec()", 0, 4, 6]],
  ["a by-name parameter's plain use", ["bv + bv", 0, 0], ["(bv: =>", 0, 1, 2]],
  ["a by-name parameter's plain use, the second", ["bv + bv", 0, 5], ["(bv: =>", 0, 1, 2]],
  ["an array's element", ["arr(0) +", 0, 0], ["arr: Array[Int], str", 0, 0, 3]],
  ["a string's character", ["str(1)", 0, 0], ["str: String, grid", 0, 0, 3]],
  ["an array of arrays' element", ["grid(0)(1)", 0, 0], ["grid: Array", 0, 0, 4]],
  ["a transparent inline method's lambda applied at once", ["adder(3)(y)", 0, 0], ["def adder", 0, 4, 5]],
  ["an inline method's lambda applied at once", ["doubler()(y)", 0, 0], ["def doubler", 0, 4, 7]],
  ["a macro's lambda applied at once", ["AppliedMacro.make(26)", 0, 13], ["def make", 0, 4, 4, "AppliedMacro.scala"]],
  ["a generator's binder spelled as `f`, at its pattern", ["f(fo)", 0, 0], ["f <- fns", 0, 0, 1]],
  ["a generator's binder spelled as `fo`, at its pattern", ["f(fo)", 0, 2], ["fo <- List", 0, 0, 2]],
  ["a given's own method, as scalac names it", ["counter.count(5)", 0, 8], ["def count(x: Int): Int = x", 0, 4, 5]],
  ["(as before) a given", ["counter.count(5)", 0, 0], ["given counter", 0, 6, 7]],
  ["an ascribed object's member: the base's, and the object's that the call runs", ["(Square: Shape).area(2)", 0, 16], [["def area(x: Int): Int\n", 0, 4, 4], ["def area(x: Int): Int = x * x", 0, 4, 4]]],
  ["(as before) the ascribed object", ["(Square: Shape).area(2)", 0, 1], ["object Square", 0, 7, 6]],
  ["(as before) the object's own member", ["Square.area(3)", 0, 7], ["def area(x: Int): Int = x * x", 0, 4, 4]],
  ["a nullary method beside a conversion of its name, its result converted", ["Spec2.spec2()(4)", 0, 6], ["def spec2(): Spec2", 0, 4, 5]],
  ["(as before) the conversion called on its object", ["Spec2.spec2(Spec2(2))(5)", 0, 6], ["implicit def spec2", 0, 13, 5]],
  ["(as before) the receiver of a written apply", ["w.apply(6)", 0, 0], ["w: Int => Int", 0, 0, 1]],
  ["(as before) the receiver of a written apply on a result", ["curried(1).apply(7)", 0, 0], ["curried: Int => Int => Int", 0, 0, 7]],
  ["(as before) the receiver of a written apply a list follows", ["curried.apply(2)(8)", 0, 0], ["curried: Int => Int => Int", 0, 0, 7]],
  ["(as before) the macro unapplied", ["mf = AppliedMacro.make", 0, 18], ["def make", 0, 4, 4, "AppliedMacro.scala"]],
  ["(as before) the object of its own member", ["Square.area(3)", 0, 0], ["object Square", 0, 7, 6]],
  ["(as before) a written infix call of a conversion, its argument spelled like it", ["Conv conv conv", 0, 5], ["implicit infix def conv", 0, 19, 4]],
  ["(as before) the argument of that call", ["Conv conv conv", 0, 10], ["(conv: V", 0, 1, 4]],
  ["(as before) a written infix call of a conversion", ["Conv conv other", 0, 5], ["implicit infix def conv", 0, 19, 4]],
  ["(as before) the argument of the other call", ["Conv conv other", 0, 10], ["other: V", 0, 0, 5]],
  ["(as before) a conversion's method value", ["= Conv.conv", 0, 7], ["implicit infix def conv", 0, 19, 4]],
  ["(as before) an extension on an ascribed object's base: the extension alone", ["(PlainObj: Plain).pf(2)", 0, 18], ["extension (p: Plain) def pf", 0, 25, 2]],
  ["(as before) the object ascribed its base", ["(PlainObj: Plain).pf(2)", 0, 1], ["object PlainObj", 0, 7, 8]],
  ["(as before, nothing) a source extension's `apply` applied twice", ["e3(1)(2)", 0, 0], null],
]

/** The `apply` written after a function value: `Function1.apply`, whose declaration is in
 * scala-library's sources where the class path has it and nowhere in the lean std (the function
 * classes are the typer's own); each the needle of the `apply` and its offset. */
const appliedFunctionApplies = [
  ["an explicit apply's `apply`", ["h.apply(3)", 0, 2]],
  ["an infix apply's `apply`", ["h apply 4", 0, 2]],
  ["the explicit apply of a val named `apply`", ["apply.apply(30)", 0, 6]],
  ["the explicit apply after an ascribed qualifier", ["(apply: Int => Int).apply(44)", 0, 20]],
  ["the explicit apply beside an import renaming another value to `apply`", ["g.apply(51)", 0, 2]],
  ["the explicit apply of a conversion's result", ["Cfg.cfg(c).apply(91)", 0, 11]],
  ["the explicit apply of a parameter", ["w.apply(6)", 0, 2]],
  ["the explicit apply of a call's result", ["curried(1).apply(7)", 0, 11]],
  ["the explicit apply of a curried value, a list after it", ["curried.apply(2)(8)", 0, 8]],
]

/** Written members of a tuple that the element reads of their applications follow, which name
 * the std's or scala-library's tuple class: the use as in `appliedRows`, and the hover's text. */
const appliedMembers = [
  ["a written `_1` of a Tuple3 whose element is read", ["tuples._1(0)(52)", 0, 7], "Tuple3._1"],
  ["a written `_2` of a Tuple3 whose element is read", ["tuples._2(1)(53)", 0, 7], "Tuple3._2"],
]

/** `ask(kind, file, needle, n, delta)` of a language server's client: definition, hover or
 * references (the declaration included). */
const lspAsk = (c) => (kind, file, needle, n, delta) => {
  const params = at_(file, needle, n, delta)
  return c.result(`textDocument/${kind}`, kind === "references" ? { ...params, context: { includeDeclaration: true } } : params)
}

/** The same of a check session's commands. */
const sessionAsk = (s) => async (kind, file, needle, n, delta) => {
  const [text, offset] = offsetIn(file, needle, n, delta)
  return (await s.ask(`${kind} ${file} ${Buffer.byteLength(text.slice(0, offset))}${kind === "references" ? " 1" : ""}`)).result
}

/** Definition and hover at every row of `appliedRows` in `file` (the hover the declaration's),
 * and the references of every declaration: the rows' uses of it; then the written `apply`s of
 * `appliedFunctionApplies`, whose declaration is scala-library's where `library` says the session
 * reads it. */
async function appliedChecks(where, file, ask, library) {
  const uses = new Map()
  const fileOf = (decl) => (decl[4] ? join(dirname(file), decl[4]) : file)
  for (const [shape, use, decls] of appliedRows) {
    const r = await ask("definition", file, ...use)
    const h = await ask("hover", file, ...use)
    if (decls === null) {
      check(`${where}: ${shape}: no definition, no hover`, r === null && h === null, { r, h })
      continue
    }
    const all = Array.isArray(decls[0]) ? decls : [decls]
    const decl = all[0]
    const expected = all.map((d) => at(fileOf(d), ...d.slice(0, 4)))
    check(`${where}: definition, ${shape}`, sameLocs(r, expected), { found: r, expected })
    const key = JSON.stringify(decl)
    if (!uses.has(key)) uses.set(key, { hover: await ask("hover", fileOf(decl), decl[0], decl[1], decl[2]), locs: [expected[0]] })
    const declared = uses.get(key)
    const written = at(file, use[0], use[1], use[2], use[3] ?? decl[3])
    declared.locs.push(written)
    check(`${where}: hover, ${shape}: the declaration's`, h?.contents?.value?.startsWith("```scala\n") && same(h.contents, declared.hover?.contents) && same(h.range, written.range), { use: h, declaration: declared.hover })
  }
  for (const [shape, use, text] of appliedMembers) {
    const h = await ask("hover", file, ...use)
    check(`${where}: hover, ${shape}: the tuple class's member`, h?.contents?.value?.includes(text) && same(h.range, at(file, use[0], use[1], use[2], 2).range), h)
  }
  for (const [key, { locs }] of uses) {
    const decl = JSON.parse(key)
    const r = await ask("references", fileOf(decl), decl[0], decl[1], decl[2])
    check(`${where}: references of \`${decl[0]}\`: its applied uses among the others`, sameLocs(r, locs), { found: r, expected: locs })
  }
  const applies = appliedFunctionApplies.map(([, use]) => at(file, ...use, 5))
  for (const [shape, use] of appliedFunctionApplies) {
    const r = await ask("definition", file, ...use)
    const h = await ask("hover", file, ...use)
    const inLibrary = r?.length === 1 && r[0].uri.endsWith("/scala-library-3.8.4-sources.jar/scala/Function1.scala") && r[0].range.start.line === 69 && r[0].range.start.character === 6
    check(`${where}: definition, ${shape}: ${library ? "scala-library's `Function1.apply`" : "none, the lean std declaring no function class"}`, library ? inLibrary : r === null, r)
    check(`${where}: hover, ${shape}: \`Function1.apply\``, h?.contents?.value === "```scala\ndef scala.Function1.apply(v1: T1): R\n```" && same(h.range, at(file, ...use, 5).range), h)
    const refs = await ask("references", file, ...use)
    const own = (refs ?? []).filter((l) => l.uri === uriOf(file))
    check(`${where}: references, ${shape}: the written \`apply\`s of function values, nothing else of the file`, sameLocs(own, applies), { found: own, expected: applies })
  }
}

/** The applied function values of the JavaScript project (the lean std), the JVM one and the
 * JavaScript one on scala-library (its std verified), and the call hierarchy around them; then a
 * check session's records of them through a body edit and an edit that moves every position. */
async function appliedValues() {
  encoding = "utf-16"
  const c = new Client(ws, { TEQ_CACHE_DIR: join(work, "applied-cache") })
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(ws), capabilities: { general: { positionEncodings: ["utf-16"] } } })
    c.notify("initialized", {})
    const std = {}
    for (const project of ["js", "jvm", "jsstd"]) {
      const file = appliedIn[project]
      std[project] = await c.result("textDocument/definition", at_(file, "PartialFunction[Int", 0, 0))
      await appliedChecks(`the ${project} project`, file, lspAsk(c), project !== "js")
      // The written name of a function value applied is no call: the hierarchy there is the
      // enclosing method's; a method whose result is applied is a call of it.
      const [item, ...more] = (await c.result("textDocument/prepareCallHierarchy", at_(file, "h(1)"))) ?? []
      check(`the ${project} project: prepareCallHierarchy on an applied function value: the enclosing method`, more.length === 0 && item?.name === "run" && same(item.selectionRange, at(file, "def run(g", 0, 4, 3).range), item)
      const out = item ? await c.result("callHierarchy/outgoingCalls", { item }) : null
      const callees = (out ?? []).map((o) => [o.to.name, o.fromRanges.map((x) => x.start)])
      const callee = (name) => callees.find(([n]) => n === name)?.[1]
      // An `apply` written after a function value is a call of `Function1.apply`, an edge where
      // scala-library's source declares it (none in the lean std).
      const library = project !== "js"
      const writtenApply = (calls, uses) => {
        const items = (calls ?? []).filter((o) => o.to.name === "apply")
        return library ? items.length === 1 && items[0].to.detail === "scala.Function1" && same(items[0].fromRanges.map((x) => x.start), uses.map(([needle, delta]) => pos(file, needle, 0, delta))) : items.length === 0
      }
      check(
        `the ${project} project: outgoingCalls: the methods whose results are applied, no function value, an \`apply\` the written ones alone`,
        same(callee("mk"), [pos(file, "mk()(11)")]) && same(callee("fun"), [pos(file, "1.fun(14)", 0, 2)]) && (out ?? []).every((o) => o.to.kind !== 13) && writtenApply(out, [["h.apply(3)", 2], ["h apply 4", 2]]),
        out,
      )
      const outgoingOf = async (method) => {
        const [decl] = (await c.result("textDocument/prepareCallHierarchy", at_(file, `def ${method}(`, 0, 4))) ?? []
        return decl ? c.result("callHierarchy/outgoingCalls", { item: decl }) : null
      }
      const fromConverted = await outgoingOf("converted")
      check(`the ${project} project: outgoingCalls: a method whose result is applied, no conversion a value was applied through`, (fromConverted ?? []).some((o) => o.to.name === "maker") && !(fromConverted ?? []).some((o) => ["spec", "config"].includes(o.to.name)), fromConverted)
      // No call of a conversion spelled like the written name: the methods alone.
      const fromTuples = await outgoingOf("tuples")
      const fromMethods = await outgoingOf("methods")
      const callItem = (o) => [o.to.name, o.to.selectionRange.start, o.fromRanges.map((x) => x.start)]
      check(
        `the ${project} project: outgoingCalls: no conversion spelled like the value it converts, the methods called`,
        same(fromTuples, []) && same((fromMethods ?? []).map(callItem), [["spec", pos(file, "def spec(): Spec", 0, 4), [pos(file, "spec()(73)")]], ["cfg", pos(file, "def cfg(): Cfg", 0, 4), [pos(file, "cfg()(74)")]]]),
        { fromTuples, fromMethods },
      )
      const fromCollisions = await outgoingOf("collisions")
      check(
        `the ${project} project: outgoingCalls: an inline method whose result is applied, and the written \`apply\``,
        same((fromCollisions ?? []).filter((o) => o.to.name !== "apply").map((o) => [o.to.name, o.fromRanges.map((x) => x.start)]), [["factory", [pos(file, "factory()(33)")]]]) && writtenApply(fromCollisions, [["apply.apply(30)", 6]]),
        fromCollisions,
      )
      for (const [name, needle, use, useDelta, caller] of [["mk", "def mk", "mk()(11)", 0, "run"], ["fun", "def fun", "1.fun(14)", 2, "run"], ["factory", "def factory()", "factory()(33)", 0, "collisions"]]) {
        const [decl] = (await c.result("textDocument/prepareCallHierarchy", at_(file, needle, 0, 4))) ?? []
        const incoming = decl ? await c.result("callHierarchy/incomingCalls", { item: decl }) : null
        const callers = (incoming ?? []).map((i) => [i.from.name, i.fromRanges.map((x) => x.start)])
        check(`the ${project} project: incomingCalls of \`${name}\`: its applied call`, same(callers, [[caller, [pos(file, use, 0, useDelta)]]]), incoming)
      }
    }
    const inStd = (r) => r?.length === 1 && r[0].uri.endsWith("/scala-library-3.8.4-sources.jar/scala/PartialFunction.scala")
    check("the projects' std: scala-library's `PartialFunction` in the JVM project and in the one its flags select, not in the lean one", inStd(std.jvm) && inStd(std.jsstd) && !(std.js ?? []).some((l) => l.uri.includes("scala-library")), std)
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the applied function values' scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
  // A check session's records through a body edit and an edit that moves every position: the
  // answers right where the names now stand, the index's records as many, its journal settled.
  const dir = join(work, "applied-resident")
  mkdirSync(dir, { recursive: true })
  for (const name of ["Applied.scala", "AppliedMacro.scala"]) copyFileSync(join(dirname(appliedIn.js), name), join(dir, name))
  const file = realpathSync(join(dir, "Applied.scala"))
  const text = readFileSync(file, "utf-8")
  const s = session(realpathSync(dir))
  try {
    const first = await s.next()
    check("a check session of the applied function values: a clean build", first.ok && (first.diagnostics ?? []).length === 0, JSON.stringify(first).slice(0, 300))
    const stats = []
    for (const [label, edited] of [["fresh", text], ["a body edit", text.replace("total + explicit + inf + n(24)", "total + explicit + inf + n(24) + 1")], ["every position moved", "\n" + text]]) {
      if (label !== "fresh") {
        writeFileSync(file, edited)
        const built = await s.ask(`build ${file}\n`)
        check(`a check session of the applied function values, ${label}: an incremental build`, built.ok && built.incremental, JSON.stringify(built).slice(0, 300))
      }
      await appliedChecks(`a check session, ${label}`, file, sessionAsk(s), false)
      stats.push((await s.ask("index-stats")).result)
    }
    check("a check session of the applied function values: the records as many after each edit, the journal settled", stats.every((x) => x?.records === stats[0]?.records && x?.journal === 0), stats)
  } catch (e) {
    check("the applied function values' check session ran to its end", false, e.stack)
  } finally {
    s.child.stdin.end("quit\n")
  }
}

// --- Completion (docs/TARGETS.md, "The language server") ------------------------------------------

/** The completion list at the `|` of `marked`, the document's text with the cursor, sent as the
 * document's next version first. */
async function completeIn(c, file, marked, version) {
  const offset = marked.indexOf("|")
  const text = marked.slice(0, offset) + marked.slice(offset + 1)
  c.notify("textDocument/didChange", { textDocument: { uri: uriOf(file), version }, contentChanges: [{ text }] })
  return [text, offset, await c.result("textDocument/completion", { textDocument: { uri: uriOf(file) }, position: positionAt(text, offset) })]
}
const labels = (list) => (list?.items ?? []).map((i) => i.label)
const itemOf = (list, label) => (list?.items ?? []).find((i) => i.label === label)

async function completion() {
  encoding = "utf-16"
  const c = new Client(ws)
  try {
    const init = await c.result("initialize", {
      processId: null,
      rootUri: uriOf(ws),
      capabilities: { general: { positionEncodings: ["utf-16"] }, textDocument: { completion: { completionItem: { insertReplaceSupport: true } } } },
    })
    const provider = init.capabilities.completionProvider
    check("initialize: completion with a trigger on `.` and resolve", same(provider?.triggerCharacters, ["."]) && provider?.resolveProvider === true, init.capabilities)
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(completing), languageId: "scala", version: 1, text: completingText } })
    let v = 1
    const edit = (needle, insert) => completingText.replace(needle, insert)
    const hoverBefore = await c.result("textDocument/hover", at_(completing, "holder.own", 0, 0))
    // Members of an instance.
    let [text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "holder.|own(1) + param"), ++v)
    let ls = labels(list)
    check("member completion: own, inherited, overloads as two items, an extension, the universal members", ls.filter((l) => l === "own").length === 2 && ls.includes("inherited") && ls.includes("foobar") && ls.includes("`my value`") && ls.includes("toString") && !list.isIncomplete, ls)
    check("member completion: a private member and a protected one from outside hidden", !ls.includes("secretly") && !ls.includes("guarded"), ls)
    const own = list.items.filter((i) => i.label === "own")
    check("an overloaded method: two items ordered by their signatures, kind Method, the signature as detail", same(own.map((i) => i.detail), ["(x: Int): Int", "(x: String): Int"]) && own.every((i) => i.kind === 2), own)
    check("a val member: kind Field, its type as detail", itemOf(list, "`my value`")?.kind === 5 && itemOf(list, "`my value`")?.detail === "Int", itemOf(list, "`my value`"))
    const range = (from, to) => ({ start: positionAt(text, from), end: positionAt(text, to) })
    check("an empty prefix after `.`: the insert and replace ranges", same(itemOf(list, "inherited")?.textEdit, { newText: "inherited", insert: range(offset, offset), replace: range(offset, offset + 3) }), itemOf(list, "inherited"))
    check("a client without snippets: every item its name alone, no insertTextFormat", list.items.every((i) => i.insertTextFormat === undefined && i.textEdit.newText === i.label), list.items.filter((i) => i.insertTextFormat !== undefined || i.textEdit.newText !== i.label))
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "point.no|rm + holder.own(1) + param"), ++v)
    check("member completion: an extension in scope", labels(list).includes("norm") && itemOf(list, "norm")?.kind === 2, labels(list))
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "5.|x + holder.own(1) + param"), ++v)
    ls = labels(list)
    check("member completion: the members through a conversion, after the receiver's own", ls.includes("x") && ls.includes("y") && ls.indexOf("x") > ls.indexOf("toInt") && /intToPoint/.test(itemOf(list, "x")?.detail), ls)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "Holder.fromC| + holder.own(1) + param"), ++v)
    check("member completion: the companion's member", same(labels(list), ["fromCompanion"]), labels(list))
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "holder.fo|obar + param"), ++v)
    check("`fo|obar`: the insert range the prefix, the replace range the whole name", same(itemOf(list, "foobar")?.textEdit, { newText: "foobar", insert: range(offset - 2, offset), replace: range(offset - 2, offset + 4) }), list)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "holder.`my v|`"), ++v)
    check("a backquoted name: the backquotes replaced with the label's", same(itemOf(list, "`my value`")?.textEdit, { newText: "`my value`", insert: range(offset - 5, offset), replace: range(offset - 5, offset + 1) }), list)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "holder.|\n    param"), ++v)
    check("`recv.` at the end of a line", labels(list).includes("inherited"), labels(list))
    // Names in scope.
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "p|"), ++v)
    ls = labels(list)
    check("scope completion: a local and a parameter first, then the std's, prefix-filtered", ls[0] === "param" && ls.includes("point") && ls.includes("println") && ls.every((l) => l.toLowerCase().startsWith("p")) && itemOf(list, "param").kind === 6, ls)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "e|"), ++v)
    check("scope completion: an enclosing member", labels(list).includes("enclosing") && labels(list).indexOf("enclosing") < labels(list).indexOf("else"), labels(list))
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "Li|"), ++v)
    ls = labels(list)
    check("scope completion: a named import, the std's List", ls.includes("ListBuffer") && ls.includes("List") && ls.indexOf("ListBuffer") < ls.indexOf("List"), ls)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "Cir|"), ++v)
    check("scope completion: a wildcard import", labels(list).includes("Circle"), labels(list))
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "Pag|"), ++v)
    check("scope completion: a member of the package in another file", labels(list).includes("Page") && itemOf(list, "Page")?.kind === 9, labels(list))
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "whi|"), ++v)
    check("scope completion: a keyword", same(itemOf(list, "while")?.kind, 14), list)
    // Types, imports, `new`.
    ;[text, offset, list] = await completeIn(c, completing, edit("val point = Point(1, 2)", "val point: Li| = ???"), ++v)
    ls = labels(list)
    check("type completion: types alone", ls.includes("List") && ls.includes("ListBuffer") && !ls.includes("println"), ls)
    ;[text, offset, list] = await completeIn(c, completing, edit("import scala.collection.mutable.ListBuffer", "import shapes.Ci|"), ++v)
    check("import completion: a member of the package", same(labels(list), ["Circle"]), labels(list))
    ;[text, offset, list] = await completeIn(c, completing, edit("val point = Point(1, 2)", "val point = new Ci|"), ++v)
    check("after `new`: the classes, as constructors", same(labels(list), ["Circle"]) && itemOf(list, "Circle")?.kind === 4, list)
    ;[text, offset, list] = await completeIn(c, completing, edit("val point = Point(1, 2)", "val point = new Sha|"), ++v)
    check("after `new`: a trait for an anonymous class", labels(list).includes("Shape"), labels(list))
    // What answers nothing.
    ;[text, offset, list] = await completeIn(c, completing, edit("val point = Point(1, 2)", "val poi|nt = Point(1, 2)"), ++v)
    check("a declaration's name: nothing", labels(list).length === 0, list)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", "param match { case pa| => 1 }"), ++v)
    check("a pattern binder: nothing", labels(list).length === 0, list)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", 'param.toString + "ho|"'), ++v)
    check("inside a string: nothing", labels(list).length === 0, list)
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", 's"${ho|}".length'), ++v)
    check("a piece of an interpolation: the scope", labels(list).includes("holder"), labels(list))
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", 's"$holder.ow|".length'), ++v)
    check("the text after an interpolated name: nothing", labels(list).length === 0, list)
    // An astral character before the cursor on its line.
    ;[text, offset, list] = await completeIn(c, completing, edit("holder.own(1) + param", '"😀".length + holder.inh|'), ++v)
    check("UTF-16 ranges after an astral character", same(itemOf(list, "inherited")?.textEdit?.insert, range(offset - 3, offset)) && itemOf(list, "inherited").textEdit.insert.end.character === 28, itemOf(list, "inherited"))
    // A completion asked right after an edit: the edit's build answers it.
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(completing), version: ++v }, contentChanges: [{ text: edit("def foobar: Int = 5", "def foobaz: Int = 5") }] })
    ;[text, offset, list] = await completeIn(c, completing, edit("def foobar: Int = 5", "def foobaz: Int = 5").replace("holder.own(1) + param", "holder.foo|"), ++v)
    check("a completion right after edits: from their build", same(labels(list), ["foobaz"]), labels(list))
    // The session answers as before.
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(completing), version: ++v }, contentChanges: [{ text: completingText }] })
    const hoverAfter = await c.result("textDocument/hover", at_(completing, "holder.own", 0, 0))
    check("hover as before the completions", same(hoverAfter, hoverBefore) && hoverBefore?.contents?.value?.includes("val holder: Holder"), { hoverBefore, hoverAfter })
    // The text's one diagnostic is the hint of its unused import (`ListBuffer`).
    const clean = (ds) => ds.length === 1 && ds[0].severity === 4 && same(ds[0].tags, [1]) && same(ds[0].range, at(completing, "ListBuffer").range)
    const diagnostics = await c.waitDiagnostics(0, uriOf(completing), clean)
    check("no diagnostic left by the completions but the text's unused import", clean(c.latestDiagnostics(uriOf(completing))), c.latestDiagnostics(uriOf(completing)))
    // A CRLF file.
    const crlfList = await c.result("textDocument/completion", { textDocument: { uri: uriOf(crlf) }, position: pos(crlf, "one + one", 0, 2) })
    const crlfText = readFileSync(crlf, "utf-8")
    const crlfAt = crlfText.indexOf("one + one")
    check("a CRLF file: the item and its range", same(itemOf(crlfList, "one")?.textEdit?.replace, { start: positionAt(crlfText, crlfAt), end: positionAt(crlfText, crlfAt + 3) }), crlfList)
    // Resolve of an item in scope: the item itself.
    const item = itemOf(crlfList, "one")
    const resolvedItem = await c.result("completionItem/resolve", item)
    check("resolve of an item in scope: the item unchanged", same(resolvedItem, item), resolvedItem)
    // Auto-import: a name out of the scope with the import that brings it, computed on resolve.
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(importing), languageId: "scala", version: 1, text: "package web\n\nobject Importing\n" } })
    let iv = 1
    const importOf = async (marked, label, owner) => {
      const [itext, , ilist] = await completeIn(c, importing, marked, ++iv)
      const it = (ilist?.items ?? []).find((i) => i.label === label && i.labelDetails?.description === owner)
      if (!it) return [itext, undefined, ilist, ilist]
      return [itext, await c.result("completionItem/resolve", it), it, ilist]
    }
    const insertsAt = (r, text, offset, newText) => same(r?.additionalTextEdits, [{ range: { start: positionAt(text, offset), end: positionAt(text, offset) }, newText }])
    const multiline = "package web\n\nimport scala.collection.mutable.{\n  ArrayBuffer,\n  ListBuffer\n}\n\nobject Importing:\n  val t = Thi|\n"
    let [itext, r, it, ilist] = await importOf(multiline, "Thing", "lib")
    check("auto-import: a program class of another package, its owner and kind, after the in-scope names, the list asked again at the next keystroke", it?.kind === 7 && it?.data?.import === "lib.Thing" && /^8/.test(it?.sortText) && ilist?.isIncomplete === true, it)
    check("auto-import: after the last import, one spanning lines with braces", insertsAt(r, itext, itext.indexOf("}") + 1, "\nimport lib.Thing"), r)
    ;[itext, r] = await importOf("package web\n\nobject Importing:\n  val u = UU|\n", "UUID", "java.util")
    check("auto-import: java.util.UUID after the package clause of a file without imports", insertsAt(r, itext, "package web".length, "\n\nimport java.util.UUID"), r)
    ;[itext, r] = await importOf("package web\n\nobject lib\n\nobject Importing:\n  val t = Thi|\n", "Thing", "lib")
    check("auto-import: `_root_` where the first segment names something else", insertsAt(r, itext, "package web".length, "\n\nimport _root_.lib.Thing"), r)
    ;[itext, r] = await importOf("package web:\n  object Importing\npackage web.inner:\n  object Other:\n    val t = Thi|\n", "Thing", "lib")
    check("auto-import: before sibling `package p:` blocks", insertsAt(r, itext, 0, "import lib.Thing\n\n"), r)
    ;[itext, r] = await importOf("object Importing:\n  val t = Thi|\n", "Thing", "lib")
    check("auto-import: at the start of a file without package or import", insertsAt(r, itext, 0, "import lib.Thing\n\n"), r)
    ;[itext, r] = await importOf("package web\r\n\r\nimport scala.util.Try\r\n\r\nobject Importing:\r\n  val t = Thi|\r\n", "Thing", "lib")
    check("auto-import: a CRLF file", insertsAt(r, itext, itext.indexOf("Try") + 3, "\r\nimport lib.Thing"), r)
    ;[itext, , it] = await importOf("package web\n\nimport shapes.*\n\nobject Importing:\n  val t = Circ|\n", "Circle", "shapes")
    const circles = (it?.items ?? []).filter((i) => i.label === "Circle")
    check("auto-import: a name in scope offered once, without an import", circles.length === 1 && circles[0].data === undefined, circles)
    ;[itext, , it] = await importOf("package web\n\nimport lib.{Thing as LibThing}\n\nobject Importing:\n  val t = Thi|\n", "Thing", "lib")
    check("auto-import: a name imported under a rename is not offered again", it === undefined || !(it.items ?? []).some((i) => i.label === "Thing"), it)
    // A resolve after a further edit: the client asks again.
    const [, , stale] = await importOf("package web\n\nobject Importing:\n  val t = Thi|\n", "Thing", "lib")
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(importing), version: ++iv }, contentChanges: [{ text: "package web\n\nobject Importing:\n  val t = Thin\n" }] })
    const late = await c.request("completionItem/resolve", stale)
    check("a resolve after a further edit: ContentModified", late.error?.code === -32801, late)
    await c.result("shutdown", null)
    c.notify("exit", null)
  } catch (e) {
    check("completion ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- A root without export, UTF-8 ---------------------------------------------------------------

async function bareRoot() {
  encoding = "utf-8"
  const c = new Client(bare)
  try {
    const init = await c.result("initialize", { processId: null, rootUri: uriOf(bare), capabilities: { general: { positionEncodings: ["utf-8", "utf-16"] } } })
    check("utf-8 negotiated when offered", init.capabilities.positionEncoding === "utf-8", init.capabilities)
    const m = c.mark()
    c.notify("initialized", {})
    // Asked while the cold build runs: answered after it, from its program.
    const early = await c.result("textDocument/definition", at_(hello, "= emoji", 0, 2))
    check("a request during a cold build, and an astral character under UTF-8", sameLocs(early, [at(hello, "val emoji", 0, 4, 5)]), early)
    const brokenDiags = await c.waitDiagnostics(m, uriOf(broken), (ds) => ds.length > 0)
    check("a cold build with an unparsable file: its errors", brokenDiags?.length > 0, brokenDiags)
    const helloDiags = await c.waitDiagnostics(m, uriOf(hello), (ds) => ds.length > 0)
    const wrong = at(hello, '"not an int"', 0, 0)
    check("a cold build with an unparsable file: the rest typed", helloDiags?.length === 1 && same(helloDiags[0].range, wrong.range), helloDiags)
    const greet = await c.result("textDocument/references", { ...at_(hello, "def greet", 0, 4), context: { includeDeclaration: true } })
    check("a root without export: one session over its files", sameLocs(greet, [at(hello, "def greet", 0, 4, 5), at(hello, 'greet("you")', 0, 0, 5)]), greet)
    const emo = await c.result("textDocument/completion", at_(hello, "= emoji", 0, 5))
    const emoAt = at(hello, "= emoji", 0, 2, 3).range
    check("completion under UTF-8, without insert-and-replace: a text edit over the prefix", same(emo?.items?.find((i) => i.label === "emoji")?.textEdit, { range: emoAt, newText: "emoji" }), emo)
    const brokenDef = await c.result("textDocument/references", { ...at_(broken, "oops"), context: { includeDeclaration: true } })
    check("a file with a syntax error in the cold build: navigation in it", sameLocs(brokenDef, [at(broken, "oops")]), brokenDef)
    // The root's first export: the bare project gives way to it, and the file it leaves out loses
    // its diagnostics.
    const bareExport = join(bare, "teq.lock")
    const m2 = c.mark()
    writeExport(bareExport, { hello: { platform: "js", sources: ["Hello.scala"] } })
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(bareExport), type: 1 }] })
    // Within 5 s: the notification's doing, not the idle search's.
    const brokenCleared = await c.waitDiagnostics(m2, uriOf(broken), (ds) => ds.length === 0, 5000)
    const greetAgain = await c.result("textDocument/references", { ...at_(hello, "def greet", 0, 4), context: { includeDeclaration: true } })
    const bareChildren = execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
    check("a root's first export replaces its bare project", brokenCleared?.length === 0 && sameLocs(greetAgain, [at(hello, "def greet", 0, 4, 5), at(hello, 'greet("you")', 0, 0, 5)]) && bareChildren === 1, { brokenCleared, greetAgain, bareChildren })
    check("without the window capability: no progress", !c.requests.some((r) => r.method === "window/workDoneProgress/create") && !c.notifications.some((n) => n.method === "$/progress"), c.requests.map((r) => r.method))
    await c.result("shutdown", null)
    c.notify("exit", null)
    const code = await Promise.race([c.exited, sleep(10000).then(() => "timeout")])
    check("the bare root: exit code 0", code === 0, code)
  } catch (e) {
    check("the bare root ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- The index's size over a hundred edits ---------------------------------------------------

async function indexSize() {
  const child = spawn(teq, ["compiler", "watch", "--check", "--index", join(ws, "shared/src"), join(ws, "js/src")], { cwd: ws, stdio: ["pipe", "pipe", "inherit"] })
  let buffered = ""
  const lines = []
  child.stdout.on("data", (d) => {
    buffered += d
    let i
    while ((i = buffered.indexOf("\n")) >= 0) {
      lines.push(buffered.slice(0, i))
      buffered = buffered.slice(i + 1)
    }
  })
  const next = async () => {
    for (let k = 0; k < 2400 && !lines.length; k++) await sleep(50)
    if (!lines.length) throw new Error("no answer from the check session")
    return JSON.parse(lines.shift())
  }
  try {
    await next()
    const text = readFileSync(findings, "utf-8")
    // The catalogue of auto-import, built by a first completion, stays through the retypes.
    child.stdin.write(`complete ${realpathSync(findings)} ${Buffer.byteLength(text.slice(0, text.indexOf("= id(x)") + 4))} 1\n`)
    await next()
    let stats
    for (let i = 0; i < 100; i++) {
      const t = text.replace("twice(1)", `twice(${i % 2 ? 1 : 2})`).replace("def hello: Int = 1", `def hello: Int = ${i % 2 ? 1 : 2}`)
      child.stdin.write(`text ${findings} ${Buffer.byteLength(t)}\n${t}build ${findings}\n\n`)
      const answer = await next()
      if (!answer.ok || !answer.incremental) throw new Error(`edit ${i}: ${JSON.stringify(answer).slice(0, 300)}`)
      child.stdin.write("index-stats\n")
      const now = (await next()).result
      if (i === 1) stats = now
      if (i === 99) check("the index's size is flat over a hundred edits, the catalogue's included", same(now, stats) && now.expansions === 0 && now.catalogue > 0, { after2: stats, after100: now })
    }
    // A hundred completions at a hundred positions leave the index and the diagnostics as they were.
    child.stdin.write("index-stats\n")
    const before = (await next()).result
    const last = text.replace("twice(1)", "twice(2)").replace("def hello: Int = 1", "def hello: Int = 2")
    let answered = 0
    for (let k = 0; k < 100; k++) {
      child.stdin.write(`complete ${realpathSync(findings)} ${Math.floor((k * Buffer.byteLength(last)) / 100)} ${k % 2}\n`)
      if ((await next()).result?.items?.length > 0) answered++
    }
    child.stdin.write("index-stats\n")
    const after = (await next()).result
    child.stdin.write("build\n")
    const rebuilt = await next()
    // Clean but for the hints of the workspace's unused imports, which no completion changes.
    const hintsAlone = (rebuilt.diagnostics ?? []).every((d) => d.severity === "hint" && d.message === "unused import")
    check("a hundred completions: most with items, the index's size as before, the next build as clean", answered >= 50 && same(before, after) && rebuilt.ok && hintsAlone, { answered, before, after, rebuilt: JSON.stringify(rebuilt).slice(0, 300) })
    // A hundred signature helps at a hundred positions leave them so too.
    child.stdin.write("index-stats\n")
    const beforeHelp = (await next()).result
    let helped = 0
    for (let k = 0; k < 100; k++) {
      child.stdin.write(`signature ${realpathSync(findings)} ${Math.floor((k * Buffer.byteLength(last)) / 100)}\n`)
      if ((await next()).result?.signatures?.length > 0) helped++
    }
    child.stdin.write("index-stats\n")
    const afterHelp = (await next()).result
    child.stdin.write("build\n")
    const rebuiltAgain = await next()
    check("a hundred signature helps: some answered, the index's size as before, the next build as clean", helped > 0 && same(beforeHelp, afterHelp) && rebuiltAgain.ok && (rebuiltAgain.diagnostics ?? []).every((d) => d.severity === "hint" && d.message === "unused import"), { helped, beforeHelp, afterHelp, rebuilt: JSON.stringify(rebuiltAgain).slice(0, 300) })
  } catch (e) {
    check("the edits ran to their end", false, e.stack)
  } finally {
    child.stdin.end("quit\n")
  }
}

// --- Completion's cases against scalac's resolution, over a check session ---------------------------

/** A check session over `dir`, its answers line by line. */
function session(dir, args = []) {
  const child = spawn(teq, ["compiler", "watch", "--check", "--index", "--positions", "utf-16", ...args, dir], { cwd: dir, stdio: ["pipe", "pipe", "inherit"] })
  let buffered = ""
  const lines = []
  child.stdout.on("data", (d) => {
    buffered += d
    let i
    while ((i = buffered.indexOf("\n")) >= 0) {
      lines.push(buffered.slice(0, i))
      buffered = buffered.slice(i + 1)
    }
  })
  const next = async () => {
    for (let k = 0; k < 2400 && !lines.length; k++) await sleep(50)
    if (!lines.length) throw new Error("no answer from the check session")
    return JSON.parse(lines.shift())
  }
  return { child, next, ask: async (line) => (child.stdin.write(line + "\n"), await next()) }
}

/** Each case a source with `|` at the cursor, the labels the completion there must hold and
 * those it must not: the program each writes is one scalac 3.8.4 accepts with a name it holds
 * there, and rejects with one it leaves out. */
async function completionCases() {
  const dir = join(work, "cases")
  const cases = [
    ["a private alternative of an imported overload left out", "object Lib { def foo(x: Int): Int = x; private def foo(x: String): String = x }\nobject Test { import Lib.foo; val x = fo|o(1) }", ["foo"], [], (items) => items.filter((i) => i.label === "foo").length === 1],
    ["an enum's builtin members", "enum E { case A; case B }\nobject Test { def f(e: E) = e.ordi|nal; def g = E.val|ues }", ["ordinal"], []],
    ["a polymorphic function's type parameter", "object Test { val f = [Thing] => (x: Thi|ng) => x }", ["Thing"], []],
    ["a recursive lazy local in its initializer", "object Test { def f = { lazy val factorial: Int => Int = n => if n <= 1 then 1 else n * fac|torial(n - 1); factorial(5) } }", ["factorial"], []],
    ["a stable value's type member", "class Outer { class Inner }\nobject Test { val o = new Outer; val x: o.In|ner = ??? }", ["Inner"], []],
    ["a generic extension a companion inherits", "class A\ntrait Ext[T] { extension (t: T) def missing: Int = 1 }\nobject A extends Ext[A]\nobject Test { def f(a: A) = a.mis|sing }", ["missing"], []],
    ["a generic conversion an enclosing object inherits", "import scala.language.implicitConversions\nclass A\nclass B { def converted: Int = 1 }\ntrait Convs[T] { given Conversion[T, B] with { def apply(t: T): B = new B } }\nobject Test extends Convs[A] { def f(a: A) = a.con|verted }", ["converted"], []],
    ["a conversion whose given is missing not followed", "import scala.language.implicitConversions\nclass A\nclass B { def unavailable: Int = 1 }\ntrait Missing\nobject Test { given conv(using Missing): Conversion[A, B] with { def apply(a: A): B = new B }; val a = new A; val r = a.unav|ailable }", [], ["unavailable"]],
    ["a local val after a local def that uses it left out", "object Test { def f = { def g = loc|al; val local = 1; g } }", [], ["local"]],
    ["a local method declared later in the block", "object Test { def f = { lat|er(); def later() = 1; 0 } }", ["later"], []],
    ["a parent's member through `super`", "class Base { def found: Int = 1 }\nclass Test extends Base { override def found: Int = super.fou|nd }", ["found"], []],
    ["`super[P]` selecting in `P` alone", "trait Left { def onlyLeft: Int = 1 }\ntrait Right { def onlyRight: Int = 2 }\nclass Test extends Left, Right { def f = super[Left].only|Left }", ["onlyLeft"], ["onlyRight"]],
    ["an extension of an imported given", "class A\ntrait Ops { extension (a: A) def found: Int = 1 }\nobject Lib { given ops: Ops = new Ops {} }\nobject Test { import Lib.given; val x = (new A).fou|nd }", ["found"], []],
    ["an extension of a generic given", "class A\ntrait Ops[T] { extension (a: T) def found: Int = 1 }\nobject Test { given ops[T]: Ops[T] = new Ops[T] {}; val x = (new A).fou|nd }", ["found"], []],
    ["an inherited extension through the companion whose substitution takes it", "trait Base\nclass A extends Base\ntrait Ext[T] { extension (t: T) def found: Int = 1 }\nobject A extends Ext[String]\nobject Base extends Ext[A]\nobject Test { val x = (new A).fou|nd }", ["found"], []],
    ["an inherited extension's signature at its site", "class A\ntrait Ext[T] { extension (t: T) def found: T = t }\nobject A extends Ext[A]\nobject Test { val x = (new A).fou|nd }", ["found"], [], (items) => items.some((i) => i.label === "found" && i.detail === "(t: A): A")],
    ["a type through a stable value's member", "class Inner { class Found }\nclass Outer { val inner = new Inner }\nobject Test { val o = new Outer; val x: o.inner.Fou|nd = ??? }", ["Found"], []],
    ["a private class and alias of a qualified type left out", "object Lib { private class Hidden; class Shown }\nclass Lib2 { private type Hidden2 = Int }\nobject Test { val x: Lib.|Shown = ???; val lib = new Lib2; val y: lib.Hid|den2 = ??? }", ["Shown"], ["Hidden", "Hidden2"]],
    ["a default argument sees the clauses before its own", "object Test { def f(first: Int, second: Int = fir|st)(third: Int = sec|ond): Int = second }", ["second"], ["first"]],
    ["ambiguous conversions offer nothing", "import scala.language.implicitConversions\nclass A\nclass B { def found: Int = 1 }\nobject Test { given one: Conversion[A, B] with { def apply(a: A): B = new B }; given two: Conversion[A, B] with { def apply(a: A): B = new B }; val x = (new A).fou|nd }", [], ["found"]],
    ["no `values` for an enum with a parameterized case", "enum E { case A(x: Int) }\nobject Test { val x = E.val|ues }", [], ["values"]],
    ["an operator's whole token replaced, the cursor before it", "class A { def ++(a: Int): Int = a }\nobject T { val a = new A; val r = a.|++(1) }", ["++"], [], (items) => items.some((i) => i.label === "++" && i.textEdit.replace.end.character - i.textEdit.replace.start.character === 2 && i.textEdit.insert.end.character === i.textEdit.insert.start.character)],
    ["an operator's whole token replaced, the cursor inside it, a member site", "class A { def ++(a: Int): Int = a; def +(a: Int): Int = a }\nobject T { val a = new A; val r = a.+|+(1) }", ["++", "+"], ["println"], (items) => items.some((i) => i.label === "++" && i.textEdit.replace.end.character - i.textEdit.replace.start.character === 2 && i.textEdit.insert.end.character - i.textEdit.insert.start.character === 1)],
    ["an identifier with a symbolic suffix replaced whole, the cursor in it", "class A { def foo_+(a: Int): Int = a }\nobject T { val a = new A; val r = a.foo_|+(1) }", ["foo_+"], [], (items) => items.some((i) => i.label === "foo_+" && i.textEdit.replace.end.character - i.textEdit.replace.start.character === 5 && i.textEdit.insert.end.character - i.textEdit.insert.start.character === 4)],
    ["an identifier with a symbolic suffix, the cursor after it: a member site", "class A { def foo_+(a: Int): Int = a }\nobject T { val a = new A; val r = a.foo_+| }", ["foo_+"], ["println"]],
    ["an exported member of an object", "object Lib { val found = 1 }\nobject Out { export Lib.* }\nobject T { val r = Out.fou|nd }", ["found"], []],
    ["a qualified-private alias inside its package", "package p\nobject Outer { private[p] type Found = Int }\nobject T { val x: Outer.Fou|nd = 1 }", ["Found"], []],
    ["a capitalized binder of a generator in its body", "object T { val r = for (X <- List(1)) yield X| }", ["X"], []],
    ["a generic conversion's member at its solved type", "import scala.language.implicitConversions\nclass A[T]\nclass B[T] { def found: T = ??? }\nobject Test { given [T]: Conversion[A[T], B[T]] with { def apply(a: A[T]): B[T] = new B[T] }; val a = new A[Int]; val x: Int = a.fou|nd }", ["found"], [], (items) => items.some((i) => i.label === "found" && /^Int /.test(i.detail))],
  ]
  let k = 0
  for (const [name, marked, wanted, unwanted, extra] of cases) {
    const caseDir = join(dir, `c${k++}`)
    mkdirSync(caseDir, { recursive: true })
    const file = join(caseDir, "Main.scala")
    const cursors = []
    let text = marked
    for (let i; (i = text.indexOf("|")) >= 0; ) {
      cursors.push(Buffer.byteLength(text.slice(0, i)))
      text = text.slice(0, i) + text.slice(i + 1)
    }
    writeFileSync(file, text)
    const s = session(realpathSync(caseDir))
    try {
      await s.next()
      const all = []
      for (const at of cursors) all.push(...((await s.ask(`complete ${realpathSync(file)} ${at} 1`)).result?.items ?? []))
      const ls = all.map((i) => i.label)
      check(`completion: ${name}`, wanted.every((w) => ls.includes(w)) && unwanted.every((u) => !ls.includes(u)) && (!extra || extra(all)), ls)
    } catch (e) {
      check(`completion: ${name} ran`, false, e.stack)
    } finally {
      s.child.stdin.end("quit\n")
    }
  }
  // Auto-import's place and what it offers, over a library of another package.
  const lib = join(dir, "auto")
  mkdirSync(lib, { recursive: true })
  writeFileSync(join(lib, "Lib.scala"), "package lib\n\nclass RemoteClass\nclass `Remote.Class`\nobject Public { protected class RemoteProtected; type RemoteAlias = Int }\n")
  writeFileSync(join(lib, "Shadow.scala"), "package shadow\n\nclass X\nclass Y\n")
  writeFileSync(join(lib, "Shadows.scala"), "package shadow1:\n  class RemoteClass\npackage shadow2:\n  class RemoteClass\n")
  const autoCases = [
    ["the import before a use that a later import follows", "package use\nobject Test { val x: RemoteCl|ass = ??? }\nimport scala.collection.*\n", "RemoteClass", "\n\nimport lib.RemoteClass", "package use".length],
    ["a `;` kept after an import on the line of a statement", "package use { import scala.collection.*; object Test { val x: RemoteCl|ass = ??? } }\n", "RemoteClass", ";\nimport lib.RemoteClass", "package use { import scala.collection.*".length],
    ["a type alias of an object", "package use\nobject Test { val x: RemoteAl|ias = 1 }\n", "RemoteAlias", "\n\nimport lib.Public.RemoteAlias", "package use".length],
    ["a backquoted name with a dot kept one segment", "package use\nobject Test { val x: `Remote.Cl|ass` = ??? }\n", "`Remote.Class`", "\n\nimport lib.`Remote.Class`", "package use".length],
    ["a spelling two renamed imports leave ambiguous: no edit", "package use\nimport shadow.{X as RemoteClass}\nimport shadow.{Y as RemoteClass}\nobject T { val x: RemoteCl|ass = ??? }\n", "RemoteClass", null, 0],
    ["a spelling two wildcard imports leave ambiguous: the explicit import that settles it", "package use\nimport shadow1.*\nimport shadow2.*\nobject T { val x: RemoteCl|ass = ??? }\n", "RemoteClass", "\nimport lib.RemoteClass", "import shadow2.*".length],
  ]
  // A definition of the file binds its spelling ahead of the wildcards: no import offered for it.
  {
    const file = join(lib, "Use.scala")
    const marked = "package use\nimport shadow1.*\nimport shadow2.*\nclass RemoteClass\nobject T { val x: RemoteCl|ass = new RemoteClass }\n"
    writeFileSync(file, marked.replace("|", ""))
    const s = session(realpathSync(lib))
    try {
      await s.next()
      const items = (await s.ask(`complete ${realpathSync(file)} ${marked.indexOf("|")} 1`)).result?.items ?? []
      const named = items.filter((i) => i.label === "RemoteClass")
      check("auto-import: a spelling the file defines, under two wildcards, offered in scope alone", named.length === 1 && named[0].data === undefined, named)
    } catch (e) {
      check("auto-import: a spelling the file defines ran", false, e.stack)
    } finally {
      s.child.stdin.end("quit\n")
    }
  }
  for (const [name, marked, label, newText, at] of autoCases) {
    const file = join(lib, "Use.scala")
    const offset = marked.indexOf("|")
    writeFileSync(file, marked.replace("|", ""))
    const s = session(realpathSync(lib))
    try {
      await s.next()
      const list = (await s.ask(`complete ${realpathSync(file)} ${offset} 1`)).result
      const item = list?.items?.find((i) => i.label === label && (!i.data || i.data.import.startsWith("lib.")))
      const d = item?.data
      const r = d && (await s.ask(`complete-resolve ${d.path} ${d.offset} ${d.generation} ${d.import}`)).result
      const edited = newText === null ? !r?.edits?.length : r?.edits?.length === 1 && r.edits[0].newText === newText && r.edits[0].range.start.character === at
      check(`auto-import: ${name}`, edited && !list.items.some((i) => i.label === "RemoteProtected"), { list: list?.items?.map((i) => i.label), r })
    } catch (e) {
      check(`auto-import: ${name} ran`, false, e.stack)
    } finally {
      s.child.stdin.end("quit\n")
    }
  }
}

// --- Signature help (docs/TARGETS.md, "The language server") ------------------------------------

/** Signature help's cases over a check session: each a source with `|` at its cursors and, per
 * cursor, the active signature's label and the active parameter's text in it (sliced by its range
 * in UTF-16 units of the label), with the number of signatures where it says more than one, or
 * null where no call's list holds the cursor. */
async function signatureCases() {
  const dir = join(work, "signature")
  const f2 = "f(a: Int)(b: String, c: Int): Int"
  const ov1 = "f(i: Int, s: String): Int"
  const ov2 = "f(b: Boolean, s: Int, d: Double): Int"
  const cases = [
    ["a method of one clause, at each argument", "object A { def f(a: Int, b: String): Int = a; val x = f(|1, |\"s\") }", [["f(a: Int, b: String): Int", "a: Int"], ["f(a: Int, b: String): Int", "b: String"]]],
    ["two clauses: the flat index across them", "object A { def f(a: Int)(b: String, c: Int): Int = a; val x = f(|1)(\"s\", |2) }", [[f2, "a: Int"], [f2, "c: Int"]]],
    ["a using clause inferred, then one written", "trait Ctx\nobject A { def f(a: Int)(using c: Ctx)(b: String): Int = a; given Ctx = new Ctx {}; val x = f(1)(|\"s\"); val y = f(1)(using |summon[Ctx])(\"t\") }", [["f(a: Int)(using c: Ctx)(b: String): Int", "b: String"], ["f(a: Int)(using c: Ctx)(b: String): Int", "c: Ctx"]]],
    ["a default, a repeated parameter past its place", "object A { def f(a: Int, b: Int = 2, c: String*): Int = a; val x = f(1, |2, \"a\", \"b\", |\"c\") }", [["f(a: Int, b: Int = ..., c: String*): Int", "b: Int = ..."], ["f(a: Int, b: Int = ..., c: String*): Int", "c: String*"]]],
    ["an overload the build chose, by the index's record", "object A { def f(i: Int, s: String): Int = 1; def f(b: Boolean, s: Int, d: Double): Int = 2; val b = true; val x = f(b, |1, 2.0) }", [[ov2, "s: Int", 2]]],
    ["an overload the written arguments' types choose, the call unfinished", "object A { def f(i: Int, s: String): Int = 1; def f(b: Boolean, s: Int, d: Double): Int = 2; val b = true; val x = f(b, |) }", [[ov2, "s: Int", 2]]],
    ["an overload a literal's type chooses, the list cut at the end of a line", "object A {\n  def f(i: Int, s: String): Int = 1\n  def f(b: Boolean, s: Int, d: Double): Int = 2\n  val x = f(false, |\n}", [[ov2, "s: Int", 2]]],
    ["an extension method: its receiver left out", "extension (s: String) def repeat2(n: Int): String = s * n\nobject A { val x = \"x\".repeat2(|3); val y = repeat2(\"x\")(|3) }", [["repeat2(n: Int): String", "n: Int"], ["repeat2(s: String)(n: Int): String", "n: Int"]]],
    ["a `new`: the constructors", "class C(x: Int, y: String) { def this(z: Boolean) = this(1, \"\") }\nobject A { val c = new C(1, |\"a\") }", [["C(x: Int, y: String)", "y: String", 2]]],
    ["a case class pattern, an extractor's components", "case class P(x: Int, y: String)\nobject E { def unapply(s: String): Option[(Int, Boolean)] = None }\nobject A { def f(p: Any) = p match { case P(a, |b) => 1; case _ => 2 }; def g(s: String) = s match { case E(a, |b) => 1; case _ => 2 } }", [["P(x: Int, y: String)", "y: String"], ["E(Int, Boolean)", "Boolean"]]],
    ["a generic extractor at the scrutinee's types, an unapplySeq's elements", "object Pair { def unapply[A, B](p: (A, B)): Option[(A, B)] = Some(p) }\nobject Many { def unapplySeq(s: String): Option[Seq[Char]] = Some(s.toSeq) }\nobject A { def f(p: (Int, String)) = p match { case Pair(a, |b) => 1 }; def g(s: String) = s match { case Many(a, |b) => 1; case _ => 2 } }", [["Pair(Int, String)", "String"], ["Many(Char*)", "Char*"]]],
    ["an application's result called, a function value", "object A { def factory(): Int => String = _.toString; val x = factory()(|1); val f: Int => String = _.toString; val y = f(|2) }", [["apply(v1: Int): String", "v1: Int"], ["apply(v1: Int): String", "v1: Int"]]],
    ["a generic method's result called, at its type", "object A { def make[T](t: T): T => List[T] = x => List(x); val y = make(1)(|2) }", [["apply(v1: Int): List[Int]", "v1: Int"]]],
    ["a nested call, the outer one after it", "object A { def f(a: Int, b: Int): Int = a; def h(s: String): Int = 1; val x = f(h(|\"b\"), |2) }", [["h(s: String): Int", "s: String"], ["f(a: Int, b: Int): Int", "b: Int"]]],
    ["a call in a lambda's body, the lambda's body itself none", "object A { def f(a: Int, g: Int => Int): Int = a; def h(s: String): Int = 1; val x = f(1, x => h(|\"a\")); val y = f(1, x => |x) }", [["h(s: String): Int", "s: String"], null]],
    ["the cursor in a string argument, a named argument", "object A { def f(s: String, n: Int): Int = n; val x = f(\"a,|b\", n = |2) }", [["f(s: String, n: Int): Int", "s: String"], ["f(s: String, n: Int): Int", "n: Int"]]],
    ["a list cut at the end of the file", "object A {\n  def f(a: Int, b: String): Int = a\n  val x = f(1, |", [["f(a: Int, b: String): Int", "b: String"]]],
    ["a trailing comma before a closer on the next line", "object A {\n  def f(a: Int, b: Int): Int = 1\n  val x = f(\n    1,\n    |\n  )\n}", [["f(a: Int, b: Int): Int", "b: Int"]]],
    ["a member of a generic receiver, a Java method's overloads", "object A { val xs = List(1, 2); val a = xs.foldLeft(0)(|_ + _); val c = \"abc\".substring(1, |2) }", [["foldLeft[B](z: B)(op: (B, Int) => B): B", "op: (B, Int) => B"], ["substring(start: Int, end: Int): String", "end: Int", 2]]],
    ["a tuple and parentheses inside the list, a block none", "object A { def f(a: (Int, Int), b: Int): Int = 1; val x = f((1, |2), 3); val y = f((1, 2), (|3)); val z = f({ val q = (1, 2); |q }, 3) }", [["f(a: (Int, Int), b: Int): Int", "a: (Int, Int)"], ["f(a: (Int, Int), b: Int): Int", "b: Int"], null]],
    ["an infix operator's list, a type application, `super`", "class B { def f(a: Int): Int = a; def op(a: Int, b: Int): Int = a }\nclass C extends B { override def f(a: Int): Int = super.f(|a); def h = this op (1, |2); def k[T](t: T, u: List[T]): Int = 1; def m = k[Int](1, |Nil) }", [["f(a: Int): Int", "a: Int"], ["op(a: Int, b: Int): Int", "b: Int"], ["k[T](t: T, u: List[T]): Int", "u: List[T]"]]],
    ["a case class's companion apply, an object's overloaded apply", "case class P(x: Int, y: String)\nobject O { def apply(a: Int): Int = a; def apply(s: String): Int = 1 }\nobject A { val p = P(1, |\"a\"); val q = O(|\"s\") }", [["P(x: Int, y: String)", "y: String"], ["apply(s: String): Int", "s: String", 2]]],
    ["label ranges in UTF-16 units: an astral character in a backquoted name", "object A { def f(`\u{1D4B3} value`: Int, y: String): Int = 1; val x = f(1, |\"a\") }", [["f(`\u{1D4B3} value`: Int, y: String): Int", "y: String"]]],
    ["a generic extension's overloads: the receiver's type for its parameter, the one the build chose", "object A {\n  extension [T](xs: List[T]) {\n    def f(x: String, y: Int): Int = 1\n    def f(x: T, y: String): Int = 2\n  }\n  val a = List(1).f(1, |\"s\")\n}", [["f(x: Int, y: String): Int", "y: String", 2]]],
    ["a named first argument of a written `using` list", "object A {\n  def f(using a: Int, b: String): Int = a\n  val x = f(using b = |\"x\", a = 1)\n}", [["f(using a: Int, b: String): Int", "b: String"]]],
    ["a comment between the callee and its list", "object A {\n  def f(x: Int, y: Int): Int = x\n  def g(s: String): Int = 1\n  val a = f(g /* hi */ (|\"s\"), 2)\n}", [["g(s: String): Int", "s: String"]]],
    ["a callee in parentheses: a function value, an application's result", "object A {\n  val g: Int => String = _.toString\n  def factory(): Int => String = _.toString\n  val a = (g)(|1); val c = (factory())(|1)\n}", [["apply(v1: Int): String", "v1: Int"], ["apply(v1: Int): String", "v1: Int"]]],
    ["a `new`'s further lists, plain and `using`", "class C(a: Int)(b: String)\nclass D(a: Int)(using b: String)\nobject A { val c = new C(1)(|\"s\"); val d = new D(1)(using |\"s\") }", [["C(a: Int)(b: String)", "b: String"], ["D(a: Int)(using b: String)", "b: String"]]],
    ["a returned function's result called again", "object A {\n  def factory(): Int => String => Boolean = i => s => true\n  val a = factory()(1)(|\"s\")\n}", [["apply(v1: String): Boolean", "v1: String"]]],
    ["an unapplySeq of fixed components and a sequence", "object E {\n  def unapplySeq(s: String): Option[(Int, Seq[Char])] =\n    Some((1, s.toSeq))\n}\nobject A {\n  def f(s: String) = s match {\n    case E(a, |b, c) => a\n    case _ => 0\n  }\n}", [["E(Int, Char*)", "Char*"]]],
    ["a Scala 2 implicit clause shown as written", "trait Ctx\nobject A { def f(a: Int)(implicit c: Ctx): Int = a; implicit val c: Ctx = new Ctx {}; val x = f(|1) }", [["f(a: Int)(implicit c: Ctx): Int", "a: Int"]]],
    ["an infix operator's single argument in parentheses inside another call", "class C { infix def op(x: Int): Int = x }\nobject A {\n  val c = new C\n  def f(s: Int): Int = s\n  val r = f(c op (|1))\n}", [["op(x: Int): Int", "x: Int"]]],
    ["`super` through the linearisation: a trait's overload, the build's choice", "class B { def f(i: Int): Int = i }\ntrait M { def f(s: String): String = s }\nclass C extends B with M {\n  def g = super.f(|\"s\")\n}", [["f(s: String): String", "s: String", 2]]],
    ["an overload the build chose through a conversion of an argument", "import scala.language.implicitConversions\nobject A {\n  given Conversion[Boolean, String] = x => x.toString\n  def f(a: Int, b: Int): Int = 1\n  def f(a: String, b: String): Int = 2\n  val x = f(true, |\"s\")\n}", [["f(a: String, b: String): Int", "b: String", 2]]],
    ["an infix operator on an application's result and on a selection", "class C {\n  infix def op(x: Int, y: String): Int = x\n}\nclass B { val c = new C }\nobject A {\n  def make(): C = new C\n  val b = new B\n  val r = make() op (1, |\"s\")\n  val q = b.c op (1, |\"s\")\n}", [["op(x: Int, y: String): Int", "y: String"], ["op(x: Int, y: String): Int", "y: String"]]],
    ["a returned object's curried apply, its second clause", "class C {\n  def apply(a: Int)(b: String): Boolean = true\n}\nobject A {\n  def factory(): C = new C\n  val x = factory()(1)(|\"s\")\n}", [["apply(a: Int)(b: String): Boolean", "b: String"]]],
    ["an anonymous class's parent constructor, an abstract one's second list", "class C(x: Int)\nabstract class D(y: String)(z: Int)\nobject A {\n  val c = new C(|1) {}\n  val d = new D(\"a\")(|2) {}\n}", [["C(x: Int)", "x: Int"], ["D(y: String)(z: Int)", "z: Int"]]],
    ["a negative literal the cursor stands before: its type chooses the overload", "object A {\n  def f(x: String, y: Int): Int = 1\n  def f(x: Int, y: Int): Int = 2\n  val a = f(|-1)\n}", [["f(x: Int, y: Int): Int", "x: Int", 2]]],
    ["a Scala 2 implicit clause given a plain list", "object A {\n  def f(x: Int)(implicit y: String): Int = x\n  val a = f(1)(|\"hi\")\n}", [["f(x: Int)(implicit y: String): Int", "y: String"]]],
    ["an extension `apply` on a value", "class C\nobject A {\n  extension (c: C) def apply(x: String): Int = 1\n  val c = new C\n  val x = c(|\"s\")\n}", [["apply(x: String): Int", "x: String"]]],
    ["a constructed object applied, with and without parentheses", "class C(x: Int) {\n  def apply(y: String): Boolean = true\n}\nobject A {\n  val x = new C(1)(|\"s\")\n  val y = (new C(1))(|\"s\")\n}", [["apply(y: String): Boolean", "y: String"], ["apply(y: String): Boolean", "y: String"]]],
    ["a function type's arrow in an ascription, a function literal's body none", "object A {\n  def f(g: Int => Int, n: Int): Int = n\n  val x: Int => Int = i => i\n  val a = f(x: Int => Int|, 2)\n  val b = f(i => i|, 2)\n}", [["f(g: Int => Int, n: Int): Int", "g: Int => Int"], null]],
    ["a comment inside an operator's parentheses, its receiver an application's result", "class C {\n  infix def op(x: Int): Int = x\n}\nobject A {\n  def make(): C = new C\n  val x = make() op ( /* hi */ |1)\n}", [["op(x: Int): Int", "x: Int"]]],
    ["a constructed instance's apply at the type arguments written", "class C[T](x: T) {\n  def apply(y: String, z: Int): String = y\n  def apply(y: T, z: Int): T = y\n}\nobject A { val a = new C[Int](1)(|2, 0) }", [["apply(y: Int, z: Int): Int", "y: Int", 2]]],
    ["an extension `apply` beside a member `apply` that cannot take the arguments", "class C { def apply(i: Int): Int = i }\nobject A {\n  extension (c: C)\n    def apply(s: String, b: Boolean): String = s\n  val c = new C\n  val a = c(\"s\", |true)\n  val b = c(|1)\n}", [["apply(s: String, b: Boolean): String", "b: Boolean", 2], ["apply(i: Int): Int", "i: Int"]]],
    ["a function type's arrow in an ascription, the cursor before or in a written type", "object A {\n  def f(g: Int => Int, n: Int): Int = n\n  val x: Int => Int = i => i\n  val a = f(x: Int => |Int, 2)\n  val b = f(x: Int => I|nt, 2)\n  val c = f(i => |i, 2)\n}", [["f(g: Int => Int, n: Int): Int", "g: Int => Int"], ["f(g: Int => Int, n: Int): Int", "g: Int => Int"], null]],
    ["a polymorphic function literal's body", "object A {\n  def f(g: [T] => T => T): Int = 1\n  val a = f([T] => |)\n  val b = f([T] => (x: T) => |x)\n}", [null, null]],
    ["outside any list: after a call, in a type argument list, in a block argument", "object A { def f(a: Int): Int = a; val x = f(1)|; val y = List[|Int](); val z = Some(1).map { x => |x } }", [null, null, null]],
  ]
  let k = 0
  for (const [name, marked, wanted] of cases) {
    const caseDir = join(dir, `s${k++}`)
    mkdirSync(caseDir, { recursive: true })
    const file = join(caseDir, "Main.scala")
    const cursors = []
    let text = marked
    for (let i; (i = text.indexOf("|")) >= 0; ) {
      cursors.push(Buffer.byteLength(text.slice(0, i)))
      text = text.slice(0, i) + text.slice(i + 1)
    }
    writeFileSync(file, text)
    const s = session(realpathSync(caseDir))
    try {
      await s.next()
      const got = []
      for (const at of cursors) {
        const help = (await s.ask(`signature ${realpathSync(file)} ${at}`)).result
        if (!help) {
          got.push(null)
          continue
        }
        const sig = help.signatures[help.activeSignature]
        const range = sig?.parameters?.[help.activeParameter]?.label
        const entry = [sig?.label, range ? sig.label.slice(range[0], range[1]) : undefined]
        if (help.signatures.length > 1) entry.push(help.signatures.length)
        // Each signature's own active parameter is the answer's for the active one.
        if (sig?.activeParameter !== help.activeParameter) entry.push("activeParameter differs")
        got.push(entry)
      }
      check(`signature help: ${name}`, same(got, wanted), got)
    } catch (e) {
      check(`signature help: ${name} ran`, false, e.stack)
    } finally {
      s.child.stdin.end("quit\n")
    }
  }
}

/** `textDocument/signatureHelp` through the server: its capability, an answer in the document's
 * own positions, null outside a call, and `ContentModified` for a request behind a build that an
 * edit cancels, never the later text's answer. */
async function signatureHelp() {
  const root = join(work, "link/signature-help")
  mkdirSync(root, { recursive: true })
  const file = join(root, "Test.scala")
  const t1 = "object Test:\n  def pick(first: Int, second: String): Int = first\n  val s = \"\u{1F600}\"; val r = pick(1, \"x\")\n"
  writeFileSync(file, t1)
  const trace = join(work, "signature-help-trace")
  const barrier = join(work, "signature-help-barrier")
  const held = (point) => (e) => e.who === "session" && e.event === "held" && e.rest[0] === point
  const c = new Client(root, { TEQ_LSP_TRACE_FILE: trace, TEQ_LSP_TEST_BARRIER: barrier })
  let version = 1
  const change = (text) => c.notify("textDocument/didChange", { textDocument: { uri: uriOf(file), version: ++version }, contentChanges: [{ text }] })
  const helpAt = (text, needle) => c.request("textDocument/signatureHelp", { ...doc(file), position: positionAt(text, text.indexOf(needle.replace("|", "")) + needle.indexOf("|")) }, { timeout: 20000 }).catch((e) => ({ error: { message: e.message } }))
  try {
    encoding = "utf-16"
    const init = await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: {} })
    const provider = init.capabilities.signatureHelpProvider
    check("initialize: signature help triggered by `(` and `,`, retriggered by `,`", same(provider, { triggerCharacters: ["(", ","], retriggerCharacters: [","] }), init.capabilities)
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text: t1 } })
    let a = await helpAt(t1, "pick(1, |\"x\")")
    const sig = a.result?.signatures?.[0]
    check("signature help: the call's signature, the second parameter active, after an astral character on the line", sig?.label === "pick(first: Int, second: String): Int" && a.result.activeParameter === 1 && same(sig.parameters.map((p) => sig.label.slice(...p.label)), ["first: Int", "second: String"]), a)
    a = await helpAt(t1, "val r = |pick")
    check("signature help: null outside a call", a.result === null, a)
    // Asked behind a build that the next edit cancels: that revision is never typed.
    writeFileSync(`${barrier}.start`, "")
    const from = traceOf(trace).length
    const t2 = t1.replace("pick(1, \"x\")", "pick(1, )")
    change(t2)
    await traced(trace, from, held("start"), "build held at its start")
    const stale = helpAt(t2, "pick(1, |)")
    await sleep(200)
    change(t2.replace("pick(1, )", "pick(1, \"y\")"))
    await traced(trace, from, (e) => e.event === "wrote-cancel", "cancel written")
    rmSync(`${barrier}.start`)
    a = await stale
    check("signature help behind a build an edit cancels: ContentModified, not the later text's answer", a.error?.code === -32801, a)
    await c.result("shutdown", null)
    c.notify("exit", null)
  } catch (e) {
    check("the signature help scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  } finally {
    rmSync(`${barrier}.start`, { force: true })
  }
}

// --- Calls completed as snippets (docs/TARGETS.md, "Completion") ---------------------------------

/** Each site a line of `Test` with `|` at the cursor, the label of its items and what each inserts
 * for a client that takes snippets (null: the name alone): the call scalac 3.8.4 accepts once each
 * placeholder holds an argument of its type, the name alone where a call would not type. A client
 * without snippets gets the name alone everywhere; one with them through the server, the resolve
 * of an auto-import item keeping its snippet. */
async function completionSnippets() {
  const root = join(work, "link/completion-snippets")
  mkdirSync(join(root, "lib"), { recursive: true })
  writeFileSync(join(root, "lib/Remote.scala"), "package lib\n\nclass Remote(a: Int, b: String)\n")
  const sites = [
    ["a method of one clause", "val a1 = Lib.in|", "inc", ["inc(${1:x})$0"]],
    ["two clauses numbered across, a name in scope", "val a2 = tw|", "two", ["two(${1:a})(${2:b})$0"]],
    ["a using clause left out", "val a3 = Lib.ct|", "ctx", ["ctx(${1:a})$0"]],
    ["parameters with defaults still placeholders", "val a4 = Lib.dfl|", "dflt", ["dflt(${1:a}, ${2:b})$0"]],
    ["a repeated parameter one placeholder", "val a5 = Lib.re|", "rep", ["rep(${1:xs})$0"]],
    ["a parameter of a function type in parentheses", "val a6 = Lib.fu|", "fun", ["fun(${1:f})$0"]],
    ["an empty clause", "val a7 = Lib.emp|", "empty", ["empty()$0"]],
    ["a parameterless method", "val a8 = Lib.noa|", "noargs", [null]],
    ["a val of a function type", "val a9 = Lib.valu|", "value", [null]],
    ["an object", "val a10 = Lib.Ob|", "Obj", [null]],
    ["an overload set, a snippet each", "val a11 = Lib.ow|", "own", ["own(${1:x})$0", "own(${1:x}, ${2:y})$0"]],
    ["a backquoted name and parameters, escaped", "val a12 = Lib.od|", "`odd name`", ["`odd name`(${1:`type`}, ${2:`a\\}b`})$0"]],
    ["an operator", "val a13 = Lib.+| + 1", "++", [null]],
    ["an extension on its receiver, the receiver's clause left out", 'val a14 = "a".appen|', "append", ["append(${1:y})$0"]],
    ["an argument list after the name", "val a15 = Lib.in|(2)", "inc", [null]],
    ["a type argument list after the name", "val a16 = Lib.i|d[Int](2)", "id", [null]],
    ["a brace block after the name", "val a17 = Lib.fu| { x => x }", "fun", [null]],
    ["an import selector", "import Lib.tw|", "two", [null]],
    ["an export selector", "export Lib.tw|", "two", [null]],
    ["a val of a function type's value", "val a20: Int => Int = Lib.in|", "inc", [null]],
    ["a function argument", "val a21 = List(1).map(Lib.in|)", "inc", [null]],
    ["a function argument, a name in scope", "val a22 = List(1).map(in|)", "inc", [null]],
    ["a case class's constructor after `new`", "val a23 = new Poi|", "Point", ["Point(${1:x}, ${2:y})$0"]],
    ["a class without parameters after `new`", "val a24 = new Pla|", "Plain", [null]],
    ["a constructor's using clause left out", "val a25 = new Cto|", "Ctor", ["Ctor(${1:a})$0"]],
    ["arguments after the class", "val a26 = new Poi|(1, 2)", "Point", [null]],
    ["a companion's apply not expanded", "val a27 = Poi|", "Point", [null]],
    ["an auto-import class after `new`: its constructor", "val a28 = new Remot|", "Remote", ["Remote(${1:a}, ${2:b})$0"]],
    ["an auto-import class elsewhere: its name", "val a29 = Remot|", "Remote", [null]],
    ["a function a variable's bound expects", "val a30: List[Int => Int] = List(Lib.in|)", "inc", [null]],
    ["an argument an overload takes a function of", "val a31 = Lib.take(Lib.in|)", "inc", [null]],
    ["an argument an overload takes a function of, the name whole", "val a32 = Lib.take(Lib.inc|)", "inc", [null]],
    ["after a dot where a function is expected", "val a33: Int => Int = (Lib.|)", "inc", [null]],
    ["after a dot where a value is expected", "val a34: Int = (Lib.|)", "inc", ["inc(${1:x})$0"]],
    ["an eta-expansion's underscore after the name", "val a35 = Lib.in| _", "inc", [null]],
    ["a name spliced bare into an interpolation", 'val a36 = s"$tw|"', "two", [null]],
    ["a name spliced in braces into an interpolation", 'val a37 = s"${tw|}"', "two", ["two(${1:a})(${2:b})$0"]],
    ["a union of functions of other arities", "val a38: Lib.Fns = Lib.ad|", "add", [null]],
    ["an opaque function type seen through", "val a39: F = Lib.in|", "inc", [null]],
    ["a branch of an argument an overload takes a function of", "val a40 = Lib.take(if (true) Lib.in| else Lib.inc)", "inc", [null]],
    ["a yield where a function is expected", "val a41: List[Int => Int] = for (i <- List(1)) yield Lib.in|", "inc", [null]],
    ["after a dot and a space where a function is expected", "val a42: Int => Int = (Lib. |)", "inc", [null]],
    ["a Unicode operator", "val a43 = Lib.⊕|", "`⊕`", [null]],
    ["an intersection of functions", "val a44: (Int => Int) & (Int => AnyVal) = Lib.in|", "inc", [null]],
    ["an overload's type parameter bounded by a function", "val a45 = Lib.gen(Lib.in|)", "inc", [null]],
    ["a try of an argument an overload takes a function of", "val a46 = Lib.take(try Lib.in| catch { case _: Exception => Lib.inc })", "inc", [null]],
    ["an unchecked argument an overload takes a function of", "val a47 = Lib.take((Lib.in|: @unchecked))", "inc", [null]],
    ["an eta-expansion's underscore after parentheses", "val a48 = (Lib.in|) _", "inc", [null]],
    ["after a dot and a comment where a function is expected", "val a49: Int => Int = (Lib. /* c */ |)", "inc", [null]],
    ["a backquoted name starting with an operator character", "val a50 = Lib.`+na|me`", "`+name`", ["`+name`(${1:x})$0"]],
    ["an argument a function is expected of, nothing written", "val a51 = List(1).map(|)", "inc", [null]],
    ["an argument a value is expected of, nothing written", "val a52 = Lib.dflt(1, |)", "inc", ["inc(${1:x})$0"]],
    ["a union of a function and five other types", "val a53: Lib.Many = Lib.in|", "inc", [null]],
    ["a definition of a function type, nothing written", "object Rhs { val a54: Int => Int = | }", "inc", [null]],
    ["a local definition of a function type, nothing written", "def a55 = { val f: Int => Int = |; f }", "inc", [null]],
    ["a named argument a function is expected of, nothing written", "val a56 = Lib.take(f = |)", "inc", [null]],
    ["a using argument a function is expected of, nothing written", "val a57 = Lib.withFn(1)(using |)", "inc", [null]],
    ["a type argument a function type, nothing written", "val a58 = identity[Int => Int](|)", "inc", [null]],
    ["a type argument a value type, nothing written", "val a59 = identity[Int](|)", "inc", ["inc(${1:x})$0"]],
    ["a value of a function type called, nothing written", "val a60 = Lib.consume(|)", "inc", [null]],
    ["a parameter of a function type called, nothing written", "def a61(g: (Int => Int) => Int) = g(|)", "inc", [null]],
    ["an overload whose function candidate is not applicable", "def a62 = { val in = 1; Lib.pick(in|, 1) }", "inc", ["inc(${1:x})$0"]],
    ["an argument of a call an underscore follows", "val a63 = Lib.outer(in|) _", "inc", ["inc(${1:x})$0"]],
    ["an overload whose function candidate a later list eliminates", "def a64(in: Nothing) = Lib.pickc(in|)(1)", "inc", ["inc(${1:x})$0"]],
    ["an overload whose function candidate a later list keeps", "def a65(in: Nothing) = Lib.pickc(in|)(\"a\")", "inc", [null]],
    ["a group after an if's condition before an underscore", "val a66 = if (Lib.test()) (Lib.in|) _ else Lib.inc", "inc", [null]],
    ["a qualified alias of a function type as a type argument", "val a67 = identity[Lib.Fn](|)", "inc", [null]],
    ["a context function value's using argument", "val a68 = Lib.ctxFn(using |)", "inc", [null]],
    ["the function after a context function's using argument", "val a69 = Lib.ctxThen(using 1)(|)", "inc", [null]],
  ]
  const header = [
    "package use",
    "",
    "object Lib:",
    "  def inc(x: Int): Int = x + 1",
    "  def two(a: Int)(b: String): Int = a",
    "  def ctx(a: Int)(using s: String): Int = a",
    "  def dflt(a: Int = 1, b: Int = 2): Int = a + b",
    "  def rep(xs: Int*): Int = xs.sum",
    "  def fun(f: Int => Int): Int = f(1)",
    "  def empty(): Int = 1",
    "  def noargs: Int = 1",
    "  val value: Int => Int = x => x",
    "  object Obj",
    "  def own(x: Int): Int = x",
    "  def own(x: String, y: Int): Int = 0",
    "  def `odd name`(`type`: Int, `a}b`: Int): Int = 1",
    "  def ++(a: Int): Int = a",
    "  def take(f: Int => Int): Int = 1",
    "  def add(x: Int, y: Int): Int = x + y",
    "  type Fns = ((Int, Int) => Int) | (() => Int)",
    "  type Many = (Int => Int) | String | Boolean | Unit | Char | Null",
    "  def ⊕(x: Int): Int = x",
    "  def gen[F <: (Int => Int)](f: F): Int = 1",
    "  def gen(s: String): Int = 2",
    "  def `+name`(x: Int): Int = x",
    "  def withFn(a: Int)(using f: Int => Int): Int = a",
    "  val consume: (Int => Int) => Int = f => f(1)",
    "  def pick(f: Int => Int, tag: String): Int = 1",
    "  def pick(x: Int, tag: Int): Int = x",
    "  def outer(x: Int): Int => Int = inc",
    "  def pickc(f: Int => Int)(tag: String): Int = 1",
    "  def pickc(x: Int)(tag: Int): Int = x",
    "  def test(): Boolean = true",
    "  type Fn = Int => Int",
    "  val ctxFn: (Int => Int) ?=> Int = summon[Int => Int](1)",
    "  val ctxThen: Int ?=> (Int => Int) => Int = f => f(1)",
    "  def take(x: String): Int = 2",
    "  def id[T](x: T): T = x",
    "  extension (s: String) def append(y: String): String = s + y",
    "",
    "case class Point(x: Int, y: Int)",
    "class Plain",
    "class Ctor(a: Int)(using s: String)",
    "",
    "object Test:",
    "  import Lib.*",
    '  given String = ""',
    "  opaque type F = Int => Int",
  ]
  let text = header.join("\n") + "\n"
  const cursors = []
  for (const [, line] of sites) {
    const i = line.indexOf("|")
    cursors.push(Buffer.byteLength(text) + 2 + Buffer.byteLength(line.slice(0, i)))
    text += "  " + line.slice(0, i) + line.slice(i + 1) + "\n"
  }
  const file = join(root, "Test.scala")
  writeFileSync(file, text)
  const s = session(realpathSync(root))
  let auto
  try {
    await s.next()
    for (let k = 0; k < sites.length; k++) {
      const [name, , label, wanted] = sites[k]
      const ask = async (flags) => ((await s.ask(`complete ${realpathSync(file)} ${cursors[k]} ${flags}`)).result?.items ?? []).filter((i) => i.label === label)
      const [snippets, plain] = [await ask(3), await ask(1)]
      const got = snippets.map((i) => (i.insertTextFormat === 2 ? i.textEdit.newText : i.textEdit.newText === label && i.insertTextFormat === undefined ? null : `not plain: ${JSON.stringify(i)}`))
      const ranges = same(snippets.map((i) => [i.textEdit.insert, i.textEdit.replace]), plain.map((i) => [i.textEdit.insert, i.textEdit.replace]))
      check(`snippets: ${name}`, same(got, wanted) && ranges, snippets)
      check(`snippets: ${name}, without the capability the name alone`, plain.length === wanted.length && plain.every((i) => i.insertTextFormat === undefined && i.textEdit.newText === label), plain)
      if (label === "Remote") {
        check(`snippets: ${name}, with its import`, snippets.length === 1 && snippets[0].data?.import === "lib.Remote" && plain[0]?.data?.import === "lib.Remote", snippets)
        auto ??= snippets[0]
      }
    }
    const d = auto?.data
    const r = d && (await s.ask(`complete-resolve ${d.path} ${d.offset} ${d.generation} ${d.import}`)).result
    check("snippets: the import of an auto-import item under a snippet", r?.edits?.length === 1 && r.edits[0].newText === "\n\nimport lib.Remote", r)
  } catch (e) {
    check("snippets: the check session ran", false, e.stack)
  } finally {
    s.child.stdin.end("quit\n")
  }
  // Through the server: the capability read, the item's snippet, its resolve keeping it.
  const c = new Client(root)
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { textDocument: { completion: { completionItem: { snippetSupport: true, insertReplaceSupport: true } } } } })
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text } })
    const at = (k) => ({ ...doc(file), position: positionAt(text, text.indexOf(sites[k][1].replace("|", "")) + sites[k][1].indexOf("|")) })
    const members = await c.result("textDocument/completion", at(0))
    const inc = itemOf(members, "inc")
    check("snippets through the server: a method's call", inc?.insertTextFormat === 2 && inc?.textEdit?.newText === "inc(${1:x})$0", inc)
    const list = await c.result("textDocument/completion", at(27))
    const item = itemOf(list, "Remote")
    const resolved = item && (await c.result("completionItem/resolve", item))
    const start = text.indexOf("new Remot") + 4
    const range = { start: positionAt(text, start), end: positionAt(text, start + 5) }
    check("snippets through the server: an auto-import constructor, its ranges the token's", item?.insertTextFormat === 2 && same(item?.textEdit, { newText: "Remote(${1:a}, ${2:b})$0", insert: range, replace: range }), item)
    check("snippets through the server: the resolve adds the import and keeps the snippet", same(resolved?.textEdit, item?.textEdit) && resolved?.insertTextFormat === 2 && resolved?.additionalTextEdits?.[0]?.newText === "\n\nimport lib.Remote", resolved)
    await c.result("shutdown", null)
    c.notify("exit", null)
  } catch (e) {
    check("snippets through the server ran to their end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- A session's state across completions: tests/split/retype_cacheable --------------------------

/** The answers of a check session over a copy of tests/split/retype_cacheable through its first
 * build and two retypes of `use.scala`, with `between` run before each build after the first: the
 * warnings of its macro count the expansions of an object with cacheable state, which the retypes
 * keep. */
async function cacheableSession(dir, between) {
  rmSync(dir, { recursive: true, force: true })
  cpSync(join(here, "../split/retype_cacheable"), dir, { recursive: true })
  const use = join(dir, "use.scala")
  const args = ["compiler", "watch", "--check", "--index", "--cacheable-state", "cacheable.Cache", "--cacheable-state", "cacheable.Holder", "--cacheable-state", "cacheable.Outer.Inner", dir]
  const child = spawn(teq, args, { cwd: dir, stdio: ["pipe", "pipe", "inherit"] })
  let buffered = ""
  const lines = []
  child.stdout.on("data", (d) => {
    buffered += d
    let i
    while ((i = buffered.indexOf("\n")) >= 0) {
      lines.push(buffered.slice(0, i))
      buffered = buffered.slice(i + 1)
    }
  })
  const next = async () => {
    for (let k = 0; k < 2400 && !lines.length; k++) await sleep(50)
    if (!lines.length) throw new Error("no answer from the check session")
    return JSON.parse(lines.shift())
  }
  const shown = (a) => JSON.stringify([a.ok, a.incremental, (a.diagnostics ?? []).map((d) => [d.line, d.col, d.severity, d.message])])
  const answers = []
  try {
    answers.push(shown(await next()))
    for (const [from, to] of [['"use"', '"used"'], ['"used"', '"used up"']]) {
      await between(child, next, use)
      writeFileSync(use, readFileSync(use, "utf-8").replace(from, to))
      child.stdin.write(`build ${use}\n\n`)
      answers.push(shown(await next()))
    }
  } finally {
    child.stdin.end("quit\n")
  }
  return answers
}

async function cacheableAcrossCompletions() {
  try {
    const plain = await cacheableSession(join(work, "cacheable-plain"), async () => {})
    const completed = await cacheableSession(join(work, "cacheable-completed"), async (child, next, use) => {
      const text = readFileSync(use, "utf-8")
      for (const needle of ["uses", "same", "held", "inner", "label"]) {
        child.stdin.write(`complete ${realpathSync(use)} ${text.indexOf(needle) + 2} 1\n`)
        await next()
      }
    })
    check("completions leave a session's cacheable state and answers as they were", same(plain, completed) && plain.some((a) => a.includes("uses 2 2")), { plain, completed })
  } catch (e) {
    check("the cacheable sessions ran to their end", false, e.stack)
  }
}

// --- A build that does not end: the child stopped ----------------------------------------------

async function stoppedChild() {
  const root = join(work, "link/stopped")
  mkdirSync(root, { recursive: true })
  const use = join(root, "Use.scala")
  writeFileSync(use, "object Use:\n  val n: Int = 1\n")
  const c = new Client(root)
  const created = () => c.requests.filter((r) => r.method === "window/workDoneProgress/create").map((r) => r.params.token)
  const progress = (kind) => c.notifications.filter((n) => n.method === "$/progress" && n.params.value.kind === kind).map((n) => n.params)
  let child
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { window: { workDoneProgress: true } } })
    c.notify("initialized", {})
    // Answered after the first build, whose token ended before the answer.
    await c.result("textDocument/hover", { ...doc(use), position: { line: 1, character: 6 } })
    check(
      "the first build's token: begun at the start, ended as it answered",
      created().length === 1 && same(progress("begin").map((p) => [p.token, p.value.title, p.value.message]), [[created()[0], "teq", "checking stopped"]]) && same(progress("end").map((p) => p.token), created()),
      { created: created(), begin: progress("begin"), end: progress("end") },
    )
    // The child stopped, an edit's build never answers: shown once it is 300 ms old (the bound's
    // arithmetic is the unit tests' matter, `wake_in`). When the stop catches an idle poll, which
    // alone is never shown, the edit waits behind it and the poll is shown in its stead: so the
    // checks are on the last token created, open with every earlier one ended.
    child = Number(execSync(`pgrep -P ${c.proc.pid}`).toString().trim().split("\n")[0])
    execSync(`kill -STOP ${child}`)
    const ended = (token) => progress("end").some((p) => p.token === token)
    const lastOpen = () => {
      const tokens = created()
      const last = tokens[tokens.length - 1]
      return tokens.length >= 2 && progress("begin").some((p) => p.token === last) && !ended(last) && tokens.slice(0, -1).every(ended)
    }
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(use), languageId: "scala", version: 1, text: "object Use:\n  val n: Int = 2\n" } })
    const begun = await until(lastOpen, 5000)
    check("a build that does not answer begins a token", begun, { created: created(), begin: progress("begin"), end: progress("end") })
    const shut = await c.request("shutdown", null, { timeout: 5000 })
    check("shutdown ends the pending build's token", shut.result === null && created().every(ended), { created: created(), end: progress("end") })
    c.notify("exit", null)
    const code = await Promise.race([c.exited, sleep(5000).then(() => "timeout")])
    let alive = true
    try {
      process.kill(child, 0)
    } catch {
      alive = false
    }
    check("exit with a stopped child: code 0, the child gone", code === 0 && !alive, { code, alive })
  } catch (e) {
    check("the stopped child scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  } finally {
    // A child the server did not kill (the scenario cut short) is let go and ended.
    try {
      if (child) {
        process.kill(child, "SIGCONT")
        process.kill(child, "SIGKILL")
      }
    } catch {}
  }
}

// --- A child that does not read ----------------------------------------------------------------

async function busyChild() {
  const slow = join(work, "link/slow")
  cpSync(join(here, "slow"), slow, { recursive: true })
  const c = new Client(slow)
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(slow), capabilities: {} })
    c.notify("initialized", {})
    await sleep(500)
    // A text larger than a pipe holds, for a child evaluating a macro that never ends.
    const use = join(slow, "Use.scala")
    const big = readFileSync(use, "utf-8") + "// " + "x".repeat(256 * 1024) + "\n"
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(use), languageId: "scala", version: 1, text: big } })
    // Eighty versions of a 1 MiB text, each with a request that sends it: the server keeps the
    // last text for the child, not every version.
    const rss = () => Number(execSync(`ps -o rss= -p ${c.proc.pid}`).toString().trim()) * 1024
    const before = rss()
    const mib = "x".repeat(1024 * 1024)
    for (let v = 2; v <= 81; v++) {
      c.notify("textDocument/didChange", { textDocument: { uri: uriOf(use), version: v }, contentChanges: [{ text: `${big}// ${v} ${mib}\n` }] })
      await c.request("textDocument/formatting", doc(use))
    }
    const grown = rss() - before
    // One text per version would be 105 MiB. What the server holds is the document and the
    // text that waits for the child, and what it maps besides is what one message's handling
    // takes (the message, its parsed form, the copies on their way), which the allocator keeps
    // mapped for the next message: nine texts, 12 MiB, by the resident allocator's count. The
    // bound is twice that.
    check("a child that does not read: the server holds one text per document", grown < 24 * 1024 * 1024, { grownMiB: grown / 1048576 })
    const children = execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).map(Number)
    await sleep(400)
    const started = Date.now()
    const shut = await c.request("shutdown", null, { timeout: 5000 })
    check("shutdown answered while a child does not read", shut.result === null && Date.now() - started < 4000, { ms: Date.now() - started })
    const after = await c.request("textDocument/hover", { ...doc(use), position: { line: 0, character: 0 } }, { timeout: 5000 })
    check("a request after shutdown: InvalidRequest", after.error?.code === -32600, after)
    c.notify("exit", null)
    const code = await Promise.race([c.exited, sleep(5000).then(() => "timeout")])
    check("exit with a child that does not read", code === 0, code)
    const alive = children.filter((pid) => {
      try {
        process.kill(pid, 0)
        return true
      } catch {
        return false
      }
    })
    check("exit straight after shutdown ends the children", children.length > 0 && alive.length === 0, { children, alive })
  } catch (e) {
    check("the busy child scenario ran to its end", false, e.stack)
    c.proc.kill()
  }
}

// --- A large document closed under traffic --------------------------------------------------------

async function closedDocument() {
  const root = join(work, "closed")
  cpSync(bare, root, { recursive: true })
  const file = join(root, "Hello.scala")
  const c = new Client(root)
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: {} })
    c.notify("initialized", {})
    const hover = () => c.request("textDocument/hover", { ...doc(file), position: { line: 0, character: 0 } })
    await hover()
    const rss = () => Number(execSync(`ps -o rss= -p ${c.proc.pid}`).toString().trim()) * 1024
    const before = rss()
    const text = readFileSync(file, "utf-8") + "// " + "x".repeat(4 * 1024 * 1024) + "\n"
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text } })
    await hover()
    const open = rss() - before
    c.notify("textDocument/didClose", { textDocument: { uri: uriOf(file) } })
    // Requests a tenth of a second apart, so that the server's receive never times out.
    for (let i = 0; i < 40; i++) {
      await hover()
      await sleep(100)
    }
    const closed = rss() - before
    check("a large document open: the server holds its text", open > text.length, { openMiB: open / 1048576 })
    check("a large document closed under traffic: its texts go back to the system", closed < text.length, { openMiB: open / 1048576, closedMiB: closed / 1048576 })
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the closed document scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- A document closed under an alias of its deleted path ------------------------------------
//
// The file deleted on disk, then closed under the URI it was opened with, which no longer canonicalises
// (here a symbolic link; on Windows a short name): the close must find the document all the same, so
// that the file made again with other text is what the server answers from.
async function closedAlias() {
  const root = join(work, "link/alias")
  cpSync(bare, root, { recursive: true })
  const file = join(root, "Hello.scala")
  const c = new Client(root)
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: {} })
    c.notify("initialized", {})
    const text = readFileSync(file, "utf-8")
    const lines = text.split("\n")
    const at = lines.findIndex((l) => l.includes("def greet"))
    const hover = () => c.request("textDocument/hover", { ...doc(file), position: { line: at, character: lines[at].indexOf("greet") } })
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text } })
    let h
    for (const until = Date.now() + 20000; Date.now() < until && !JSON.stringify(h ?? "").includes("String");) { h = await hover(); await sleep(200) }
    rmSync(file)
    c.notify("textDocument/didClose", { textDocument: { uri: uriOf(file) } })
    await sleep(300)
    const again = text.replace('def greet(name: String): String = "hello " + name', "def greet(name: String): Int = 123").replace('val wrong: Int = "not an int"', "val wrong: Int = 123")
    writeFileSync(file, again)
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(file), type: 1 }] })
    h = undefined
    for (const until = Date.now() + 20000; Date.now() < until && !JSON.stringify(h ?? "").includes("Int");) { h = await hover(); await sleep(300) }
    check("a document closed under an alias of its deleted path and made again: the server answers from the new text", JSON.stringify(h ?? "").includes("Int") && !JSON.stringify(h).includes("String ="), h)
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the closed alias scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- A build a newer edit overtakes ------------------------------------------------------------
//
// The server and its sessions trace each build (TEQ_LSP_TRACE_FILE) and the sessions hold a
// build of named files at its start, its commit point or before its answer while the barrier's
// file is there (TEQ_LSP_TEST_BARRIER): the driver forces each schedule, never sleeping for it.

/** The trace's lines: `<who> <pid> <ms> <event> <build or query> <more>...`. */
function traceOf(file) {
  if (!existsSync(file)) return []
  return readFileSync(file, "utf-8").split("\n").filter(Boolean).map((line) => {
    const [who, pid, ms, event, id, ...rest] = line.split(" ")
    return { who, pid: Number(pid), ms: Number(ms), event, id: Number(id), rest }
  })
}
/** The first line of the trace from `from` on that `pred` accepts, within `timeout`. */
async function traced(file, from, pred, what, timeout = 60000) {
  let found
  if (!(await until(() => (found = traceOf(file).slice(from).find(pred)) !== undefined, timeout))) throw new Error(`no ${what} in the trace: ${JSON.stringify(traceOf(file).slice(from))}`)
  return found
}
/** The range of the `n`th `needle` in `text`. */
function rangeIn(text, needle, n = 0, delta = 0, length = needle.length - delta) {
  let i = -1
  for (let k = 0; k <= n; k++) i = text.indexOf(needle, i + 1)
  return { start: positionAt(text, i + delta), end: positionAt(text, i + delta + length) }
}

async function overtaken() {
  const root = join(work, "link/overtaken")
  mkdirSync(root, { recursive: true })
  const edit = join(root, "Edit.scala")
  writeFileSync(edit, "object Edit:\n  def value: Int = 0\n  def twice: Int = value * 2\n")
  writeFileSync(join(root, "Use.scala"), "object Use:\n  val v: Int = Edit.twice\n")
  const trace = join(work, "overtaken-trace")
  const barrier = join(work, "overtaken-barrier")
  const c = new Client(root, { TEQ_LSP_TRACE_FILE: trace, TEQ_LSP_TEST_BARRIER: barrier })
  const has = (who, event, id) => traceOf(trace).some((e) => e.who === who && e.event === event && e.id === id)
  const held = (point) => (e) => e.who === "session" && e.event === "held" && e.rest[0] === point
  const ofEdit = (m) => c.diagnosticsSince(m).filter((p) => p.uri === uriOf(edit))
  let version = 1
  const change = (text) => c.notify("textDocument/didChange", { textDocument: { uri: uriOf(edit), version: ++version }, contentChanges: [{ text }] })
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: {} })
    // The document opened before the session's child starts is typed by its first build: the
    // startup's disk build and the build of the open texts are one.
    const t1 = 'object Edit:\n  def value: Int = "one"\n  def twice: Int = value * 2\n'
    let m = c.mark()
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(edit), languageId: "scala", version: 1, text: t1 } })
    c.notify("initialized", {})
    let d = await c.waitDiagnostics(m, uriOf(edit), (ds) => ds.length > 0, 60000)
    let t = traceOf(trace)
    const firstPublished = t.findIndex((e) => e.who === "server" && e.event === "published")
    const startup = t.slice(0, firstPublished).filter((e) => e.who === "session" && e.event === "start")
    check(
      "the startup's builds as one: the first build types the text opened before it",
      startup.length === 1 && ofEdit(m)[0]?.version === 1 && d?.length === 1 && same(d[0].range, rangeIn(t1, '"one"')),
      { startup, published: ofEdit(m) },
    )

    // An edit while the build of the text before waits at its start: the build is cancelled
    // before it types, and the next types the latest text, whose diagnostics alone are published.
    writeFileSync(`${barrier}.start`, "")
    let from = traceOf(trace).length
    m = c.mark()
    const t2 = "object Edit:\n  def value: Int = true\n  def twice: Int = value * 2\n"
    change(t2)
    const early = await traced(trace, from, held("start"), "build held at its start")
    const t3 = 'object Edit:\n  val pad = 0\n  def value: Int = "three"\n  def twice: Int = value * 2\n'
    change(t3)
    await traced(trace, from, (e) => e.event === "wrote-cancel" && e.id === early.id, "cancel written")
    rmSync(`${barrier}.start`)
    d = await c.waitDiagnostics(m, uriOf(edit), (ds) => ds.length === 1 && same(ds[0].range, rangeIn(t3, '"three"')), 60000)
    check(
      "an edit while the build waits at its start: cancelled before typing, the latest text typed and published alone",
      d !== undefined && has("session", "cancelled", early.id) && has("server", "cancelled", early.id) && !has("session", "answered", early.id) && same(ofEdit(m).map((p) => p.version), [3]),
      { early, published: ofEdit(m), trace: traceOf(trace).slice(from) },
    )

    // An edit once the build began typing: it runs to its end and its answer is dropped as stale;
    // its cancel, read after it, stops nothing, and the next build publishes the latest text's.
    writeFileSync(`${barrier}.answer`, "")
    from = traceOf(trace).length
    m = c.mark()
    const t4 = "object Edit:\n  def value: Int = 'c'\n  def twice: Int = value * 2\n"
    change(t4)
    const typing = await traced(trace, from, held("answer"), "build held before its answer")
    const t5 = "object Edit:\n  val pad = 0\n  val more = 1\n  def value: Int = 5\n  def twice: Int = value * 2\n"
    change(t5)
    await traced(trace, from, (e) => e.event === "wrote-cancel" && e.id === typing.id, "cancel written")
    rmSync(`${barrier}.answer`)
    const next = await traced(trace, from, (e) => e.who === "server" && e.event === "published" && e.id > typing.id, "the next build published")
    d = await c.waitDiagnostics(m, uriOf(edit), (ds) => ds.length === 0, 10000)
    check(
      "an edit after typing began: the build runs on, its answer dropped, the latest text's published",
      d !== undefined && has("session", "answered", typing.id) && has("server", "dropped", typing.id) && same(ofEdit(m).map((p) => p.version), [5]),
      { typing, published: ofEdit(m), trace: traceOf(trace).slice(from) },
    )
    check(
      "a cancel read after its build answered is ignored and stops not the next",
      has("session", "ignored-cancel", typing.id) && has("session", "answered", next.id) && !has("session", "cancelled", next.id),
      traceOf(trace).slice(from),
    )

    // A query asked while a build of an older text is under way: its offset is in the latest
    // text, so it is asked after the build that sends that text.
    writeFileSync(`${barrier}.answer`, "")
    from = traceOf(trace).length
    const t6 = "object Edit:\n  def value: Int = 6\n  def twice: Int = value * 2\n"
    change(t6)
    const behind = await traced(trace, from, held("answer"), "build held before its answer")
    const t7 = "object Edit:\n  // one\n  // two\n  def value: Int = 7\n  def twice: Int = value * 2\n"
    change(t7)
    const asked = c.result("textDocument/definition", { ...doc(edit), position: positionAt(t7, t7.indexOf("value * 2")) })
    await traced(trace, from, (e) => e.who === "server" && e.event === "deferred", "the query deferred")
    rmSync(`${barrier}.answer`)
    const def = await asked
    check(
      "a query behind a stale build waits for the build of its text",
      sameLocs(def, [{ uri: uriOf(edit), range: rangeIn(t7, "def value", 0, 4) }]) && has("server", "dropped", behind.id),
      { def, trace: traceOf(trace).slice(from) },
    )

    // A query written behind a build that is then cancelled: the program it was to read is never
    // made, so it is asked again after the next build, whose program answers it.
    writeFileSync(`${barrier}.start`, "")
    from = traceOf(trace).length
    const t8 = "object Edit:\n  def value: Int = 8\n  def twice: Int = value * 2\n"
    change(t8)
    const cancelledBuild = await traced(trace, from, held("start"), "build held at its start")
    const use = join(root, "Use.scala")
    const behindCancelled = c.result("textDocument/definition", at_(use, "Edit.twice", 0, 5))
    const t9 = "object Edit:\n  // a\n  // b\n  // c\n  def value: Int = 9\n  def twice: Int = value * 2\n"
    change(t9)
    await traced(trace, from, (e) => e.event === "wrote-cancel" && e.id === cancelledBuild.id, "cancel written")
    rmSync(`${barrier}.start`)
    const twice = await behindCancelled
    const order = traceOf(trace).slice(from).filter((e) => e.who === "server" && ["cancelled", "deferred"].includes(e.event)).map((e) => e.event)
    check(
      "a query behind a cancelled build is asked again after the next build",
      sameLocs(twice, [{ uri: uriOf(edit), range: rangeIn(t9, "def twice", 0, 4) }]) && same(order, ["cancelled", "deferred"]),
      { twice, trace: traceOf(trace).slice(from) },
    )

    // A closed document's diagnostics after its session's child started anew: the new child
    // typed the disk's text, the latest, and what it finds is published.
    c.notify("textDocument/didClose", doc(edit))
    from = traceOf(trace).length
    await traced(trace, from, (e) => e.who === "server" && e.event === "published", "the build after the close")
    const sessionPid = traceOf(trace).filter((e) => e.who === "session").pop().pid
    m = c.mark()
    process.kill(sessionPid, "SIGKILL")
    const ended = await until(() => c.diagnosticsSince(m).some((p) => p.diagnostics.some((x) => x.message.includes("session ended"))), 10000)
    const onDisk = 'object Edit:\n  def value: Int = "disk"\n  def twice: Int = value * 2\n'
    writeFileSync(edit, onDisk)
    m = c.mark()
    await c.result("textDocument/hover", at_(use, "Edit.twice", 0, 5))
    d = await c.waitDiagnostics(m, uriOf(edit), (ds) => ds.length === 1, 30000)
    const closedPublished = c.diagnosticsSince(m).filter((p) => p.uri === uriOf(edit))
    check(
      "a closed document's diagnostics after its session's child started anew",
      ended && d !== undefined && same(d[0].range, rangeIn(onDisk, '"disk"')) && closedPublished.every((p) => p.version === undefined),
      { ended, closedPublished },
    )

    // A query while a build of every file waits behind the build under way (a file changed on
    // disk, the watched files told): its offset is in the disk's new text, so it is asked after
    // the build that reads it.
    writeFileSync(`${barrier}.answer`, "")
    from = traceOf(trace).length
    const t10 = "object Edit:\n  def value: Int = 10\n  def twice: Int = value * 2\n"
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(edit), languageId: "scala", version: 10, text: t10 } })
    await traced(trace, from, held("answer"), "build held before its answer")
    const useText = "object Use:\n  // one\n  // two\n  val v: Int = Edit.twice\n"
    writeFileSync(use, useText)
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(use), type: 2 }] })
    const behindPlain = c.result("textDocument/definition", { ...doc(use), position: positionAt(useText, useText.indexOf("twice")) })
    await traced(trace, from, (e) => e.who === "server" && e.event === "deferred", "the query deferred")
    rmSync(`${barrier}.answer`)
    const twiceAgain = await behindPlain
    check("a query behind a build of every file asked for waits for it", sameLocs(twiceAgain, [{ uri: uriOf(edit), range: rangeIn(t10, "def twice", 0, 4) }]), { twiceAgain, trace: traceOf(trace).slice(from) })
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
  } catch (e) {
    check("the overtaken build scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  } finally {
    for (const point of ["start", "commit", "answer"]) rmSync(`${barrier}.${point}`, { force: true })
  }

  // A file of two sessions, one of which builds an older text of it: its part is left out of the
  // union, so that what the other typed from the latest text is published at once.
  const root2 = join(work, "link/overtaken2")
  const shared = join(root2, "shared/S.scala")
  mkdirSync(dirname(shared), { recursive: true })
  for (const p of ["one", "two"]) {
    mkdirSync(join(root2, p), { recursive: true })
    writeFileSync(join(root2, p, `${p}.scala`), `object ${p === "one" ? "One" : "Two"}:\n  val s: Int = S.v\n`)
  }
  writeFileSync(shared, "object S:\n  def v: Int = 1\n")
  writeExport(join(root2, "teq.lock"), { one: { platform: "js", sources: ["shared", "one"] }, two: { platform: "js", sources: ["shared", "two"] } })
  // A trace of its own: the identities are each server's.
  const trace2 = join(work, "overtaken-trace2")
  const c2 = new Client(root2, { TEQ_LSP_TRACE_FILE: trace2, TEQ_LSP_TEST_BARRIER: barrier })
  const has2 = (who, event, id) => traceOf(trace2).some((e) => e.who === who && e.event === event && e.id === id)
  try {
    await c2.result("initialize", { processId: null, rootUri: uriOf(root2), capabilities: {} })
    c2.notify("initialized", {})
    let m = c2.mark()
    const s1 = 'object S:\n  def v: Int = "s"\n'
    c2.notify("textDocument/didOpen", { textDocument: { uri: uriOf(shared), languageId: "scala", version: 1, text: s1 } })
    const both = await c2.waitDiagnostics(m, uriOf(shared), (ds) => ds.length === 1, 60000)
    const twoBuild = await traced(trace2, 0, (e) => e.who === "server" && e.event === "build" && e.rest[0] === "two/compile", "two's first build")
    await traced(trace2, 0, (e) => e.who === "server" && e.event === "published" && e.id === twoBuild.id, "two's first build published")
    const twoPid = traceOf(trace2).find((e) => e.who === "session" && e.event === "start" && e.id === twoBuild.id).pid
    const opened = traceOf(trace2).filter((e) => e.who === "session" && e.event === "start").length
    writeFileSync(`${barrier}.answer`, `${twoPid}\n`)
    const from = traceOf(trace2).length
    m = c2.mark()
    c2.notify("textDocument/didChange", { textDocument: { uri: uriOf(shared), version: 2 }, contentChanges: [{ text: "object S:\n  def v: Int = 2\n" }] })
    const twoHeld = await traced(trace2, from, (e) => e.event === "held" && e.pid === twoPid, "two's build held")
    const clean = await c2.waitDiagnostics(m, uriOf(shared), (ds) => ds.length === 0, 60000)
    const whileHeld = !has2("session", "released", twoHeld.id)
    rmSync(`${barrier}.answer`)
    await traced(trace2, from, (e) => e.who === "server" && e.event === "published" && e.id === twoHeld.id, "two's build published")
    const after = c2.diagnosticsSince(m).filter((p) => p.uri === uriOf(shared))
    check(
      "a file of two sessions: one session's stale part left out of the union",
      both?.length === 1 && clean !== undefined && whileHeld && same(after.map((p) => [p.version, p.diagnostics.length]), [[2, 0]]),
      { both, after, trace: traceOf(trace2).slice(from) },
    )
    check("a document opened as its sessions start: one build each", opened === 2, traceOf(trace2))
    await c2.result("shutdown", null)
    c2.notify("exit", null)
    await Promise.race([c2.exited, sleep(5000)])
  } catch (e) {
    check("the shared file scenario ran to its end", false, `${e.stack}\nstderr: ${c2.stderr.slice(-1000)}`)
    c2.proc.kill()
  } finally {
    for (const point of ["start", "commit", "answer"]) rmSync(`${barrier}.${point}`, { force: true })
  }
  // A blank line after an identified build: the session looks for its cancel without waiting
  // for a command that has not come.
  const lone = join(work, "link/overtaken3")
  mkdirSync(lone, { recursive: true })
  const a = join(lone, "A.scala")
  writeFileSync(a, "object A:\n  def a: Int = 1\n")
  const session = spawn(teq, ["compiler", "watch", "--check", "--index", lone], { cwd: lone, stdio: ["pipe", "pipe", "inherit"] })
  let answers = ""
  session.stdout.on("data", (d) => (answers += d))
  try {
    const lines = () => answers.split("\n").filter(Boolean)
    await until(() => lines().length === 1, 60000)
    const text = "object A:\n  def a: Int = 2\n"
    session.stdin.write(`text ${a} ${Buffer.byteLength(text)}\n${text}build #1 ${a}\n\n\n`)
    const answered = await until(() => lines().length === 2, 10000)
    check("a blank line after an identified build waits for nothing", answered && JSON.parse(lines()[1]).build === 1, lines())
  } finally {
    session.stdin.end("quit\n")
  }

  const all = [...traceOf(trace), ...traceOf(trace2)]
  const count = (who, event) => all.filter((e) => e.who === who && e.event === event).length
  console.log(`overtaken builds: ${count("session", "start")} started, ${count("session", "cancelled")} cancelled before typing, ${count("server", "dropped")} answers dropped as stale`)
}

// A query an edit of another file came before, a withdrawal held back while the other owner was
// stale, and a command not all there when a build looks for its cancel.
/** Completions against cancelled builds: one asked behind a build that an edit cancels answers
 * `ContentModified`, its revision never typed; one asked on an edit while an older build waits is
 * answered from the build of its own text. */
async function completionRaces() {
  const root = join(work, "link/completion-races")
  mkdirSync(root, { recursive: true })
  const file = join(root, "Test.scala")
  const t1 = "object Test:\n  def oldMember: Int = 1\n  def result: Int = oldMember\n"
  writeFileSync(file, t1)
  const trace = join(work, "completion-races-trace")
  const barrier = join(work, "completion-races-barrier")
  const held = (point) => (e) => e.who === "session" && e.event === "held" && e.rest[0] === point
  const c = new Client(root, { TEQ_LSP_TRACE_FILE: trace, TEQ_LSP_TEST_BARRIER: barrier })
  let version = 1
  const change = (text) => c.notify("textDocument/didChange", { textDocument: { uri: uriOf(file), version: ++version }, contentChanges: [{ text }] })
  const completeAt = (text, needle) => c.request("textDocument/completion", { ...doc(file), position: positionAt(text, text.indexOf(needle.replace("|", "")) + needle.indexOf("|")) }, { timeout: 20000 }).catch((e) => ({ error: { message: e.message } }))
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { textDocument: { completion: { completionItem: { insertReplaceSupport: true } } } } })
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text: t1 } })
    await c.result("textDocument/documentSymbol", doc(file))
    // Asked behind a build that the next edit cancels: that revision is never typed.
    writeFileSync(`${barrier}.start`, "")
    let from = traceOf(trace).length
    const t2 = t1.replace("= 1", "= 2")
    change(t2)
    await traced(trace, from, held("start"), "build held at its start")
    const stale = completeAt(t2, "result: Int = old|Member")
    await sleep(200)
    change(t2.replaceAll("oldMember", "newMember"))
    await traced(trace, from, (e) => e.event === "wrote-cancel", "cancel written")
    rmSync(`${barrier}.start`)
    const a = await stale
    check("a completion behind a build an edit cancels: ContentModified, not the later text's items", a.error?.code === -32801, a)
    // Asked on an edit while an older build waits: answered once its own text's build is sent.
    writeFileSync(`${barrier}.start`, "")
    from = traceOf(trace).length
    const t4 = t2.replaceAll("oldMember", "newMember").replace("= 2", "= 4")
    change(t4)
    await traced(trace, from, held("start"), "build held at its start")
    const t5 = t4.replaceAll("newMember", "newerMember")
    change(t5)
    const fresh = completeAt(t5, "result: Int = newer|Member")
    await traced(trace, from, (e) => e.event === "wrote-cancel", "cancel written")
    rmSync(`${barrier}.start`)
    const b = await fresh
    check("a completion on an edit while an older build waits: answered from its own text's build", (b.result?.items ?? []).some((i) => i.label === "newerMember"), b)
    // Asked behind a build that the document's close cancels: the disk's text never answers it.
    writeFileSync(`${barrier}.start`, "")
    from = traceOf(trace).length
    const t6 = t5.replace("= 4", "= 6")
    change(t6)
    await traced(trace, from, held("start"), "build held at its start")
    const closed = completeAt(t6, "result: Int = newer|Member")
    await sleep(200)
    c.notify("textDocument/didClose", doc(file))
    await traced(trace, from, (e) => e.event === "wrote-cancel", "cancel written")
    rmSync(`${barrier}.start`)
    const d = await closed
    check("a completion behind a build its document's close cancels: ContentModified, not the disk text's items", d.error?.code === -32801, d)
    await c.result("shutdown", null)
    c.notify("exit", null)
  } catch (e) {
    check("the completion races ran to their end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  } finally {
    rmSync(`${barrier}.start`, { force: true })
  }
}

async function overtakenAgain() {
  const trace = join(work, "again-trace")
  const barrier = join(work, "again-barrier")
  const held = (point) => (e) => e.who === "session" && e.event === "held" && e.rest[0] === point

  // A query about an unchanged file, and a workspace query, asked after an edit of another file
  // while a build is under way: they read the program of that edit.
  const root = join(work, "link/again")
  mkdirSync(root, { recursive: true })
  const a = join(root, "A.scala")
  const use = join(root, "Use.scala")
  const at1 = "object A:\n  def oldName: Int = 1\n"
  const useText = "object Use:\n  def v: Int = A.newName\n"
  writeFileSync(a, at1)
  writeFileSync(use, useText)
  const c = new Client(root, { TEQ_LSP_TRACE_FILE: trace, TEQ_LSP_TEST_BARRIER: barrier })
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: {} })
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(a), languageId: "scala", version: 1, text: at1 } })
    await c.result("textDocument/documentSymbol", doc(a))
    writeFileSync(`${barrier}.answer`, "")
    const from = traceOf(trace).length
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(a), version: 2 }, contentChanges: [{ text: at1.replace("= 1", "= 2") }] })
    await traced(trace, from, held("answer"), "build held before its answer")
    const at3 = at1.replace("oldName", "newName")
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(a), version: 3 }, contentChanges: [{ text: at3 }] })
    const symbols = c.result("workspace/symbol", { query: "newName" })
    const definition = c.result("textDocument/definition", { ...doc(use), position: positionAt(useText, useText.indexOf("newName")) })
    await traced(trace, from, (e) => e.event === "wrote-cancel", "cancel written")
    rmSync(`${barrier}.answer`)
    const [found, def] = [await symbols, await definition]
    const declared = { uri: uriOf(a), range: rangeIn(at3, "def newName", 0, 4) }
    check(
      "a query about an unchanged file and a workspace query, after an edit of another file: the edit's program answers",
      found?.some((x) => x.name === "newName" && same(x.location, declared)) && sameLocs(def, [declared]),
      { found, def, trace: traceOf(trace).slice(from) },
    )
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
  } catch (e) {
    check("the query after another file's edit ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  } finally {
    rmSync(`${barrier}.answer`, { force: true })
  }

  // A closed file of two sessions whose withdrawal was held back while the other owner was stale:
  // the next build answered publishes it, the empty union clearing the error.
  const shared = join(work, "link/again2")
  for (const p of ["shared", "one", "two"]) mkdirSync(join(shared, p), { recursive: true })
  const sFile = join(shared, "shared/S.scala")
  const sText = "object S:\n  def v: Int = V.value\n"
  writeFileSync(sFile, sText)
  writeFileSync(join(shared, "one/V.scala"), 'object V:\n  def value: String = "bad"\n')
  writeFileSync(join(shared, "two/V.scala"), "object V:\n  def value: Int = 1\n")
  writeExport(join(shared, "teq.lock"), { one: { platform: "js", sources: ["shared", "one"] }, two: { platform: "js", sources: ["shared", "two"] } })
  const trace2 = join(work, "again-trace2")
  const d = new Client(shared, { TEQ_LSP_TRACE_FILE: trace2, TEQ_LSP_TEST_BARRIER: barrier })
  try {
    await d.result("initialize", { processId: null, rootUri: uriOf(shared), capabilities: {} })
    d.notify("initialized", {})
    let m = d.mark()
    d.notify("textDocument/didOpen", { textDocument: { uri: uriOf(sFile), languageId: "scala", version: 1, text: sText } })
    const initial = await d.waitDiagnostics(m, uriOf(sFile), (ds) => ds.length === 1, 60000)
    const pid = {}
    for (const label of ["one/compile", "two/compile"]) {
      const build = await traced(trace2, 0, (e) => e.who === "server" && e.event === "build" && e.rest[0] === label, `${label}'s first build`)
      pid[label] = (await traced(trace2, 0, (e) => e.who === "session" && e.event === "start" && e.id === build.id, `${label}'s start`)).pid
    }
    writeFileSync(`${barrier}.answer`, "")
    const from = traceOf(trace2).length
    m = d.mark()
    d.notify("textDocument/didClose", doc(sFile))
    await until(() => traceOf(trace2).slice(from).filter((e) => e.event === "held").length === 2, 10000)
    process.kill(pid["one/compile"], "SIGKILL")
    await until(() => d.diagnosticsSince(m).some((p) => p.diagnostics.some((x) => x.message.includes("session ended"))), 10000)
    process.kill(pid["two/compile"], "SIGKILL")
    await until(() => !existsSync(`/proc/${pid["two/compile"]}`), 10000)
    rmSync(`${barrier}.answer`)
    // Only the clean owner starts again, asked about a file of its own.
    await d.result("textDocument/hover", { ...doc(join(shared, "two/V.scala")), position: { line: 1, character: 7 } })
    const cleared = await d.waitDiagnostics(m, uriOf(sFile), (ds) => ds.length === 0, 30000)
    check("a closed file's withdrawal held back while the other owner was stale is published with the next build", initial?.length === 1 && cleared !== undefined, { initial, published: d.diagnosticsSince(m), trace: traceOf(trace2).slice(from) })
    await d.result("shutdown", null)
    d.notify("exit", null)
    await Promise.race([d.exited, sleep(5000)])
  } catch (e) {
    check("the held back withdrawal ran to its end", false, `${e.stack}\nstderr: ${d.stderr.slice(-1000)}`)
    d.proc.kill()
  } finally {
    rmSync(`${barrier}.answer`, { force: true })
  }

  // A watch session whose build looks for its cancel while the next command is not all there: the
  // build answers, and the command is read whole in its turn (a line, a text's bytes, a build's
  // paths).
  const lone = join(work, "link/again3")
  mkdirSync(lone, { recursive: true })
  const la = join(lone, "A.scala")
  writeFileSync(la, "object A:\n  def x: Int = 1\n")
  const trace3 = join(work, "again-trace3")
  const session = spawn(teq, ["compiler", "watch", "--check", "--index", lone], { cwd: lone, env: { ...process.env, TEQ_LSP_TRACE_FILE: trace3, TEQ_LSP_TEST_BARRIER: barrier }, stdio: ["pipe", "pipe", "inherit"] })
  let out = ""
  session.stdout.on("data", (x) => (out += x))
  const lines = () => out.split("\n").filter(Boolean).map((l) => JSON.parse(l))
  try {
    await until(() => lines().length === 1, 60000)
    const textOf = (n) => `object A:\n  def x: Int = ${n}\n`
    const partials = [
      ["a line", "sta", "ts\n", (x) => x.stats !== undefined],
      ["a text's bytes", `text ${la} ${Buffer.byteLength(textOf(9))}\nobject A:`, `\n  def x: Int = 9\n`, null],
      ["a build's paths", `build #9 ${la}\n`, "\n", (x) => x.build === 9],
    ]
    let n = 2
    const outcomes = []
    for (const [what, start, rest, answer] of partials) {
      writeFileSync(`${barrier}.commit`, "")
      const from = traceOf(trace3).length
      const before = lines().length
      session.stdin.write(`text ${la} ${Buffer.byteLength(textOf(n))}\n${textOf(n)}build #${n} ${la}\n\n`)
      await traced(trace3, from, held("commit"), `build ${n} held at its commit point`)
      // Handed to the pipe before the build goes on.
      await new Promise((done) => session.stdin.write(start, done))
      rmSync(`${barrier}.commit`)
      const answered = await until(() => lines().length > before && lines()[before].build === n, 10000)
      session.stdin.write(rest)
      const after = lines().length
      const whole = answer === null || (await until(() => lines().length > after && answer(lines()[after]), 10000))
      outcomes.push({ what, answered, whole })
      n++
    }
    check("a command not all there when a build looks for its cancel: the build answers, the command is read whole", outcomes.every((o) => o.answered && o.whole), { outcomes, answers: lines().slice(1) })
  } catch (e) {
    check("the partial command scenario ran to its end", false, e.stack)
  } finally {
    rmSync(`${barrier}.commit`, { force: true })
    session.stdin.end("quit\n")
  }
}

// --- Libraries' sources ----------------------------

/** Projects reading libraries: two over scala-library and two versions of sourcecode, one over a
 * copy of sourcecode without its sources jar, and an upstream module whose products a downstream
 * one reads. A library's declaration is a document of its sources jar under the cache, named by
 * the sources jar; a product's is the producer's source. */
async function libraries() {
  const root = join(work, "link/libs")
  cpSync(join(here, "libs"), root, { recursive: true })
  const cache = join(work, "cache")
  mkdirSync(cache, { recursive: true })
  mkdirSync(join(root, "nosrc/lib"), { recursive: true })
  copyFileSync(jar, join(root, "nosrc/lib/sourcecode_3-0.4.2.jar"))
  const upOut = join(root, "up/out")
  // One worker, whatever the environment asks of the sessions: the joins tests/lsp.sh counts are theirs.
  execSync(`"${teq}" compiler build --target jvm --products "${upOut}" up/src --threads 1`, { cwd: root, stdio: "pipe" })
  // A jar of the coursier cache as the export pins it: its key, the coordinates of its path under
  // the repository, and the table's record, its sha1 and size (the path the Maven layout's).
  const pinned = (file) => {
    const data = readFileSync(file)
    const parts = file.slice(file.indexOf("/maven2/") + "/maven2/".length).split("/")
    const key = [parts.slice(0, -3).join("."), parts.at(-3), parts.at(-2)].join(":")
    if (parts.at(-1) !== `${parts.at(-3)}-${parts.at(-2)}.jar`) throw new Error(`${file} is not at the Maven layout of ${key}`)
    return { key, record: `maven-central ${createHash("sha1").update(data).digest("hex")} ${data.length}` }
  }
  writeExport(join(root, "teq.lock"), {
    p1: { sources: ["p1/src"], classpath: [pinned(jar), { file: "lib/liba-1.0.jar" }, { file: "lib/libb-1.0.jar" }] },
    p2: { sources: ["p2/src"], classpath: [pinned(jar2 ?? jar)] },
    // The same `libb` over another version of `liba`: one document of `libb`'s source for both.
    p3: { sources: ["p3/src"], classpath: [{ file: "lib/liba-2.0.jar" }, { file: "lib/libb-1.0.jar" }] },
    nosrc: { sources: ["nosrc/src"], classpath: [{ file: "nosrc/lib/sourcecode_3-0.4.2.jar" }] },
    up: { sources: ["up/src"] },
    down: { sources: ["down/src"], classpath: [{ file: "up/out" }] },
  })
  const p1 = join(root, "p1/src/P1.scala")
  const p2 = join(root, "p2/src/P2.scala")
  const p3 = join(root, "p3/src/P3.scala")
  const n = join(root, "nosrc/src/N.scala")
  const up = join(root, "up/src/u/Up.scala")
  const down = join(root, "down/src/d/Down.scala")
  const c = new Client(root, { TEQ_CACHE_DIR: cache })
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { general: { positionEncodings: ["utf-16"] } }, initializationOptions: { maxSessions: 8 } })
    c.notify("initialized", {})
    const attached = realpathSync(cache) + "/attached/"
    const docOf = (loc) => loc?.uri && fileURLToPath(loc.uri)
    const underCache = (loc, tail) => !!docOf(loc)?.startsWith(attached) && docOf(loc).endsWith(tail)
    // Two projects sharing a jar: one document of its source, the declaration's name in it.
    const list1 = await c.result("textDocument/definition", at_(p1, "List(1"))
    const listDoc = docOf(list1?.[0])
    check("definition into a jar: its sources jar's document", list1?.length === 1 && underCache(list1[0], "/scala-library-3.8.4-sources.jar/scala/collection/immutable/List.scala"), list1)
    check("definition into a jar: the declaration's name", listDoc && same(list1[0], at(listDoc, "object List extends", 0, 7, 4)), list1)
    const list2 = await c.result("textDocument/definition", at_(p2, "List(3"))
    check("two projects sharing a jar: one document", same(list1, list2), { list1, list2 })
    const hover = await c.result("textDocument/hover", at_(p1, "List(1"))
    check("hover into a jar", hover?.contents?.value?.includes("object scala.collection.immutable.List"), hover)
    const refs = await c.result("textDocument/references", { ...at_(p1, "List(1"), context: { includeDeclaration: true } })
    const listRefs = [list1?.[0], at(p1, "List(1", 0, 0, 4), at(p2, "List(3", 0, 0, 4)]
    // The document's own occurrences of the object (`= List`, `List.from`) come with the
    // program's, once the document is indexed (which the follow-up at the declaration does).
    const inDoc = (locs) => (locs ?? []).filter((l) => docOf(l) === listDoc)
    const docUses = (locs) => inDoc(locs).filter((l) => !same(l, list1?.[0]))
    check("references of a jar's member: the declaration, both projects' uses and the document's own", listRefs.every((l) => (refs ?? []).some((r) => same(r, l))) && docUses(refs).length > 0 && (refs ?? []).every((l) => listRefs.some((r) => same(r, l)) || docUses(refs).includes(l)), { found: refs, expected: listRefs })
    // Two versions of one jar: two documents.
    const line1 = await c.result("textDocument/definition", at_(p1, "sourcecode.Line", 0, 11))
    const line2 = await c.result("textDocument/definition", at_(p2, "sourcecode.Line", 0, 11))
    check("definition into a jar's other source", line1?.length === 1 && underCache(line1[0], "/sourcecode/SourceContext.scala") && same(line1[0], at(docOf(line1[0]), "class Line", 0, 6, 4)), line1)
    check("two versions of one jar: two documents", jar2 === undefined || (line2?.length === 1 && docOf(line2[0]) !== docOf(line1?.[0]) && same(line2[0], at(docOf(line2[0]), "class Line", 0, 6, 4))), { line1, line2 })
    // A jar without its sources jar: no location, the hover kept.
    const noDef = await c.request("textDocument/definition", at_(n, "sourcecode.Line", 0, 11))
    check("a jar without sources: no location", noDef.result === null && !noDef.error, noDef)
    const noHover = await c.result("textDocument/hover", at_(n, "sourcecode.Line", 0, 11))
    check("a jar without sources: the hover kept", noHover?.contents?.value?.includes("class sourcecode.Line"), noHover)
    // An upstream module's source through its products' manifest, and references both ways.
    const twice = await c.result("textDocument/definition", at_(down, "twice(21"))
    check("definition into an upstream module's source", sameLocs(twice, [at(up, "def twice", 0, 4, 5)]), twice)
    const twiceRefs = [at(up, "def twice", 0, 4, 5), at(down, "twice(21", 0, 0, 5)]
    let r = await c.result("textDocument/references", { ...at_(down, "twice(21"), context: { includeDeclaration: true } })
    check("references from a downstream module", sameLocs(r, twiceRefs), { found: r, expected: twiceRefs })
    r = await c.result("textDocument/references", { ...at_(up, "def twice", 0, 4), context: { includeDeclaration: true } })
    check("references from the upstream module include the downstream's", sameLocs(r, twiceRefs), { found: r, expected: twiceRefs })
    // The call hierarchy across the modules.
    const prep = await c.result("textDocument/prepareCallHierarchy", at_(down, "twice(21"))
    check("prepareCallHierarchy on an upstream method: its item in its source", prep?.length === 1 && prep[0].uri === uriOf(up) && same(prep[0].selectionRange, at(up, "def twice", 0, 4, 5).range), prep)
    const incoming = await c.result("callHierarchy/incomingCalls", { item: prep?.[0] })
    check("incoming calls at the upstream declaration: the downstream caller", incoming?.length === 1 && incoming[0].from?.name === "run" && incoming[0].from.uri === uriOf(down) && same(incoming[0].fromRanges, [at(down, "twice(21", 0, 0, 5).range]), incoming)
    const runItem = await c.result("textDocument/prepareCallHierarchy", at_(down, "def run", 0, 4))
    const outgoing = await c.result("callHierarchy/outgoingCalls", { item: runItem?.[0] })
    check("outgoing calls into the upstream module", outgoing?.length === 1 && outgoing[0].to?.name === "twice" && outgoing[0].to.uri === uriOf(up), outgoing)
    // The document opened, asked, changed and closed.
    if (listDoc) {
      const text = readFileSync(listDoc, "utf-8")
      const self = at(listDoc, "object List extends", 0, 7, 4)
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(listDoc), languageId: "scala", version: 1, text } })
      r = await c.result("textDocument/definition", at_(listDoc, "object List extends", 0, 7))
      check("a library document: its declaration answers itself", sameLocs(r, [self]), r)
      const h = await c.result("textDocument/hover", at_(listDoc, "object List extends", 0, 7))
      check("a library document: hover on a declaration", h?.contents?.value?.includes("object scala.collection.immutable.List"), h)
      r = await c.result("textDocument/references", { ...at_(listDoc, "object List extends", 0, 7), context: { includeDeclaration: false } })
      check("a library document: references in every project reading the jar, and its own", listRefs.slice(1).every((l) => (r ?? []).some((x) => same(x, l))) && docUses(r).length > 0 && (r ?? []).every((l) => listRefs.slice(1).some((x) => same(x, l)) || docUses(r).includes(l)), r)
      // Inside the document (the occurrence index): a use in the same file, of
      // the same jar's other files, a declaration the program never reached.
      const nil = await c.result("textDocument/definition", at_(listDoc, "def empty[A]: List[A] = Nil", 0, 24))
      check("a library document: a use of the same file's other class (`Nil`, never reached by the program)", sameLocs(nil, [at(listDoc, "case object Nil", 0, 12, 3)]), nil)
      const nilHover = await c.result("textDocument/hover", at_(listDoc, "def empty[A]: List[A] = Nil", 0, 24))
      check("a library document: hover on a use", nilHover?.contents?.value?.includes("object scala.collection.immutable.Nil") && same(nilHover.range, at(listDoc, "def empty[A]: List[A] = Nil", 0, 24, 3).range), nilHover)
      const lb = await c.result("textDocument/definition", at_(listDoc, "new ListBuffer()", 0, 4))
      const lbDoc = docOf(lb?.[0])
      check("a library document: a use of the same jar's other file (`ListBuffer`)", lb?.length === 1 && underCache(lb[0], "/scala-library-3.8.4-sources.jar/scala/collection/mutable/ListBuffer.scala") && same(lb[0], at(lbDoc, "class ListBuffer[A]", 0, 6, 10)), lb)
      const lbImport = await c.result("textDocument/definition", at_(listDoc, "import mutable.{Builder, ListBuffer}", 0, 25))
      check("a library document: an import's selector", sameLocs(lbImport, lb), lbImport)
      const nilDecl = await c.result("textDocument/definition", at_(listDoc, "case object Nil", 0, 12))
      check("a library document: the declaration of a class the program never reached answers itself", sameLocs(nilDecl, [at(listDoc, "case object Nil", 0, 12, 3)]), nilDecl)
      const nilRefs = await c.result("textDocument/references", { ...at_(listDoc, "def empty[A]: List[A] = Nil", 0, 24), context: { includeDeclaration: true } })
      check("a library document: references of a use hold its declaration and the document's uses", (nilRefs ?? []).some((l) => same(l, at(listDoc, "case object Nil", 0, 12, 3))) && (nilRefs ?? []).some((l) => same(l, at(listDoc, "def empty[A]: List[A] = Nil", 0, 24, 3))) && (nilRefs ?? []).every((l) => docOf(l) === listDoc), nilRefs)
      const that = await c.result("textDocument/definition", at_(listDoc, "that.isEmpty", 0, 5))
      check("a library document: a selection on a parameter", sameLocs(that, [at(listDoc, "def isEmpty", 0, 4, 7)]), that)
      const not = await c.result("textDocument/definition", at_(listDoc, "while (!that.isEmpty)", 0, 7))
      check("a library document: a prefix operator on an application, `unary_!` written `!`", not?.length === 1 && underCache(not[0], "/scala-library-3.8.4-sources.jar/scala/Boolean.scala") && same(not[0], at(docOf(not[0]), "def unary_! : Boolean", 0, 4, 7)), not)
      const localVal = await c.result("textDocument/hover", at_(listDoc, "val h = new ::(head, Nil)", 0, 4))
      check("a library document: hover on a local val of a body", localVal?.contents?.value?.includes("val h: ::[A]"), localVal)
      // A function value the document applies (`p(these.head)` of `takeWhile`), recorded from
      // the pickled trees: its parameter.
      const pDecl = at(listDoc, "takeWhile(p: A => Boolean)", 0, 10, 1)
      const pDef = await c.result("textDocument/definition", at_(listDoc, "p(these.head)"))
      const pHover = await c.result("textDocument/hover", at_(listDoc, "p(these.head)"))
      const pRefs = await c.result("textDocument/references", { ...at_(listDoc, "p(these.head)"), context: { includeDeclaration: true } })
      check("a library document: an applied function value, its parameter's definition, hover and references", sameLocs(pDef, [pDecl]) && pHover?.contents?.value?.includes("p: A => Boolean") && sameLocs(pRefs, [pDecl, at(listDoc, "p(these.head)", 0, 0, 1)]), { pDef, pHover, pRefs })
      c.notify("textDocument/didChange", { textDocument: { uri: uriOf(listDoc), version: 2 }, contentChanges: [{ text: "// changed\n" + text }] })
      const changed = await c.request("textDocument/definition", at_(listDoc, "object List extends", 0, 7))
      check("a library document changed: null", changed.result === null && !changed.error, changed)
      c.notify("textDocument/didClose", { textDocument: { uri: uriOf(listDoc) } })
      r = await c.result("textDocument/definition", at_(listDoc, "object List extends", 0, 7))
      check("a library document closed: answered again", sameLocs(r, [self]), r)
    }
    // A jar's method in the call hierarchy, then its document changed: no answer from the text
    // on disk.
    const maxItem = await c.result("textDocument/prepareCallHierarchy", at_(p1, "max(1", 0, 0))
    const maxDoc = maxItem?.[0]?.uri && fileURLToPath(maxItem[0].uri)
    check("prepareCallHierarchy on a jar's method: its item in its sources jar's document", !!maxDoc?.endsWith("/scala-library-3.8.4-sources.jar/scala/math/package.scala") && maxItem[0].name === "max", maxItem)
    const maxCallers = await c.result("callHierarchy/incomingCalls", { item: maxItem?.[0] })
    // The project's caller, and the callers in the documents indexed so far (`slice` of
    // `List.scala`, whose index the queries above built).
    const projectCallers = (maxCallers ?? []).filter((i) => !docOf(i.from)?.startsWith(attached))
    check("incoming calls of a jar's method: the project's caller, and the indexed documents'", projectCallers.length === 1 && projectCallers[0].from?.name === "larger" && (maxCallers ?? []).every((i) => projectCallers.includes(i) || underCache(i.from, "/scala-library-3.8.4-sources.jar/scala/collection/immutable/List.scala")), maxCallers)
    if (maxDoc) {
      const text = readFileSync(maxDoc, "utf-8")
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(maxDoc), languageId: "scala", version: 1, text } })
      // Changed past its declarations, so that the item's position is still the declaration's.
      c.notify("textDocument/didChange", { textDocument: { uri: uriOf(maxDoc), version: 2 }, contentChanges: [{ text: text + "\n// changed\n" }] })
      const changedIn = await c.request("callHierarchy/incomingCalls", { item: maxItem[0] })
      const changedOut = await c.request("callHierarchy/outgoingCalls", { item: maxItem[0] })
      check("the call hierarchy on a changed library document: null", changedIn.result === null && changedOut.result === null && !changedIn.error && !changedOut.error, { changedIn, changedOut })
      c.notify("textDocument/didClose", { textDocument: { uri: uriOf(maxDoc) } })
    }
    if (listDoc) {
      // The indexed document's occurrences of another document's class (`ListBuffer`, asked
      // from the project after the call hierarchy above, whose callers its index would add to),
      // the file damaged since: made whole before they are answered, left out while it cannot be.
      const lbRefs = () => c.result("textDocument/references", { ...at_(p1, "ListBuffer[Int]"), context: { includeDeclaration: false } })
      const before = await lbRefs()
      const original = readFileSync(listDoc)
      const spelled = (locs) => inDoc(locs).every((l) => original.toString("utf-8").split("\n")[l.range.start.line].slice(l.range.start.character, l.range.end.character) === "ListBuffer")
      check("references of another document's class from a project: the project's use and the indexed document's", (before ?? []).some((l) => same(l, at(p1, "ListBuffer[Int]", 0, 0, 10))) && inDoc(before).length > 0 && spelled(before), before)
      const damage = () => {
        chmodSync(listDoc, 0o644)
        writeFileSync(listDoc, "// damaged\n")
      }
      const wholeList = () => readFileSync(listDoc).equals(original) && (statSync(listDoc).mode & 0o222) === 0
      damage()
      // A link where a session makes its partial file (`List.part<pid>`, every child's): the
      // repair never writes through it.
      const decoy = join(root, "decoy.txt")
      writeFileSync(decoy, "decoy\n")
      chmodSync(decoy, 0o644)
      const pids = execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean)
      for (const pid of pids) symlinkSync(decoy, listDoc.replace(/\.scala$/, `.part${pid}`))
      const repaired = await lbRefs()
      check("a damaged indexed document is made whole before its occurrences are answered", wholeList() && sameLocs(repaired, before), { repaired, before, now: readFileSync(listDoc, "utf-8").slice(0, 40) })
      check("the repair writes nothing through a link at its partial file's name", pids.length > 0 && readFileSync(decoy, "utf-8") === "decoy\n" && (statSync(decoy).mode & 0o777) === 0o644, { pids, decoy: readFileSync(decoy, "utf-8"), mode: (statSync(decoy).mode & 0o777).toString(8) })
      for (const pid of pids) rmSync(listDoc.replace(/\.scala$/, `.part${pid}`), { force: true })
      if (process.getuid?.() !== 0) {
        damage()
        chmodSync(dirname(listDoc), 0o555)
        const omitted = await lbRefs()
        const still = readFileSync(listDoc, "utf-8")
        chmodSync(dirname(listDoc), 0o755)
        const again = await lbRefs()
        check("an indexed document that cannot be made whole answers none of its occurrences, and does once it can", still === "// damaged\n" && sameLocs(omitted, (before ?? []).filter((l) => docOf(l) !== listDoc)) && wholeList() && sameLocs(again, before), { omitted, again })
      }
      // A query at the document whose file was damaged since its index was built, at the same
      // positions (every character but the line ends blanked): declined, the file made whole,
      // answered again after.
      const hoverAt = at_(listDoc, "object List extends", 0, 7)
      chmodSync(listDoc, 0o644)
      writeFileSync(listDoc, original.toString("utf-8").replace(/[^\n]/g, " "))
      const blank = await c.request("textDocument/hover", hoverAt)
      const blankWhole = wholeList()
      const hoverAgain = await c.result("textDocument/hover", hoverAt)
      check("a query at an indexed document whose file was damaged is declined, the file made whole, and answered after", blank.result === null && !blank.error && blankWhole && !!hoverAgain?.contents?.value?.includes("object scala.collection.immutable.List"), { blank, blankWhole, hoverAgain })
      // References at the same place, whose target is looked up for the follow-up as well as for
      // the answer: declined as the hover is, not answered by a second lookup after the repair.
      chmodSync(listDoc, 0o644)
      writeFileSync(listDoc, original.toString("utf-8").replace(/[^\n]/g, " "))
      const blankRefs = await c.request("textDocument/references", { ...hoverAt, context: { includeDeclaration: true } })
      check("references at an indexed document whose file was damaged are declined, the file made whole", blankRefs.result === null && !blankRefs.error && wholeList(), { blankRefs })
      // A pipe in the document's place: never opened, by the server for a position in it nor by
      // a session answering into it, which leaves its occurrences out and logs its path.
      rmSync(listDoc, { force: true })
      execSync(`mkfifo "${listDoc}"`)
      const logged = c.mark()
      const onPipe = await c.request("textDocument/hover", hoverAt)
      const intoPipe = await lbRefs()
      // Told once by each session that answered into it.
      const toldLines = c.notifications.slice(logged).filter((n) => n.method === "window/logMessage" && n.params.message.includes(listDoc)).map((n) => n.params.message)
      const toldPipe = toldLines.length > 0 && new Set(toldLines).size === toldLines.length
      const stillPipe = statSync(listDoc).isFIFO()
      rmSync(listDoc, { force: true })
      const afterPipe = await lbRefs()
      check(
        "a pipe at an indexed document's path is never opened: no answer at it, its occurrences left out and its path told, and the document written again once it goes",
        onPipe.result === null && stillPipe && sameLocs(intoPipe, (before ?? []).filter((l) => docOf(l) !== listDoc)) && toldPipe && wholeList() && sameLocs(afterPipe, before),
        { onPipe, intoPipe, toldLines, stillPipe, afterPipe },
      )
    }
    // The fixture jars (tests/lsp/libs/fixture): a document of one jar referring to another's,
    // a renamed import, a named argument, `this` and `super`, locals and parameters, a type
    // lambda, a refinement, an inline method, a context bound, a chain of selections.
    const twiceB = await c.result("textDocument/definition", at_(p1, "b.twiceHello", 0, 2))
    const bDoc = docOf(twiceB?.[0])
    check("definition into a fixture jar: its document", twiceB?.length === 1 && underCache(twiceB[0], "/libb-1.0-sources.jar/libb/B.scala") && same(twiceB[0], at(bDoc, "def twiceHello", 0, 4, 10)), twiceB)
    const helperFromApp = await c.result("textDocument/definition", at_(p1, "c.helper(x = 4)", 0, 2))
    const casesDoc = docOf(helperFromApp?.[0])
    check("definition into the fixture's other source", helperFromApp?.length === 1 && underCache(helperFromApp[0], "/libb-1.0-sources.jar/libb/Cases.scala") && same(helperFromApp[0], at(casesDoc, "def helper(x: Int = 1)", 0, 4, 6)), helperFromApp)
    const paramFromApp = await c.result("textDocument/definition", at_(p1, "c.helper(x = 4)", 0, 9))
    check("a named argument from a source: the jar method's parameter", casesDoc && sameLocs(paramFromApp, [at(casesDoc, "def helper(x: Int = 1)", 0, 11, 1)]), paramFromApp)
    if (bDoc && casesDoc) {
      for (const d of [bDoc, casesDoc]) c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(d), languageId: "scala", version: 1, text: readFileSync(d, "utf-8") } })
      let r = await c.result("textDocument/definition", at_(bDoc, "twiceHello + helper"))
      check("a fixture document: a use in the same file", sameLocs(r, [at(bDoc, "def twiceHello", 0, 4, 10)]), r)
      r = await c.result("textDocument/definition", at_(bDoc, "+ helper", 0, 2))
      check("a fixture document: a use of a private member", sameLocs(r, [at(bDoc, "private def helper", 0, 12, 6)]), r)
      r = await c.result("textDocument/definition", at_(bDoc, "extends A", 0, 8))
      const aDoc = docOf(r?.[0])
      check("a fixture document: a parent of another jar", r?.length === 1 && underCache(r[0], "/liba-1.0-sources.jar/liba/A.scala") && same(r[0], at(aDoc, "class A", 0, 6, 1)), r)
      r = await c.result("textDocument/definition", at_(bDoc, "import liba.A", 0, 12))
      check("a fixture document: an import of another jar's class", aDoc && sameLocs(r, [at(aDoc, "class A", 0, 6, 1)]), r)
      r = await c.result("textDocument/definition", at_(bDoc, "hello * 2"))
      check("a fixture document: an inherited member of another jar", aDoc && sameLocs(r, [at(aDoc, "def hello", 0, 4, 5)]), r)
      r = await c.result("textDocument/definition", at_(bDoc, "A.make", 0, 2))
      check("a fixture document: a member of another jar's object", aDoc && sameLocs(r, [at(aDoc, "def make", 0, 4, 4)]), r)
      r = await c.result("textDocument/definition", at_(bDoc, "def twiceHello: Int", 0, 16))
      const intDoc = docOf(r?.[0])
      check("a fixture document: a builtin class in scala-library's source", r?.length === 1 && underCache(r[0], "/scala-library-3.8.4-sources.jar/scala/Int.scala") && same(r[0], at(intDoc, "abstract class Int ", 0, 15, 3)), r)
      r = await c.result("textDocument/definition", at_(bDoc, "hello * 2", 0, 6))
      check("a fixture document: a primitive's operator, in the jar's declaration of the class", intDoc && sameLocs(r, [at(intDoc, "def *(x: Int): Int", 0, 4, 1)]), r)
      const starHover = await c.result("textDocument/hover", at_(bDoc, "hello * 2", 0, 6))
      check("a fixture document: hover on a primitive's operator", starHover?.contents?.value?.includes("def *(x: Int): Int"), starHover)
      const aHover = await c.result("textDocument/hover", at_(bDoc, "extends A", 0, 8))
      check("a fixture document: hover on a use of another jar's class", aHover?.contents?.value?.includes("class liba.A") && same(aHover.range, at(bDoc, "extends A", 0, 8, 1).range), aHover)
      r = await c.result("textDocument/references", { ...at_(bDoc, "extends A", 0, 8), context: { includeDeclaration: true } })
      // The documents indexed so far: B.scala's own, not yet Cases.scala's, asked below.
      const aRefs = aDoc && [at(aDoc, "class A", 0, 6, 1), at(bDoc, "import liba.A", 0, 12, 1), at(bDoc, "extends A", 0, 8, 1), at(bDoc, "def fresh: A", 0, 11, 1)]
      check("a fixture document: references of a use, the declaration and the document's occurrences", aRefs && sameLocs(r, aRefs), { found: r, expected: aRefs })
      r = await c.result("textDocument/definition", at_(bDoc, "def fresh", 0, 4))
      check("a fixture document: a declaration the program never used answers itself", sameLocs(r, [at(bDoc, "def fresh", 0, 4, 5)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "new LB[Int]()", 0, 4))
      check("a fixture document: a renamed import's alias at its use", r?.length === 1 && underCache(r[0], "/scala/collection/mutable/ListBuffer.scala") && same(r[0], at(docOf(r[0]), "class ListBuffer[A]", 0, 6, 10)), r)
      const lbAlias = await c.result("textDocument/definition", at_(casesDoc, "{ListBuffer as LB}", 0, 15))
      check("a fixture document: a renamed import's alias in the import", sameLocs(lbAlias, r), lbAlias)
      r = await c.result("textDocument/definition", at_(casesDoc, "helper(x = 2)", 0, 7))
      check("a fixture document: a named argument names the parameter", sameLocs(r, [at(casesDoc, "def helper(x: Int = 1)", 0, 11, 1)]), r)
      const xHover = await c.result("textDocument/hover", at_(casesDoc, "helper(x = 2)", 0, 7))
      check("a fixture document: hover on a named argument", xHover?.contents?.value?.includes("x: Int"), xHover)
      r = await c.result("textDocument/definition", at_(casesDoc, "this.helper()"))
      check("a fixture document: `this` is the enclosing class", sameLocs(r, [at(casesDoc, "class Cases", 0, 6, 5)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "this.helper()", 0, 5))
      check("a fixture document: a member selected from `this`", sameLocs(r, [at(casesDoc, "def helper(x: Int = 1)", 0, 4, 6)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "super.base", 0, 6))
      check("a fixture document: a member selected from `super`", sameLocs(r, [at(casesDoc, "def base", 0, 4, 4)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def inner(q: Int): Int = n + q", 0, 25))
      check("a fixture document: a local val of a body", sameLocs(r, [at(casesDoc, "val n = p", 0, 4, 1)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def inner(q: Int): Int = n + q", 0, 29))
      check("a fixture document: a parameter of a local method", sameLocs(r, [at(casesDoc, "def inner(q: Int)", 0, 10, 1)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "inner(p)"))
      check("a fixture document: a local method at its call", sameLocs(r, [at(casesDoc, "def inner(q: Int)", 0, 4, 5)]), r)
      const innerHover = await c.result("textDocument/hover", at_(casesDoc, "inner(p)"))
      check("a fixture document: hover on a local method", innerHover?.contents?.value?.includes("def inner(q: Int): Int"), innerHover)
      r = await c.result("textDocument/references", { ...at_(casesDoc, "def local(p: Int)", 0, 10), context: { includeDeclaration: true } })
      const pRefs = [at(casesDoc, "def local(p: Int)", 0, 10, 1), at(casesDoc, "val n = p", 0, 8, 1), at(casesDoc, "inner(p)", 0, 6, 1)]
      check("a fixture document: references of a parameter are its occurrences in the document", sameLocs(r, pRefs), { found: r, expected: pRefs })
      r = await c.result("textDocument/definition", at_(casesDoc, "x.Out = 1", 0, 2))
      check("a fixture document: a refinement's member", sameLocs(r, [at(casesDoc, "type Out = Int", 0, 5, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "[A] =>> List[A]", 0, 13))
      check("a fixture document: a type lambda's parameter", sameLocs(r, [at(casesDoc, "[A] =>> List[A]", 0, 1, 1)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "twice(helper(3))"))
      check("a fixture document: an inline method at its call", sameLocs(r, [at(casesDoc, "inline def twice", 0, 11, 5)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "twice(helper(3))", 0, 6))
      check("a fixture document: the argument of an inline call", sameLocs(r, [at(casesDoc, "def helper(x: Int = 1)", 0, 4, 6)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "[T: Ordering]", 0, 4))
      check("a fixture document: a context bound names its alias", r?.length === 1 && underCache(r[0], "/scala-library-3.8.4-sources.jar/scala/package.scala") && same(r[0], at(docOf(r[0]), "type Ordering[T] = scala.math.Ordering[T]", 0, 5, 8)), r)
      const boundHover = await c.result("textDocument/hover", at_(casesDoc, "[T: Ordering]", 0, 4))
      check("a fixture document: hover on a context bound", boundHover?.contents?.value?.includes("type Ordering"), boundHover)
      r = await c.result("textDocument/definition", at_(casesDoc, ".filter(_ > 2).sum", 0, 1))
      check("a fixture document: a selection on an application", r?.length === 1 && underCache(r[0], "/scala/collection/immutable/List.scala") && same(r[0], at(listDoc, "override def filter(p: A => Boolean)", 0, 13, 6)), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "A.make.hello", 0, 7))
      check("a fixture document: a selection on a selection of another jar", aDoc && sameLocs(r, [at(aDoc, "def hello", 0, 4, 5)]), r)
      const usesItem = await c.result("textDocument/prepareCallHierarchy", at_(casesDoc, "def uses: Int", 0, 4))
      check("a fixture document: prepareCallHierarchy on a library method", usesItem?.length === 1 && usesItem[0].name === "uses" && usesItem[0].uri === uriOf(casesDoc), usesItem)
      const usesOut = await c.result("callHierarchy/outgoingCalls", { item: usesItem?.[0] })
      const callees = (usesOut ?? []).map((o) => o.to?.name).sort()
      check("a fixture document: outgoing calls of a library method, from its document's index (`+` the jar's `Int.+`)", same(callees, ["+", "addOne", "base", "helper"]), usesOut)
      const helperItem = await c.result("textDocument/prepareCallHierarchy", at_(casesDoc, "def helper(x: Int = 1)", 0, 4))
      const helperIn = await c.result("callHierarchy/incomingCalls", { item: helperItem?.[0] })
      const callers = (helperIn ?? []).map((i) => i.from?.name).sort()
      check("a fixture document: incoming calls of a library method, the program's and the document's", same(callers, ["expanded", "m", "uses"]), helperIn)
      r = await c.result("textDocument/references", { ...at_(bDoc, "extends A", 0, 8), context: { includeDeclaration: true } })
      check("a fixture document: references of a use once the jar's other document is indexed hold its occurrences too", aRefs && sameLocs(r, [...aRefs, at(casesDoc, "import liba.A", 0, 12, 1)]), { found: r, expected: aRefs })
      // A selection on a branching expression names the member of the branches' join; a local
      // val's initializer's calls are its method's; a local method's call is a call, and a named
      // argument to it names its parameter.
      r = await c.result("textDocument/definition", at_(casesDoc, "(if flag then l else r).value", 0, 24))
      check("a fixture document: a selection on an `if` names the member of the branches' join", sameLocs(r, [at(casesDoc, "def value: Int = 0", 0, 4, 5)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "case false => r }).value", 0, 19))
      check("a fixture document: a selection on a match names the member of the cases' join", sameLocs(r, [at(casesDoc, "def value: Int = 0", 0, 4, 5)]), r)
      const callerItem = await c.result("textDocument/prepareCallHierarchy", at_(casesDoc, "def caller: Int", 0, 4))
      const callerOut = await c.result("callHierarchy/outgoingCalls", { item: callerItem?.[0] })
      check("a fixture document: a call in a local val's initializer belongs to the method", same((callerOut ?? []).map((o) => o.to?.name), ["helper2"]), callerOut)
      const helper2Item = await c.result("textDocument/prepareCallHierarchy", at_(casesDoc, "def helper2", 0, 4))
      const helper2In = await c.result("callHierarchy/incomingCalls", { item: helper2Item?.[0] })
      check("a fixture document: incoming calls through a local val's initializer", same((helper2In ?? []).map((i) => i.from?.name), ["caller"]), helper2In)
      const localCallItem = await c.result("textDocument/prepareCallHierarchy", at_(casesDoc, "def localCall: Int", 0, 4))
      const localCallOut = await c.result("callHierarchy/outgoingCalls", { item: localCallItem?.[0] })
      check("a fixture document: a call of a local method is a call", same((localCallOut ?? []).map((o) => o.to?.name), ["inner2"]) && same(localCallOut?.[0]?.to?.selectionRange, at(casesDoc, "def inner2", 0, 4, 6).range), localCallOut)
      const inner2Item = await c.result("textDocument/prepareCallHierarchy", at_(casesDoc, "inner2(x = 1)"))
      const inner2In = await c.result("callHierarchy/incomingCalls", { item: inner2Item?.[0] })
      check("a fixture document: incoming calls of a local method", same((inner2In ?? []).map((i) => i.from?.name), ["localCall"]), inner2In)
      r = await c.result("textDocument/definition", at_(casesDoc, "inner2(x = 1)", 0, 7))
      check("a fixture document: a named argument to a local method names its parameter", sameLocs(r, [at(casesDoc, "def inner2(x: Int)", 0, 11, 1)]), r)
      // The receiver's type as scalac has it: a join of one class applied differently keeps no
      // branch's arguments, a type parameter's bound and a refinement's parent carry their
      // arguments, an intersection selects in its more specific component, a user-defined
      // `asInstanceOf` is no cast.
      r = await c.result("textDocument/definition", at_(casesDoc, "(if flag then l else r).get.value", 0, 28))
      check("a fixture document: a selection on the join of one class applied differently makes no record", r === null, r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def bounded2[T <: Box[Left]](x: T): Int = x.get.value", 0, 50))
      check("a fixture document: a selection through a type parameter's bound keeps its arguments", sameLocs(r, [at(casesDoc, "override def value: Int = 1", 0, 13, 5)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def meet(x: Base & Left): Int = x.value", 0, 34))
      check("a fixture document: a selection on an intersection names the more specific component's member", sameLocs(r, [at(casesDoc, "override def value: Int = 1", 0, 13, 5)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def refinedBox(x: Box[Left] { def extra: Int }): Int = x.get.value", 0, 63))
      check("a fixture document: a selection through a refinement keeps the parent's arguments", sameLocs(r, [at(casesDoc, "override def value: Int = 1", 0, 13, 5)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "asInstanceOf[Int](1).value", 0, 21))
      check("a fixture document: a user-defined `asInstanceOf` is no cast", sameLocs(r, [at(casesDoc, "override def value: Int = 2", 0, 13, 5)]), r)
      // The member of a refined or an intersection receiver, as scalac's `findMember` has it:
      // a refinement's type member fixes the result, its value member
      // narrows it; of an intersection's two members the one whose type overrides the other's,
      // else the second component's, with the meet of the types.
      const leftValue = at(casesDoc, "override def value: Int = 1", 0, 13, 5)
      r = await c.result("textDocument/definition", at_(casesDoc, "def unrelated(x: Wide & Narrow): Int = x.get.value", 0, 41))
      check("a fixture document: an intersection of unrelated traits names the member whose type overrides", sameLocs(r, [at(casesDoc, "trait Narrow:\n  def get", 0, 20, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def unrelated(x: Wide & Narrow): Int = x.get.value", 0, 45))
      check("a fixture document: a selection on that member's narrowed type", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def unrelatedReverse(x: Narrow & Wide): Int = x.get.value", 0, 48))
      check("a fixture document: the reversed intersection names the second component's member, the joint one", sameLocs(r, [at(casesDoc, "trait Wide:\n  def get", 0, 18, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def unrelatedReverse(x: Narrow & Wide): Int = x.get.value", 0, 52))
      check("a fixture document: a selection on the joint member's meet type", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def refinedCarrier(x: Carrier { type Item = Left }): Int = x.get.value", 0, 65))
      check("a fixture document: a refinement's type member fixes a selection's result", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def refinedCarrier(x: Carrier { type Item = Left }): Int = x.get.value", 0, 61))
      check("a fixture document: the member through a type refinement is the parent's", sameLocs(r, [at(casesDoc, "def get: Item", 0, 4, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def refinedVal(x: Box[Base] { val get: Left }): Int = x.get.value", 0, 60))
      check("a fixture document: a refinement's value member narrows a selection's result", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def refinedVal(x: Box[Base] { val get: Left }): Int = x.get.value", 0, 56))
      check("a fixture document: the member through a value refinement is the parent's", sameLocs(r, [at(casesDoc, "class Box[+A](val get: A)", 0, 18, 3)]), r)
      // An opaque type is transparent inside the object that defines it, its bound outside.
      r = await c.result("textDocument/definition", at_(casesDoc, "def hidden(x: Hidden): Int = x.value", 0, 33))
      check("a fixture document: an opaque receiver inside its defining object is its right-hand side", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def opaqueUse(x: Opaques.Hidden): Int = x.value", 0, 44))
      check("a fixture document: an opaque receiver outside its defining object is its bound", sameLocs(r, [at(casesDoc, "def value: Int = 0", 0, 4, 5)]), r)
      // A refinement in a parameter's bound narrows the member's
      // result; a concrete constructor accessor outscores a deferred member; a this-type with a
      // self type orders the owners as the class's base classes do (the class's own first).
      r = await c.result("textDocument/definition", at_(casesDoc, "def boundedVal[T <: Box[Base] { val get: Left }](x: T): Int = x.get.value", 0, 71))
      check("a fixture document: a refinement in a parameter's bound narrows the selected member's result", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def valConcrete(x: ValWide & ValNarrow): Int = x.get.value", 0, 50))
      check("a fixture document: a concrete constructor accessor outscores a deferred member of an intersection", sameLocs(r, [at(casesDoc, "class ValWide(val get: Base)", 0, 18, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def valConcrete(x: ValWide & ValNarrow): Int = x.get.value", 0, 54))
      check("a fixture document: a selection on that accessor's narrowed type", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def read: Int = this.get.value", 0, 23))
      check("a fixture document: a self-typed `this` selects the class's own member first, as its base classes order the owners", sameLocs(r, [at(casesDoc, "trait Wide:\n  def get", 0, 18, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def read: Int = this.get.value", 0, 27))
      check("a fixture document: a selection on the joint member of a self-typed `this`", sameLocs(r, [leftValue]), r)
      // One symbol through both components meets its results; a
      // nested intersection scores the owners against the whole receiver's base classes; a
      // join that would lose type arguments or several parents makes no record; a type member
      // two components fix differently takes the narrower where one conforms, none otherwise.
      r = await c.result("textDocument/definition", at_(casesDoc, "def same1(x: Box[Base] & Ref2): Int = x.get.value", 0, 46))
      check("a fixture document: one member through both components, the narrower result (a refinement)", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def same3(x: Box[Base] & Box[Left]): Int = x.get.value", 0, 50))
      check("a fixture document: one member through both components, the narrower result (an application)", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def generic1(x: GBase & GLeft): Int = x.get.value", 0, 47))
      check("a fixture document: one member inherited with different arguments, the narrower result", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def generic3(x: GIndirect[Base] & GIndirect[Left]): Int = x.get.value", 0, 67))
      check("a fixture document: one member through an intermediate generic base, the narrower result", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def genericJoin(flag: Boolean, a: GA, b: GB): Int = (if flag then a else b).get.value", 0, 83))
      check("a fixture document: the join of two instances of a generic class makes no record", r === null, r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def typeMeet(x: Carrier { type Item = Base } & Carrier { type Item = Left }): Int = x.get.value", 0, 93))
      check("a fixture document: a type member two components fix differently, the narrower result", sameLocs(r, [leftValue]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def fixed[A <: Base, B <: Left](x: Carrier { type Item = A } & Carrier { type Item = B }): Int = x.get.value", 0, 108))
      check("a fixture document: a type member fixed to two unrelated parameters makes no record", r === null, r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def fixedReverse[A <: Base, B <: Left](x: Carrier { type Item = B } & Carrier { type Item = A }): Int = x.get.value", 0, 115))
      check("a fixture document: the reversed one makes no record too", r === null, r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def n5(x: (Wide & OtherGet) & (SubNarrow & SubWide)): Int = x.get.value", 0, 65))
      check("a fixture document: a nested intersection scores the owners against the whole receiver's base classes", sameLocs(r, [at(casesDoc, "trait Narrow:\n  def get", 0, 20, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def n6(x: (Narrow & Wide) & (SubOther & SubNarrow)): Int = x.get.value", 0, 64))
      check("a fixture document: the other nesting, the member of the component whose classes come first", sameLocs(r, [at(casesDoc, "trait OtherGet:\n  def get", 0, 22, 3)]), r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def mix(flag: Boolean, a: Mix1, b: Mix2): Int = (if flag then a else b).get.value", 0, 79))
      check("a fixture document: a join with two dominant common parents makes no record", r === null, r)
      r = await c.result("textDocument/definition", at_(casesDoc, "def mix(flag: Boolean, a: Mix1, b: Mix2): Int = (if flag then a else b).get.value", 0, 83))
      check("a fixture document: nor does a selection on it", r === null, r)
      // One document under two dependency versions: p3 reads the same `libb` over `liba` 2.0.
      // Its own navigation reaches `liba` 2.0's document; `libb`'s document is the one p1
      // produced, and the session that produced it answers inside it, with its version of `A`.
      const twice3 = await c.result("textDocument/definition", at_(p3, "b.twiceHello", 0, 2))
      check("two dependency versions: one document of the shared jar's source", docOf(twice3?.[0]) === bDoc, twice3)
      const a2 = await c.result("textDocument/definition", at_(p3, "new liba.A", 0, 9))
      const a2Doc = docOf(a2?.[0])
      check("two dependency versions: the other project's navigation reaches its own version's document", a2?.length === 1 && underCache(a2[0], "/liba-2.0-sources.jar/liba/A.scala") && a2Doc !== aDoc && same(a2[0], at(a2Doc, "class A", 0, 6, 1)), a2)
      const added = await c.result("textDocument/definition", at_(p3, "a.added", 0, 2))
      check("two dependency versions: a member of the other version", a2Doc && sameLocs(added, [at(a2Doc, "def added", 0, 4, 5)]), added)
      r = await c.result("textDocument/definition", at_(bDoc, "extends A", 0, 8))
      check("two dependency versions: inside the shared document the producing session answers, its version", aDoc && sameLocs(r, [at(aDoc, "class A", 0, 6, 1)]), r)
    }
    // Context bounds in a source, each navigating to the
    // written type's declaration, an applied bound through its alias.
    const bounds = join(root, "p1/src/Bounds.scala")
    const show = at(bounds, "trait Show", 0, 6, 4)
    for (const [what, needle, delta] of [["a simple bound", "[T: Show]", 4], ["the second of several bounds", "[T: Show: Eq]", 4], ["a class's bound", "class Box[T: Show]", 13], ["an extension's bound", "extension [T: Show]", 14], ["a using clause's type", "(using s: Show[T])", 10], ["a given's declared type", "given made: Show[Int]", 12]]) {
      const r = await c.result("textDocument/definition", at_(bounds, needle, 0, delta))
      check(`context bounds: ${what}`, sameLocs(r, [show]), r)
    }
    let eqDef = await c.result("textDocument/definition", at_(bounds, "[T: Show: Eq]", 0, 10))
    check("context bounds: the second bound of several", sameLocs(eqDef, [at(bounds, "trait Eq", 0, 6, 2)]), eqDef)
    eqDef = await c.result("textDocument/definition", at_(bounds, "[T: Conv[Int]]", 0, 4))
    check("context bounds: an applied bound names its alias", sameLocs(eqDef, [at(bounds, "type Conv[A]", 0, 5, 4)]), eqDef)
    eqDef = await c.result("textDocument/definition", at_(bounds, "[T <: U]", 0, 6))
    check("context bounds: an upper bound", sameLocs(eqDef, [at(bounds, "trait U", 0, 6, 1)]), eqDef)
    eqDef = await c.result("textDocument/definition", at_(bounds, "[T >: L]", 0, 6))
    check("context bounds: a lower bound", sameLocs(eqDef, [at(bounds, "trait L", 0, 6, 1)]), eqDef)
    const showHover = await c.result("textDocument/hover", at_(bounds, "[T: Show]", 0, 4))
    check("context bounds: hover on a bound shows the type class", showHover?.contents?.value?.includes("trait Show[T]"), showHover)
    const showRefs = await c.result("textDocument/references", { ...at_(bounds, "[T: Show]", 0, 4), context: { includeDeclaration: true } })
    const showAll = []
    for (let i = 0, text = readFileSync(bounds, "utf-8"); (i = text.indexOf("Show", i)) >= 0; i += 4) showAll.push(at(bounds, "Show", showAll.length, 0, 4))
    check("context bounds: references of the type class are every written bound", sameLocs(showRefs, showAll), { found: showRefs, expected: showAll })
    // An upstream source edited since its products were built: their positions describe
    // another text, and the downstream answers no location there.
    writeFileSync(up, "//x\n" + readFileSync(up, "utf-8"))
    const stale = await c.request("textDocument/definition", at_(down, "twice(21"))
    check("an upstream source changed since its build: no location", stale.result === null && !stale.error, stale)
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the libraries scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

/** A library document asked after the session that produced it was parked (idle past the
 * configured seconds): the session is woken for it, no app query in between. */
async function libraryDocumentParked() {
  const root = join(work, "link/park")
  cpSync(join(here, "libs"), root, { recursive: true })
  const cache = join(work, "cache")
  mkdirSync(cache, { recursive: true })
  writeExport(join(root, "teq.lock"), { p1: { sources: ["p1/src"], classpath: [{ file: "lib/liba-1.0.jar" }, { file: "lib/libb-1.0.jar" }] } })
  const p1 = join(root, "p1/src/P1.scala")
  writeFileSync(p1, "object P1:\n  val b = new libb.B\n  def n: Int = b.twiceHello\n")
  const c = new Client(root, { TEQ_CACHE_DIR: cache })
  const children = () => execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { general: { positionEncodings: ["utf-16"] } }, initializationOptions: { maxSessions: 8, sessionIdleSeconds: 2 } })
    c.notify("initialized", {})
    const twice = await c.result("textDocument/definition", at_(p1, "b.twiceHello", 0, 2))
    const bDoc = twice?.[0]?.uri && fileURLToPath(twice[0].uri)
    check("parked: definition into the jar's document", !!bDoc?.endsWith("/libb-1.0-sources.jar/libb/B.scala"), twice)
    const parked = await until(() => children() === 0, 10000)
    check("parked: the session stopped once idle", parked, { children: children() })
    if (bDoc) {
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(bDoc), languageId: "scala", version: 1, text: readFileSync(bDoc, "utf-8") } })
      const r = await c.result("textDocument/definition", at_(bDoc, "twiceHello + helper"))
      check("parked: a query on the document wakes the session that produced it and answers", sameLocs(r, [at(bDoc, "def twiceHello", 0, 4, 10)]) && children() === 1, { r, children: children() })
    }
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the parked library document scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

/** The session that produced a library document is replaced once it is retired: the first
 * mapping session the fallback chose answers from then on, woken when parked, so that the
 * document keeps answering with that session's dependency versions. Three projects under a cap
 * of one, p1 and p3 over `liba` 1.0, p2 over `liba` 2.0, all reading `libb`. */
async function libraryDocumentProducerReplaced() {
  const root = join(work, "link/producer")
  cpSync(join(here, "libs"), root, { recursive: true })
  const cache = join(work, "cache")
  mkdirSync(cache, { recursive: true })
  const over = (liba) => ({ sources: [], classpath: [{ file: `lib/liba-${liba}.jar` }, { file: "lib/libb-1.0.jar" }] })
  const projects = { p1: { ...over("1.0"), sources: ["p1/src"] }, p2: { ...over("2.0"), sources: ["p2/src"] }, p3: { ...over("1.0"), sources: ["p3/src"] } }
  const exportFile = join(root, "teq.lock")
  writeExport(exportFile, projects)
  const apps = {}
  for (const p of ["p1", "p2", "p3"]) {
    mkdirSync(join(root, p, "src"), { recursive: true })
    apps[p] = join(root, p, "src/App.scala")
    writeFileSync(apps[p], "object App:\n  val b = new libb.B\n  def n: Int = b.twiceHello\n")
  }
  const c = new Client(root, { TEQ_CACHE_DIR: cache })
  const children = () => execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
  const open = (file) => c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text: readFileSync(file, "utf-8") } })
  const close = (file) => c.notify("textDocument/didClose", { textDocument: { uri: uriOf(file) } })
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { general: { positionEncodings: ["utf-16"] }, workspace: { didChangeWatchedFiles: { dynamicRegistration: true } } }, initializationOptions: { maxSessions: 1 } })
    c.notify("initialized", {})
    open(apps.p1)
    const twice = await c.result("textDocument/definition", at_(apps.p1, "b.twiceHello", 0, 2))
    const bDoc = twice?.[0]?.uri && fileURLToPath(twice[0].uri)
    check("producer replaced: p1 produces the shared document", !!bDoc?.endsWith("/libb-1.0-sources.jar/libb/B.scala"), twice)
    if (bDoc) {
      open(bDoc)
      const a = (version, line) => (loc) => !!loc && loc.uri.endsWith(`/liba-${version}-sources.jar/liba/A.scala`) && same(loc.range, { start: { line, character: 6 }, end: { line, character: 7 } })
      let r = await c.result("textDocument/definition", at_(bDoc, "extends A", 0, 8))
      check("producer replaced: p1 answers inside it with liba 1.0", r?.length === 1 && a("1.0", 1)(r[0]), r)
      close(apps.p1)
      open(apps.p2)
      await c.result("textDocument/hover", at_(apps.p2, "libb.B", 0, 5))
      r = await c.result("textDocument/definition", at_(bDoc, "extends A", 0, 8))
      check("producer replaced: the parked producer is woken under the cap and answers with liba 1.0", r?.length === 1 && a("1.0", 1)(r[0]) && children() === 1, { r, children: children() })
      await c.result("textDocument/hover", at_(apps.p2, "libb.B", 0, 5))
      // p1 leaves the export: its session retires, and the next query falls back to p2.
      delete projects.p1
      writeExport(exportFile, projects)
      c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(exportFile), type: 2 }] })
      await sleep(500)
      r = await c.result("textDocument/definition", at_(bDoc, "extends A", 0, 8))
      check("producer replaced: with the producer retired the first mapping session answers, liba 2.0", r?.length === 1 && a("2.0", 2)(r[0]), r)
      close(apps.p2)
      open(apps.p3)
      await c.result("textDocument/hover", at_(apps.p3, "libb.B", 0, 5))
      r = await c.result("textDocument/definition", at_(bDoc, "extends A", 0, 8))
      check("producer replaced: the replacement is woken for the document and keeps its version", r?.length === 1 && a("2.0", 2)(r[0]) && children() === 1, { r, children: children() })
    }
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the replaced producer scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- The lean std's documents ---------------------------------------------------------------

/** A JavaScript project on the lean std, without class path: a std symbol is located in its std file's document, written read-only under the
 * cache at `attached/std-<identity>/<the file's path under std/>`, the identity a hash of the
 * binary's std sources, its text the embedded file's. */
async function leanStd() {
  const root = join(work, "link/lean")
  const cache = join(work, "cache")
  mkdirSync(join(root, "lean/src"), { recursive: true })
  mkdirSync(cache, { recursive: true })
  const app = join(root, "lean/src/L.scala")
  writeFileSync(
    app,
    `object L:
  val xs = List(1, 2)
  val ys = xs.map(_ + 1)
  val pair = 1 -> "one"
  val ref = new java.lang.ref.WeakReference(xs)
  val boxed = java.lang.Integer.valueOf(3)
  val tried: scala.util.Try[Int] = scala.util.Success(1)
  val stripped = "hi".stripPrefix("h")
  def show(): Unit = println(ys.mkString(","))
  case class Neg(n: Int):
    def unary_- : Neg = Neg(n)
  val negated = -Neg(1)
  class Plus:
    def unary_+ : Int = 1
  val twice = +(+new Plus)
  class Base:
    def unary_- : Int = 1
  object Obj extends Base:
    override def unary_- : Int = 2
  val ascribed = -(Obj: Base)
`,
  )
  // A class no std file imports, which a completion in a std document must not offer with one.
  mkdirSync(join(root, "lean/src/foreign"), { recursive: true })
  writeFileSync(join(root, "lean/src/foreign/Nimble.scala"), "package foreign\n\nclass Nimble\n")
  // Under the unused imports' warnings made errors: the std's own imports, which its documents
  // record, are never reported (`program_source` keeps its meaning).
  writeExport(join(root, "teq.lock"), { lean: { platform: "js", sources: ["lean/src"], flags: { wunusedImports: true, werror: true } } })
  const std = join(here, "../../std")
  const c = new Client(root, { TEQ_CACHE_DIR: cache })
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { general: { positionEncodings: ["utf-16"] } } })
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(app), languageId: "scala", version: 1, text: readFileSync(app, "utf-8") } })
    const attached = realpathSync(cache) + "/attached/"
    const docOf = (loc) => loc?.uri && fileURLToPath(loc.uri)
    // The std file a location names, its path under std/, where it is one of the std's documents.
    const stdFile = (loc) => {
      const m = docOf(loc)?.startsWith(attached) && /^std-[0-9a-f]{16}\/(.*)$/.exec(docOf(loc).slice(attached.length))
      return m ? m[1] : undefined
    }
    const into = async (label, needle, file, decl, n = 0, delta = 0, length) => {
      const r = await c.result("textDocument/definition", at_(app, needle))
      const d = docOf(r?.[0])
      check(`lean std: definition of ${label} from the app, into ${file}`, r?.length === 1 && stdFile(r[0]) === file && existsSync(d) && same(r[0], at(d, decl, n, delta, length)), r)
      return d
    }
    const collections = await into("List", "List(1", "collections.scala", "object List:", 0, 7, 4)
    // Answered after the first build: a clean file has nothing published, or an empty list.
    check("lean std: the app checks clean under --wunused imports --werror", same(c.latestDiagnostics(uriOf(app)), []), c.diagnosticsSince(0))
    await into("a method of List", "map(_", "collections.scala", "def map[B](f: A => B): List[B] =", 0, 4, 3)
    const prelude = await into("an extension of the prelude", "-> \"one\"", "prelude.scala", "def ->[B]", 0, 4, 2)
    await into("a prelude method", "println(", "prelude.scala", "def println(x: Any", 0, 4, 7)
    const lang = await into("a class of a later package block", "WeakReference(", "javalib/lang.scala", "class WeakReference[T]", 0, 6, 13)
    await into("an object of the file's first package block", "Integer.valueOf", "javalib/lang.scala", "object Integer:", 0, 7, 7)
    for (const [d, file] of [[collections, "collections.scala"], [prelude, "prelude.scala"], [lang, "javalib/lang.scala"]]) {
      const written = d && existsSync(d) && statSync(d)
      check(`lean std: ${file}'s document is the embedded text, read-only`, !!written && readFileSync(d, "utf-8") === readFileSync(join(std, file), "utf-8") && (written.mode & 0o222) === 0, d)
    }
    const hover = await c.result("textDocument/hover", at_(app, "List(1"))
    check("lean std: hover on a std object from the app", hover?.contents?.value === "```scala\nobject scala.List\n```", hover)
    if (!collections || !prelude || !lang) throw new Error("no std documents to go into")
    // A cached document truncated or altered since it was written: made the embedded text again,
    // read-only, before a location is answered in it, and the location declined where it cannot be.
    const whole = (d, file) => readFileSync(d, "utf-8") === readFileSync(join(std, file), "utf-8") && (statSync(d).mode & 0o222) === 0
    chmodSync(collections, 0o644)
    writeFileSync(collections, "package scala\n")
    const afterTruncation = await c.result("textDocument/definition", at_(app, "List(1"))
    check("lean std: a truncated std document is made whole before a location into it", whole(collections, "collections.scala") && sameLocs(afterTruncation, [at(collections, "object List:", 0, 7, 4)]), afterTruncation)
    chmodSync(prelude, 0o644)
    writeFileSync(prelude, readFileSync(prelude, "utf-8").replace("def println(", "def printlx("))
    const afterAlteration = await c.result("textDocument/definition", at_(app, "println("))
    check("lean std: a std document altered at its length is made whole before a location into it", whole(prelude, "prelude.scala") && sameLocs(afterAlteration, [at(prelude, "def println(x: Any", 0, 4, 7)]), afterAlteration)
    if (process.getuid?.() !== 0) {
      chmodSync(lang, 0o644)
      writeFileSync(lang, "package java.lang\n")
      chmodSync(dirname(lang), 0o555)
      const declined = await c.result("textDocument/definition", at_(app, "WeakReference("))
      chmodSync(dirname(lang), 0o755)
      const answered = await c.result("textDocument/definition", at_(app, "WeakReference("))
      check("lean std: a std document that cannot be made whole answers no location, and does once it can", declined === null && whole(lang, "javalib/lang.scala") && sameLocs(answered, [at(lang, "class WeakReference[T]", 0, 6, 13)]), { declined, answered })
    }
    // Inside a std document: the session's own records of the std file, its method bodies typed
    // on the document's first query.
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(collections), languageId: "scala", version: 1, text: readFileSync(collections, "utf-8") } })
    const stdDoc = (file) => join(dirname(collections), file)
    const inside = async (label, needle, n, delta, file, decl, dn, ddelta, dlength) => {
      const r = await c.result("textDocument/definition", at_(collections, needle, n, delta))
      check(`lean std, inside collections.scala: ${label}`, r?.length === 1 && stdFile(r[0]) === file && same(r[0], at(stdDoc(file), decl, dn, ddelta, dlength)), r)
    }
    await inside("same file, in a body the app never calls (`Nil` in `List.empty`)", "def empty[A]: List[A] = Nil", 0, 24, "collections.scala", "case object Nil", 0, 12, 3)
    await inside("same file, in a signature (`List` in `List.empty`'s result)", "def empty[A]: List[A] = Nil", 0, 14, "collections.scala", "sealed trait List[+A]", 0, 13, 4)
    await inside("another std file (`toList` in `List.apply`'s body)", "elems.toList", 0, 6, "iterable.scala", "def toList: List[A] = fromArray", 0, 4, 6)
    await inside("a parameter (`elems` in `List.apply`'s body)", "elems.toList", 0, 0, "collections.scala", "def apply[A](elems: A*)", 0, 13, 5)
    const hovered = await c.result("textDocument/hover", at_(collections, "elems.toList", 0, 6))
    check("lean std, inside collections.scala: hover on a call", hovered?.contents?.value === "```scala\ndef scala.IterableOps.toList: List[A]\n```" && same(hovered.range, at(collections, "elems.toList", 0, 6, 6).range), hovered)
    // A std document completes as a program file: its first query (a completion here, before any
    // references query demands every std file) demands its file, whose bodies record the
    // receivers of their selections.
    const iterable = stdDoc("iterable.scala")
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(iterable), languageId: "scala", version: 1, text: readFileSync(iterable, "utf-8") } })
    const site = "def nonEmpty: Boolean = it.iterator.hasNext"
    const completed = await c.result("textDocument/completion", at_(iterable, site, 0, site.indexOf("hasNext") + 3))
    const hasNext = itemOf(completed, "hasNext")
    check(
      "lean std, inside iterable.scala: completion after a receiver in a body, the document's first query",
      hasNext?.detail === "Boolean" && same(hasNext.textEdit?.range, at(iterable, site, 0, site.indexOf("hasNext"), 3).range) && hasNext.textEdit.newText === "hasNext" && labels(completed).every((l) => l.toLowerCase().startsWith("has")),
      completed,
    )
    const scoped = await c.result("textDocument/completion", at_(collections, "def empty[A]: List[A] = Nil", 0, 25))
    const nil = itemOf(scoped, "Nil")
    check("lean std, inside collections.scala: completion of a name in scope", !!nil && same(nil.textEdit?.range, at(collections, "def empty[A]: List[A] = Nil", 0, 24, 1).range) && labels(scoped).every((l) => l.toLowerCase().startsWith("n")), scoped)
    // Two letters typed, where a program file is offered the names out of scope with their
    // imports: a std document takes no import, and is offered none (`foreign.Nimble`).
    const two = await c.result("textDocument/completion", at_(collections, "def empty[A]: List[A] = Nil", 0, 26))
    check("lean std, inside collections.scala: no name out of scope, which an import would bring", !!itemOf(two, "Nil") && !itemOf(two, "Nimble") && (two?.items ?? []).every((i) => !i.data?.import && !i.additionalTextEdits), labels(two))
    const signature = await c.result("textDocument/signatureHelp", at_(collections, "List.from(suffix)", 0, 10))
    check("lean std, inside collections.scala: signature help", signature?.signatures?.[0]?.label === "from[A](source: IterableOnce[A]): List[A]" && signature.activeParameter === 0, signature)
    // A std document saved damaged as the editor holds it (open and on disk alike, a line altered
    // at its length): the server's comparison of the two finds nothing changed, the session finds
    // its file is not the embedded text, makes it whole and declines the query; the editor holding
    // the whole text again, it answers.
    const reopen = (file, text) => {
      c.notify("textDocument/didClose", { textDocument: { uri: uriOf(file) } })
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text } })
    }
    const saveDamaged = (file, from, to) => {
      const damaged = readFileSync(file, "utf-8").replace(from, to)
      chmodSync(file, 0o644)
      writeFileSync(file, damaged)
      reopen(file, damaged)
    }
    // The whole text in the editor, and on disk where the session left it damaged: the
    // positions of what follows are the embedded source's.
    const restore = (file, name) => {
      const text = readFileSync(join(std, name), "utf-8")
      if (!whole(file, name)) {
        chmodSync(file, 0o644)
        writeFileSync(file, text)
        chmodSync(file, 0o444)
      }
      reopen(file, text)
    }
    const at_std = (file, name, needle, n, delta) => ({ ...doc(file), position: pos(join(std, name), needle, n, delta) })
    const garbage = "def nonEmpty: Boolean = it.iterator.garbage"
    saveDamaged(iterable, site, garbage)
    const onGarbage = await c.result("textDocument/completion", at_(iterable, garbage, 0, garbage.indexOf("garbage") + 3))
    const iterableWhole = whole(iterable, "iterable.scala")
    restore(iterable, "iterable.scala")
    const completedAgain = await c.result("textDocument/completion", at_std(iterable, "iterable.scala", site, 0, site.indexOf("hasNext") + 3))
    check("lean std: a std document saved damaged as it is open, a completion in it declined, the file made whole, answered once the editor holds it", labels(onGarbage).length === 0 && iterableWhole && itemOf(completedAgain, "hasNext")?.detail === "Boolean", { onGarbage: labels(onGarbage), iterableWhole, completedAgain: labels(completedAgain) })
    saveDamaged(collections, "List.from(suffix)", "List.nope(suffix)")
    const onNope = await c.request("textDocument/signatureHelp", at_(collections, "List.nope(suffix)", 0, 10))
    const collectionsWhole = whole(collections, "collections.scala")
    restore(collections, "collections.scala")
    saveDamaged(collections, "List.from(suffix)", "List.nope(suffix)")
    const hoverNope = await c.request("textDocument/hover", at_(collections, "List.nope(suffix)", 0, 5))
    const collectionsWholeAgain = whole(collections, "collections.scala")
    restore(collections, "collections.scala")
    const signedAgain = await c.result("textDocument/signatureHelp", at_std(collections, "collections.scala", "List.from(suffix)", 0, 10))
    check(
      "lean std: a std document saved damaged as it is open, signature help and hover in it declined, the file made whole, answered once the editor holds it",
      onNope.result === null && !onNope.error && hoverNope.result === null && !hoverNope.error && collectionsWhole && collectionsWholeAgain && signedAgain?.signatures?.[0]?.label === "from[A](source: IterableOnce[A]): List[A]",
      { onNope, hoverNope, collectionsWhole, collectionsWholeAgain, signedAgain },
    )
    // The same damage under a burst: the session stopped while two signature helps and a hover
    // are sent, so that all three are queued before it answers any. The first's check makes the
    // file whole; the others, which then find the file whole, are declined by the server, whose
    // editor holds the damaged text still.
    saveDamaged(collections, "List.from(suffix)", "List.nope(suffix)")
    const child = Number(execSync(`pgrep -P ${c.proc.pid}`).toString().trim().split("\n")[0])
    process.kill(child, "SIGSTOP")
    const burst = [
      c.request("textDocument/signatureHelp", at_(collections, "List.nope(suffix)", 0, 10)),
      c.request("textDocument/signatureHelp", at_(collections, "List.nope(suffix)", 0, 10)),
      c.request("textDocument/hover", at_(collections, "List.nope(suffix)", 0, 5)),
    ]
    await sleep(300)
    process.kill(child, "SIGCONT")
    const burstAnswers = await Promise.all(burst)
    const burstWhole = whole(collections, "collections.scala")
    restore(collections, "collections.scala")
    check(
      "lean std: queries queued on a std document saved damaged as it is open, each declined though the first made the file whole",
      burstAnswers.every((a) => a.result === null && !a.error) && burstWhole,
      { burstAnswers, burstWhole },
    )
    // A prefix operator names the `unary_` method it calls, in a program file and in a std
    // document alike (a builtin's `!b` has none).
    const negHover = await c.result("textDocument/hover", at_(app, "-Neg(1)"))
    const negDef = await c.result("textDocument/definition", at_(app, "-Neg(1)"))
    check("lean std: a prefix operator in the app, its hover and its declaration", negHover?.contents?.value === "```scala\ndef L.Neg.unary_-: Neg\n```" && same(negHover.range, at(app, "-Neg(1)", 0, 0, 1).range) && sameLocs(negDef, [at(app, "def unary_- : Neg", 0, 4, 7)]), { negHover, negDef })
    // `+(+new Plus)`: the inner `+` is `Plus.unary_+`, the outer a builtin's on its `Int`, whose
    // node is the operand's; `-(Obj: Base)` names the ascribed `Base.unary_-`, and definition adds
    // the object's override, which runs.
    const outer = await c.request("textDocument/hover", at_(app, "+(+new Plus)"))
    const inner = await c.result("textDocument/hover", at_(app, "+(+new Plus)", 0, 2))
    check("lean std: a builtin's prefix `+` over a member's names nothing, the member's names its method", outer.result === null && inner?.contents?.value === "```scala\ndef L.Plus.unary_+: Int\n```", { outer, inner })
    const ascHover = await c.result("textDocument/hover", at_(app, "-(Obj: Base)"))
    const ascDef = await c.result("textDocument/definition", at_(app, "-(Obj: Base)"))
    check("lean std: a prefix operator on an ascribed object, the parent's method and the override", ascHover?.contents?.value === "```scala\ndef L.Base.unary_-: Int\n```" && sameLocs(ascDef, [at(app, "def unary_- : Int = 1", 0, 4, 7), at(app, "override def unary_- : Int = 2", 0, 13, 7)]), { ascHover, ascDef })
    const listRefs = await c.result("textDocument/references", { ...at_(collections, "object List:", 0, 7), context: { includeDeclaration: true } })
    const appList = at(app, "List(1", 0, 0, 4)
    check(
      "lean std, inside collections.scala: references of a std object hold the declaration, the std's uses and the app's",
      listRefs?.some((l) => same(l, at(collections, "object List:", 0, 7, 4))) && listRefs.some((l) => same(l, appList)) && listRefs.some((l) => same(l, at(collections, "List.from(suffix)", 0, 0, 4))),
      listRefs,
    )
    const fromApp = await c.result("textDocument/references", { ...at_(app, "List(1"), context: { includeDeclaration: false } })
    check("lean std: references of a std object from the app hold its uses in every std file, as inside its document", sameLocs(fromApp, listRefs.filter((l) => !same(l, at(collections, "object List:", 0, 7, 4)))), fromApp)
    const mapRefs = await c.result("textDocument/references", { ...at_(collections, "def map[B](f: A => B): List[B] =", 0, 4), context: { includeDeclaration: false } })
    check("lean std, inside collections.scala: references of a std method hold the app's call", mapRefs?.some((l) => same(l, at(app, "map(_", 0, 0, 3))), mapRefs)
    const applyItem = await c.result("textDocument/prepareCallHierarchy", at_(collections, "def apply[A](elems: A*): List[A]", 0, 4))
    check("lean std, inside collections.scala: prepareCallHierarchy on a std method", applyItem?.length === 1 && applyItem[0].name === "apply" && same(applyItem[0].selectionRange, at(collections, "def apply[A](elems: A*): List[A]", 0, 4, 5).range) && applyItem[0].uri === uriOf(collections), applyItem)
    const outgoing = applyItem?.[0] && (await c.result("callHierarchy/outgoingCalls", { item: applyItem[0] }))
    check("lean std, inside collections.scala: outgoing calls into another std file", outgoing?.length === 1 && outgoing[0].to?.name === "toList" && stdFile(outgoing[0].to) === "iterable.scala" && same(outgoing[0].fromRanges, [at(collections, "elems.toList", 0, 6, 6).range]), outgoing)
    const mapItem = await c.result("textDocument/prepareCallHierarchy", at_(collections, "def map[B](f: A => B): List[B] =", 0, 4))
    const incoming = mapItem?.[0] && (await c.result("callHierarchy/incomingCalls", { item: mapItem[0] }))
    check("lean std, inside collections.scala: incoming calls of a std method hold the app's caller", incoming?.some((i) => i.from?.name === "ys" && i.from.uri === uriOf(app) && same(i.fromRanges, [at(app, "map(_", 0, 0, 3).range])), incoming)
    // A std file the program never entered, its document written by another session: entered
    // and typed on its first query.
    const bignum = stdDoc("bignum.scala")
    if (!existsSync(bignum)) {
      writeFileSync(bignum, readFileSync(join(std, "bignum.scala"), "utf-8"))
      chmodSync(bignum, 0o444)
    }
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(bignum), languageId: "scala", version: 1, text: readFileSync(bignum, "utf-8") } })
    const gcdType = await c.result("textDocument/definition", at_(bignum, "def gcd(that: BigInt)", 0, 15))
    check("lean std, a std file the program never entered: entered on its first query", sameLocs(gcdType, [at(bignum, "final class BigInt(", 0, 12, 6)]), gcdType)
    const gcdCall = await c.result("textDocument/definition", at_(bignum, "bigInteger.gcd(", 0, 11))
    check("lean std, a std file the program never entered: its bodies into another std file", gcdCall?.length === 1 && stdFile(gcdCall[0]) === "javalib/math.scala" && same(gcdCall[0], at(stdDoc("javalib/math.scala"), "def gcd(", 0, 4, 3)), gcdCall)
    const absAt = "def abs: BigInt = if signum < 0 then -this else this"
    const minus = await c.result("textDocument/hover", at_(bignum, absAt, 0, absAt.indexOf("-this")))
    const minusDef = await c.result("textDocument/definition", at_(bignum, absAt, 0, absAt.indexOf("-this")))
    check(
      "lean std, inside bignum.scala: a prefix operator in a body, its hover and its declaration",
      minus?.contents?.value === "```scala\ndef scala.math.BigInt.unary_-: BigInt\n```" && same(minus.range, at(bignum, absAt, 0, absAt.indexOf("-this"), 1).range) && sameLocs(minusDef, [at(bignum, "def unary_- : BigInt", 0, 4, 7)]),
      { minus, minusDef },
    )
    // A std file's import and a body that uses it, recorded and demanded: still no diagnostic.
    const tryDoc = await into("a std case class", "Success(1", "try.scala", "final case class Success", 0, 17, 7)
    if (tryDoc) {
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(tryDoc), languageId: "scala", version: 1, text: readFileSync(tryDoc, "utf-8") } })
      const inBody = await c.result("textDocument/definition", at_(tryDoc, "case NonFatal(e)", 0, 5))
      const inImport = await c.result("textDocument/definition", at_(tryDoc, "import scala.util.control.NonFatal", 0, 26))
      const control = stdDoc("control.scala")
      const nonFatalAt = existsSync(control) && at(control, "object NonFatal", 0, 7, 8)
      check("lean std, inside try.scala: an extractor in a body and the import's name, into control.scala", !!nonFatalAt && sameLocs(inBody, [nonFatalAt]) && sameLocs(inImport, [nonFatalAt]), { inBody, inImport })
    }
    // What the program never reaches of a document it reached: the parent of a class it never
    // uses, the body of an inline method it never expands.
    const lang2 = await c.result("textDocument/definition", at_(lang, "class SoftReference[T](referent: T) extends Reference[T]", 0, 44))
    check("lean std, inside javalib/lang.scala: the parent of a class the program never uses", sameLocs(lang2, [at(lang, "abstract class Reference[T]", 0, 15, 9)]), lang2)
    const vt = await c.result("textDocument/definition", at_(prelude, "= vt.value", 0, 2))
    check("lean std, inside prelude.scala: a parameter in the body of an inline method the program never expands", sameLocs(vt, [at(prelude, "using vt: ValueOf[T]", 0, 6, 2)]), vt)
    // Something other than a file at a std document's path (a read-only directory holding a
    // file): left as it is, the location declined, the path told in the log once.
    const stringsAt = stdDoc("strings.scala")
    if (existsSync(stringsAt)) {
      chmodSync(stringsAt, 0o644)
      rmSync(stringsAt)
    }
    mkdirSync(stringsAt)
    writeFileSync(join(stringsAt, "inside"), "kept\n")
    chmodSync(stringsAt, 0o555)
    const logged = c.mark()
    const refused = await c.request("textDocument/definition", at_(app, "stripPrefix("))
    const kept = statSync(stringsAt)
    const told = c.notifications.slice(logged).filter((n) => n.method === "window/logMessage" && n.params.message.includes(stringsAt))
    check(
      "lean std: a directory at a std document's path keeps its type, contents and mode, its path told, no location",
      refused.result === null && !refused.error && kept.isDirectory() && (kept.mode & 0o777) === 0o555 && same(readdirSync(stringsAt), ["inside"]) && readFileSync(join(stringsAt, "inside"), "utf-8") === "kept\n" && told.length === 1,
      { refused, mode: (kept.mode & 0o777).toString(8), entries: readdirSync(stringsAt), told },
    )
    chmodSync(stringsAt, 0o755)
    rmSync(stringsAt, { recursive: true })
    // A std file with a secondary constructor (`StringBuilder`'s), demanded: typed as its class's.
    const strings = await into("a std extension", "stripPrefix(", "strings.scala", "def stripPrefix(", 0, 4, 11)
    if (strings) {
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(strings), languageId: "scala", version: 1, text: readFileSync(strings, "utf-8") } })
      const ctor = await c.result("textDocument/hover", at_(strings, "def stripPrefix(", 0, 4))
      check("lean std, inside strings.scala: hover on a declaration", !!ctor?.contents?.value?.includes("stripPrefix"), ctor)
    }
    // An edit of the app: the std's answers stand, and no diagnostic comes of the std's records.
    const m = c.mark()
    writeFileSync(app, readFileSync(app, "utf-8") + "  val size = xs.size\n")
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(app), version: 2 }, contentChanges: [{ text: readFileSync(app, "utf-8") }] })
    await c.waitDiagnostics(m, uriOf(app), () => true, 10000)
    const again = await c.result("textDocument/definition", at_(collections, "elems.toList", 0, 6))
    check("lean std: after the std documents' demands, an edit of the app checks clean", c.diagnosticsSince(0).every((p) => p.diagnostics.length === 0), c.diagnosticsSince(0))
    check("lean std, inside collections.scala: the answer after an edit of the app", again?.length === 1 && stdFile(again[0]) === "iterable.scala", again)
    const outline = await c.result("textDocument/documentSymbol", doc(collections))
    check("lean std: the outline of a std document", outline?.some((s) => s.name === "List" && same(s.selectionRange, at(collections, "object List:", 0, 7, 4).range)), outline?.length)
    // The document edited in the editor: its positions are no longer those the session typed.
    const edited = "// edited\n" + readFileSync(collections, "utf-8")
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(collections), version: 2 }, contentChanges: [{ text: edited }] })
    const onEdited = await c.result("textDocument/definition", { textDocument: { uri: uriOf(collections) }, position: pos(collections, "elems.toList", 0, 6) })
    const editedOutline = await c.result("textDocument/documentSymbol", doc(collections))
    const editedCompletion = await c.result("textDocument/completion", { textDocument: { uri: uriOf(collections) }, position: pos(collections, "elems.toList", 0, 8) })
    check("lean std: a std document edited in the editor answers nothing, its outline and completion included", onEdited === null && editedOutline === null && labels(editedCompletion).length === 0, { onEdited, editedOutline, editedCompletion })
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the lean std scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

/** Two lean projects reaching one std document, each shadowing the std's `scala.printlnImpl`
 * with a definition of its own, so that `println`'s body names a different target in each: the
 * document answers from its producer, the first session whose answer named it while that session
 * lives (woken when parked), as a jar's document does; the references of a std symbol asked in one
 * project hold the other's uses. */
async function leanStdSessions() {
  const root = join(work, "link/lean2")
  const cache = join(work, "cache")
  mkdirSync(cache, { recursive: true })
  const files = {}
  for (const p of ["a", "b"]) {
    mkdirSync(join(root, p, "src"), { recursive: true })
    files[p] = join(root, p, "src/App.scala")
    files[p + "Impl"] = join(root, p, "src/Impl.scala")
    writeFileSync(files[p], `object App:\n  def run(): Unit = println("${p}")\n`)
    // The two definitions at different places, so that their locations differ.
    writeFileSync(files[p + "Impl"], `package scala\n${p === "b" ? "\n// b's own\n" : ""}\ndef printlnImpl(x: Any): Unit = ()\n`)
  }
  writeExport(join(root, "teq.lock"), { a: { platform: "js", sources: ["a/src"] }, b: { platform: "js", sources: ["b/src"] } })
  const implOf = (p) => at(files[p + "Impl"], "def printlnImpl", 0, 4, 11)
  const run = async (first, second, label) => {
    const c = new Client(root, { TEQ_CACHE_DIR: cache })
    const children = () => execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
    try {
      await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { general: { positionEncodings: ["utf-16"] } }, initializationOptions: { maxSessions: 8, sessionIdleSeconds: 2 } })
      c.notify("initialized", {})
      const d1 = await c.result("textDocument/definition", at_(files[first], "println("))
      const d2 = await c.result("textDocument/definition", at_(files[second], "println("))
      const prelude = d1?.[0]?.uri && fileURLToPath(d1[0].uri)
      check(`${label}: both projects reach one prelude document`, !!prelude?.endsWith("/prelude.scala") && same(d1, d2) && same(d1[0], at(prelude, "def println(x: Any", 0, 4, 7)), { d1, d2 })
      if (!prelude) return
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(prelude), languageId: "scala", version: 1, text: readFileSync(prelude, "utf-8") } })
      const inside = at_(prelude, "Unit = printlnImpl(x)", 0, 7)
      const impl = await c.result("textDocument/definition", inside)
      check(`${label}: inside the document, the producer's target (${first}'s printlnImpl)`, sameLocs(impl, [implOf(first)]), impl)
      const refs = await c.result("textDocument/references", { ...at_(files[second], "println("), context: { includeDeclaration: false } })
      check(`${label}: references of a std symbol from one project hold the other's use`, refs?.some((l) => same(l, at(files.a, "println(", 0, 0, 7))) && refs.some((l) => same(l, at(files.b, "println(", 0, 0, 7))), refs)
      const parked = await until(() => children() === 0, 15000)
      check(`${label}: the sessions stopped once idle`, parked, { children: children() })
      const woken = await c.result("textDocument/definition", inside)
      check(`${label}: a query on the document wakes its producer, which answers`, sameLocs(woken, [implOf(first)]) && children() === 1, { woken, children: children() })
      await c.result("shutdown", null)
      c.notify("exit", null)
      await Promise.race([c.exited, sleep(10000)])
    } catch (e) {
      check(`${label}: ran to its end`, false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
      c.proc.kill()
    }
  }
  await run("a", "b", "two lean sessions, a first")
  await run("b", "a", "two lean sessions, b first")
}

/** The references, implementations and callers of a std definition asked from the app as the
 * session's first query into the std (docs/TARGETS.md, "Navigation"): complete, every std file
 * demanded first, those the program never entered (`duration.scala` and `bignum.scala`, whose
 * names it never uses) and those it entered (`ordering.scala`) alike, their exact spans. */
async function leanStdComplete() {
  const root = join(work, "link/lean3")
  // A cache of its own: the documents its last checks put a pipe and a link in place of are
  // written by no query before them.
  const cache = join(work, "cache-complete")
  mkdirSync(join(root, "app/src"), { recursive: true })
  mkdirSync(cache, { recursive: true })
  const app = join(root, "app/src/V.scala")
  writeFileSync(
    app,
    `object V:
  case class Version(n: Int) extends Ordered[Version]:
    def compare(that: Version): Int = n - that.n
  def stamp(): Long = System.nanoTime()
  def show(): Unit = println("v1".stripPrefix("v"))
`,
  )
  writeExport(join(root, "teq.lock"), { app: { platform: "js", sources: ["app/src"] } })
  const c = new Client(root, { TEQ_CACHE_DIR: cache })
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { general: { positionEncodings: ["utf-16"] } } })
    c.notify("initialized", {})
    const refs = await c.result("textDocument/references", { ...at_(app, "Ordered[Version]"), context: { includeDeclaration: false } })
    const ordering = (refs ?? []).map((l) => fileURLToPath(l.uri)).find((d) => d.endsWith("/ordering.scala"))
    if (!ordering) throw new Error(`no std document among ${JSON.stringify(refs)}`)
    const std = (file) => join(dirname(ordering), file)
    const uses = [
      at(app, "Ordered[Version]", 0, 0, 7),
      at(std("ordering.scala"), "def ordered[T <: Ordered[T]]", 0, 17, 7),
      at(std("math.scala"), "Ordered, Ordering}", 0, 0, 7),
      at(std("duration.scala"), "extends Ordered[Deadline]", 0, 8, 7),
      at(std("bignum.scala"), "Ordered[BigDecimal]", 0, 0, 7),
      at(std("bignum.scala"), "Ordered[Value]", 0, 0, 7),
      at(std("bignum.scala"), "Ordered[BigInt]", 0, 0, 7),
    ]
    check("lean std, the first query: references of a std trait from the app, its uses in every std file, those never entered included", sameLocs(refs, uses), { found: refs, expected: uses })
    const withDecl = await c.result("textDocument/references", { ...at_(app, "Ordered[Version]"), context: { includeDeclaration: true } })
    check("lean std: the same references with the declaration", sameLocs(withDecl, [...uses, at(std("ordering.scala"), "trait Ordered[T]", 0, 6, 7)]), withDecl)
    const impls = await c.result("textDocument/implementation", at_(app, "Ordered[Version]"))
    const implementers = [
      at(app, "case class Version", 0, 11, 7),
      at(std("duration.scala"), "final case class Deadline", 0, 17, 8),
      at(std("bignum.scala"), "final class BigDecimal(", 0, 12, 10),
      at(std("bignum.scala"), "final class Value(", 0, 12, 5),
      at(std("bignum.scala"), "final class BigInt(", 0, 12, 6),
    ]
    check("lean std: implementations of a std trait from the app, in every std file", sameLocs(impls, implementers), { found: impls, expected: implementers })
    const item = await c.result("textDocument/prepareCallHierarchy", at_(app, "nanoTime()"))
    const callers = item?.[0] && (await c.result("callHierarchy/incomingCalls", { item: item[0] }))
    const named = (callers ?? []).map((i) => [i.from?.name, fileURLToPath(i.from?.uri ?? "file:///"), i.fromRanges]).sort((a, b) => a[0].localeCompare(b[0]))
    const expected = [
      ["isOverdue", std("duration.scala"), [at(std("duration.scala"), "System.nanoTime() < 0", 0, 7, 8).range]],
      ["now", std("duration.scala"), [at(std("duration.scala"), "FiniteDuration(System.nanoTime(), NANOSECONDS)", 0, 22, 8).range]],
      ["stamp", app, [at(app, "nanoTime()", 0, 0, 8).range]],
    ]
    check("lean std: incoming calls of a std method from the app, the bodies of a std file the program never entered included", same(named, expected), { found: named, expected })
    // A pipe and a link at std documents' paths: never read nor replaced, no location, each
    // path told once.
    const pipe = std("strings.scala")
    const link = std("prelude.scala")
    const decoy = join(root, "decoy.txt")
    writeFileSync(decoy, "decoy\n")
    if (existsSync(pipe) || existsSync(link)) throw new Error("the documents are written already")
    execSync(`mkfifo "${pipe}"`)
    symlinkSync(decoy, link)
    const logged = c.mark()
    const onPipe = await c.request("textDocument/definition", at_(app, "stripPrefix("))
    const onLink = await c.request("textDocument/definition", at_(app, "println("))
    const told = (p) => c.notifications.slice(logged).filter((n) => n.method === "window/logMessage" && n.params.message.includes(p)).length
    check(
      "lean std: a pipe and a link at std documents' paths are left as they are, no location, their paths told",
      onPipe.result === null && onLink.result === null && statSync(pipe).isFIFO() && lstatSync(link).isSymbolicLink() && readFileSync(decoy, "utf-8") === "decoy\n" && told(pipe) === 1 && told(link) === 1,
      { onPipe, onLink, toldPipe: told(pipe), toldLink: told(link) },
    )
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the complete std references scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

/** A std document asked a completion with no producer known (a server started after another
 * wrote the document), where a JVM session, whose std is scala-library's jar and maps no std
 * document, runs before a lean one: the lean session answers. */
async function stdDocumentRouting() {
  const root = join(work, "link/lean4")
  const cache = join(work, "cache")
  for (const p of ["j", "l"]) mkdirSync(join(root, p, "src"), { recursive: true })
  const jvmFile = join(root, "j/src/J.scala")
  const leanFile = join(root, "l/src/L.scala")
  writeFileSync(jvmFile, "object J:\n  val xs = List(1)\n")
  writeFileSync(leanFile, "object L:\n  val xs = List(1)\n")
  writeExport(join(root, "teq.lock"), { j: { platform: "jvm", sources: ["j/src"] }, l: { platform: "js", sources: ["l/src"] } })
  const init = async (c) => {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { general: { positionEncodings: ["utf-16"] } } })
    c.notify("initialized", {})
  }
  let c = new Client(root, { TEQ_CACHE_DIR: cache })
  let collections
  try {
    await init(c)
    const d = await c.result("textDocument/definition", at_(leanFile, "List(1"))
    collections = d?.[0]?.uri && fileURLToPath(d[0].uri)
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("std document routing: the first server ran", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
  if (!collections?.endsWith("/collections.scala")) return check("std document routing: a document written by the first server", false, collections)
  c = new Client(root, { TEQ_CACHE_DIR: cache })
  try {
    await init(c)
    for (const f of [jvmFile, leanFile]) {
      c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(f), languageId: "scala", version: 1, text: readFileSync(f, "utf-8") } })
      await c.result("textDocument/documentSymbol", doc(f))
    }
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(collections), languageId: "scala", version: 1, text: readFileSync(collections, "utf-8") } })
    const completed = await c.result("textDocument/completion", at_(collections, "def empty[A]: List[A] = Nil", 0, 25))
    check("std document routing: a completion in a std document no navigation reached, a JVM session running first, answered by the lean one", !!itemOf(completed, "Nil"), labels(completed))
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("std document routing ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- The export's generators and a root created during the session ---------------------------

async function serverGenerators() {
  const root = join(work, "link/gens")
  mkdirSync(join(root, "g/src"), { recursive: true })
  writeFileSync(join(root, "g/value.txt"), "1\n")
  writeFileSync(join(root, "gen.sh"), 'mkdir -p g/gen && printf "object Gen:\\n  val v = %s\\n" "$(cat g/value.txt)" > g/gen/Gen.new && if cmp -s g/gen/Gen.new g/gen/Gen.scala; then rm g/gen/Gen.new; else mv g/gen/Gen.new g/gen/Gen.scala; fi\n')
  const use = join(root, "g/src/Use.scala")
  writeFileSync(use, "object Use:\n  val x: Int = Gen.v\n")
  const generator = { kind: "command", run: ["sh", "gen.sh"], cwd: ".", inputs: ["g/value.txt"], outputs: ["g/gen"] }
  writeExport(join(root, "teq.lock"), { g: { platform: "js", sources: ["g/src", "g/gen", "g/more"], generators: [generator] } })
  const c = new Client(root)
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: { workspace: { didChangeWatchedFiles: { dynamicRegistration: true } } } })
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(use), languageId: "scala", version: 1, text: readFileSync(use, "utf-8") } })
    const h = await c.result("textDocument/hover", at_(use, "Gen.v", 0, 4))
    check("a session's generators run before it starts: the generated source typed", h?.contents?.value?.includes("val Gen.v: Int") && existsSync(join(root, "g/gen/Gen.scala")), h)
    const m = c.mark()
    writeFileSync(join(root, "g/value.txt"), '"s"\n')
    const d = await c.waitDiagnostics(m, uriOf(use), (ds) => ds.length === 1, 15000)
    check("a generator's input changed during the session: generated again, its use checked against it", d?.[0]?.message.includes("type mismatch"), d)
    const more = join(root, "g/more/M.scala")
    mkdirSync(dirname(more), { recursive: true })
    writeFileSync(more, 'object M:\n  val wrong: Int = "m"\n')
    const m2 = c.mark()
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(more), type: 1 }] })
    const dm = await c.waitDiagnostics(m2, uriOf(more), (ds) => ds.length === 1, 15000)
    check("a declared root created during the session: the session started anew over it", dm?.[0]?.message.includes("type mismatch"), dm)
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
  } catch (e) {
    check("the generators scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
  // A generator sbt alone runs (kind sbt), which the export records with sbt's managed directory
  // among the sources, under the root's target/teq/ (a build without the native driver): nothing
  // is run, what sbt last generated there is typed, and the project answers from it at the
  // session's start and after a later full build (a save); so does a project depending on it,
  // whose description selects that directory, and the project's Test configuration, whose own
  // generator sbt runs into its managed directory of Test.
  const recorded = join(work, "link/recorded")
  const managed = "target/out/jvm/scala-3.8.4/r/src_managed/main"
  const managedTest = "target/out/jvm/scala-3.8.4/r/src_managed/test"
  for (const d of ["r/src", "r/test", "c/src", managed, managedTest]) mkdirSync(join(recorded, d), { recursive: true })
  const made = join(recorded, managed, "Made.scala")
  writeFileSync(made, "object Made:\n  val v: Int = 1\n")
  writeFileSync(join(recorded, managedTest, "MadeTest.scala"), "object MadeTest:\n  val t: Int = 2\n")
  const user = join(recorded, "r/src/User.scala")
  const userText = "object User:\n  val x: Int = Made.v\n"
  writeFileSync(user, userText)
  const tester = join(recorded, "r/test/Tester.scala")
  writeFileSync(tester, "object Tester:\n  val z: Int = MadeTest.t + Made.v\n")
  const consumer = join(recorded, "c/src/Consumer.scala")
  writeFileSync(consumer, "object Consumer:\n  val y: Int = Made.v\n")
  const bySbt = [{ kind: "sbt", task: "an unnamed task" }]
  const recordedLock = {
    teq: "0.1.2", format: 1, binaries: {}, inputs: { files: {} }, repositories: [], jars: {},
    projects: {
      r: {
        base: "r", platform: "js", description: { sources: ["r/src", managed] },
        configurations: {
          compile: { sources: ["r/src", managed], classpath: [], flags: {}, generators: bySbt },
          test: { sources: ["r/test", managedTest], classpath: [{ project: "r", configuration: "compile" }], flags: {}, generators: bySbt },
        },
      },
      c: {
        base: "c", platform: "js", description: { sources: ["r/src", managed, "c/src"] },
        configurations: { compile: { sources: ["c/src"], classpath: [{ project: "r", configuration: "compile" }], flags: {}, generators: [] } },
      },
    },
  }
  mkdirSync(join(recorded, "target/teq"), { recursive: true })
  writeFileSync(join(recorded, "target/teq/teq.lock"), lockText(recordedLock))
  const c2 = new Client(recorded)
  try {
    await c2.result("initialize", { processId: null, rootUri: uriOf(recorded), capabilities: {} })
    c2.notify("initialized", {})
    const m = c2.mark()
    c2.notify("textDocument/didOpen", { textDocument: { uri: uriOf(user), languageId: "scala", version: 1, text: userText } })
    const h = await c2.result("textDocument/hover", at_(user, "Made.v", 0, 5))
    const d = await c2.result("textDocument/definition", at_(user, "Made.v", 0, 5))
    const failed = () => c2.diagnosticsSince(m).filter((p) => p.diagnostics.some((x) => x.message.includes("generator")))
    check(
      "a generator sbt alone runs: its project answers hover and definition from sbt's managed directory, under target/teq/",
      h?.contents?.value?.includes("val Made.v: Int") && sameLocs(d, [at(made, "v: Int", 0, 0, 1)]) && c2.diagnosticsSince(m).every((p) => p.diagnostics.length === 0),
      { h, d, published: c2.diagnosticsSince(m) },
    )
    // A later full build, after a save: the generator is skipped again, the build's own
    // diagnostic published and then withdrawn.
    const m2 = c2.mark()
    c2.notify("textDocument/didChange", { textDocument: { uri: uriOf(user), version: 2 }, contentChanges: [{ text: 'object User:\n  val x: Int = Made.v\n  val w: Int = "w"\n' }] })
    c2.notify("textDocument/didSave", { textDocument: { uri: uriOf(user) } })
    const wrong = await c2.waitDiagnostics(m2, uriOf(user), (ds) => ds.length === 1, 30000)
    const m3 = c2.mark()
    c2.notify("textDocument/didChange", { textDocument: { uri: uriOf(user), version: 3 }, contentChanges: [{ text: userText }] })
    c2.notify("textDocument/didSave", { textDocument: { uri: uriOf(user) } })
    const right = await c2.waitDiagnostics(m3, uriOf(user), (ds) => ds.length === 0, 30000)
    const h2 = await c2.result("textDocument/hover", at_(user, "Made.v", 0, 5))
    check(
      "a generator sbt alone runs: a later full build skips it too, its own diagnostic published and withdrawn",
      wrong?.[0]?.message.includes("type mismatch") && right?.length === 0 && h2?.contents?.value?.includes("val Made.v: Int") && failed().length === 0,
      { wrong, right, h2, failed: failed() },
    )
    c2.notify("textDocument/didOpen", { textDocument: { uri: uriOf(consumer), languageId: "scala", version: 1, text: readFileSync(consumer, "utf-8") } })
    const hc = await c2.result("textDocument/hover", at_(consumer, "Made.v", 0, 5))
    const dc = await c2.result("textDocument/definition", at_(consumer, "Made.v", 0, 5))
    check("a generator sbt alone runs: a project depending on it answers from its managed directory, which its description selects", hc?.contents?.value?.includes("val Made.v: Int") && sameLocs(dc, [at(made, "v: Int", 0, 0, 1)]), { hc, dc })
    c2.notify("textDocument/didOpen", { textDocument: { uri: uriOf(tester), languageId: "scala", version: 1, text: readFileSync(tester, "utf-8") } })
    const ht = await c2.result("textDocument/hover", at_(tester, "MadeTest.t", 0, 9))
    check("a Test generator sbt alone runs: the test configuration answers from its managed directory of Test", ht?.contents?.value?.includes("val MadeTest.t: Int") && failed().length === 0, { ht, failed: failed() })
    await c2.result("shutdown", null)
    c2.notify("exit", null)
    await Promise.race([c2.exited, sleep(5000)])
  } catch (e) {
    check("the recorded generator scenario ran to its end", false, `${e.stack}\nstderr: ${c2.stderr.slice(-1000)}`)
    c2.proc.kill()
  }
}

// --- An sbt build the server exports by itself ------------------------------------------------

// The driver's sbt (tests/lsp/fake-sbt), first on the PATH of the servers of these scenarios, told
// the plugin the server selects, the checkout's own line that build.rs bakes in.
const sbtBin = join(work, "bin")
mkdirSync(sbtBin, { recursive: true })
copyFileSync(join(here, "fake-sbt"), join(sbtBin, "sbt"))
chmodSync(join(sbtBin, "sbt"), 0o755)
const sbtPlugin = readFileSync(join(here, "../../integrations/sbt/plugin-version.txt"), "utf-8").trim()
const sbtEnv = { PATH: `${sbtBin}:${process.env.PATH}`, FAKE_SBT_PLUGIN: sbtPlugin }
const progressCapabilities = { general: { positionEncodings: ["utf-16"] }, workspace: { didChangeWatchedFiles: { dynamicRegistration: true } }, window: { workDoneProgress: true } }
const progressOf = (c, kind) => c.notifications.filter((n) => n.method === "$/progress" && n.params.value.kind === kind).map((n) => n.params)
const exportTokens = (c) => progressOf(c, "begin").filter((p) => p.value.message.startsWith("exporting the sbt build")).map((p) => p.token)
const checkingTokens = (c) => progressOf(c, "begin").filter((p) => p.value.message.startsWith("checking")).map((p) => p.token)
const endedAll = (c, tokens) => tokens.every((t) => progressOf(c, "end").some((p) => p.token === t))
/** Whether `workspace/symbol` finds `name` within `timeout`. */
async function symbolAppears(c, name, timeout) {
  const deadline = Date.now() + timeout
  for (;;) {
    const found = await c.result("workspace/symbol", { query: name })
    if (found?.some((s) => s.name === name)) return true
    if (Date.now() > deadline) return false
    await sleep(100)
  }
}
/** Copies the sbt fixture to `name` under the work directory, with the mode the fake sbt reads. */
function sbtCopy(name, mode) {
  const root = join(work, "link", name)
  cpSync(join(here, "sbt"), root, { recursive: true })
  if (mode) writeFileSync(join(root, "fake-sbt-mode"), mode)
  return root
}
async function changeBuild(c, root) {
  appendFileSync(join(root, "build.sbt"), "// changed\n")
  c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(join(root, "build.sbt")), type: 2 }] })
}

/** The automatic export, its lock at the build's root (`teqBuildTool`) or, as without the native
 * driver, under its target/teq/ (the fake sbt's `fake-sbt-target`): the sessions as from the
 * root, the stale warning, the run once. */
async function sbtBuild() {
  await sbtBuildAt("teq.lock")
  await sbtBuildAt("target/teq/teq.lock")
}

async function sbtBuildAt(place) {
  const under = place !== "teq.lock"
  const where = under ? "under target/teq/: " : ""
  const root = sbtCopy(under ? "sbt-target" : "sbt")
  if (under) writeFileSync(join(root, "fake-sbt-target"), "")
  const demo = join(root, "src/Demo.scala")
  const buildFile = join(root, "build.sbt")
  const exportFile = join(root, place)
  const runs = () => (existsSync(join(root, "fake-sbt-runs")) ? readFileSync(join(root, "fake-sbt-runs"), "utf-8").trim().split("\n").length : 0)
  const c = new Client(root, sbtEnv)
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: progressCapabilities })
    const m = c.mark()
    c.notify("initialized", {})
    const begun = await until(() => exportTokens(c).length === 1, 5000)
    const watched = c.requests.filter((r) => r.method === "client/registerCapability").map((r) => JSON.stringify(r.params))
    check(where + "the build files are watched", watched.length === 1 && watched[0].includes("**/*.sbt") && watched[0].includes("**/project/build.properties"), watched)
    check(where + "an sbt build without export: the export begun, nothing typed meanwhile", begun && c.diagnosticsSince(m).length === 0 && checkingTokens(c).length === 0, { begun: progressOf(c, "begin"), published: c.diagnosticsSince(m).map((p) => p.uri) })
    const ended = await until(() => endedAll(c, exportTokens(c)), 10000)
    await sleep(300)
    check(where + `the export written at ${place}, no session before a file of it is opened`, ended && existsSync(exportFile) && (!under || !existsSync(join(root, "teq.lock"))) && checkingTokens(c).length === 0, { ended, begun: progressOf(c, "begin") })
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(demo), languageId: "scala", version: 1, text: readFileSync(demo, "utf-8") } })
    const diags = await c.waitDiagnostics(m, uriOf(demo), (ds) => ds.length > 0, 10000)
    check(where + "a file of the export's project opened: its session's diagnostics", diags?.length === 1 && diags[0].message.includes("type mismatch"), diags)
    const refs = await c.result("textDocument/references", { ...at_(demo, "def greet", 0, 4), context: { includeDeclaration: true } })
    check(where + "navigation from the export", sameLocs(refs, [at(demo, "def greet", 0, 4, 5), at(demo, 'greet("you")', 0, 0, 5)]), refs)
    // The build changed: nothing runs again, a warning on the build file names the change and
    // the command.
    const m2 = c.mark()
    await changeBuild(c, root)
    const warned = await c.waitDiagnostics(m2, uriOf(buildFile), (ds) => ds.length === 1 && ds[0].severity === 2, 5000)
    await sleep(2500)
    check(
      where + "a build change exports nothing again: a warning on the build file naming the change and the command",
      warned?.[0]?.message.includes("build.sbt changed") && warned[0].message.includes("sbt teqExportAll") && exportTokens(c).length === 1 && runs() === 1,
      { warned, tokens: exportTokens(c).length, runs: runs() },
    )
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
  } catch (e) {
    check(where + "the sbt build scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
  // Opened again: the export exists, nothing runs, and the warning stands from the start; the
  // export written anew withdraws it.
  const c2 = new Client(root, sbtEnv)
  try {
    await c2.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: progressCapabilities })
    c2.notify("initialized", {})
    const warned = await c2.waitDiagnostics(0, uriOf(buildFile), (ds) => ds.length === 1 && ds[0].severity === 2, 5000)
    check(where + "opened again: no export runs, the stale warning from the start", warned?.length === 1 && exportTokens(c2).length === 0 && runs() === 1, { warned, begun: progressOf(c2, "begin") })
    const lock = parseLock(readFileSync(exportFile, "utf-8"))
    lock.inputs.files["build.sbt"] = createHash("sha256").update(readFileSync(buildFile)).digest("hex")
    writeFileSync(exportFile, lockText(lock))
    const m = c2.mark()
    c2.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(exportFile), type: 2 }] })
    const cleared = await c2.waitDiagnostics(m, uriOf(buildFile), (ds) => ds.length === 0, 5000)
    check(where + "the export written anew: the warning withdrawn", cleared?.length === 0, cleared)
    // The build file checked out with CRLF where the export read LF: stale all the same, the
    // warning noting that it differs by line ends alone, with the remedy in the place of the
    // command; LF again withdraws it.
    const lf = readFileSync(buildFile, "utf-8")
    const m3 = c2.mark()
    writeFileSync(buildFile, lf.replaceAll("\n", "\r\n"))
    c2.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(buildFile), type: 2 }] })
    const noted = await c2.waitDiagnostics(m3, uriOf(buildFile), (ds) => ds.length === 1 && ds[0].severity === 2, 5000)
    check(
      where + "a CRLF checkout of the build file: the stale warning, noting the line ends and the remedy",
      noted?.[0]?.message.startsWith("teq.lock was written before build.sbt changed\nbuild.sbt differs") && !noted[0].message.includes("sbt teqExportAll") && noted[0].message.includes("\nbuild.sbt differs from teq.lock's record by line ends alone, LF against CRLF: .gitattributes keeps the build's files LF in every checkout with *.sbt text eol=lf"),
      noted,
    )
    const m4 = c2.mark()
    writeFileSync(buildFile, lf)
    c2.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(buildFile), type: 2 }] })
    const again = await c2.waitDiagnostics(m4, uriOf(buildFile), (ds) => ds.length === 0, 5000)
    check(where + "the build file LF again: the warning withdrawn", again?.length === 0, again)
    await c2.result("shutdown", null)
    c2.notify("exit", null)
    await Promise.race([c2.exited, sleep(5000)])
  } catch (e) {
    check(where + "the sbt build's second opening ran to its end", false, `${e.stack}\nstderr: ${c2.stderr.slice(-1000)}`)
    c2.proc.kill()
  }
}

// The export under target/teq/ removed (sbt's clean of a root target there): one export again once
// the change has rested, after a watched-files notification and, a second time, found by the
// periodic search alone; the lock written again and the project answering again.
async function sbtLockGone() {
  const root = sbtCopy("sbt-gone")
  writeFileSync(join(root, "fake-sbt-target"), "")
  const demo = join(root, "src/Demo.scala")
  const exportFile = join(root, "target/teq/teq.lock")
  const runs = () => (existsSync(join(root, "fake-sbt-runs")) ? readFileSync(join(root, "fake-sbt-runs"), "utf-8").trim().split("\n").length : 0)
  const c = new Client(root, sbtEnv)
  const refs = () => c.result("textDocument/references", { ...at_(demo, "def greet", 0, 4), context: { includeDeclaration: true } })
  const greetRefs = [at(demo, "def greet", 0, 4, 5), at(demo, 'greet("you")', 0, 0, 5)]
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: progressCapabilities })
    c.notify("initialized", {})
    const first = await until(() => existsSync(exportFile) && endedAll(c, exportTokens(c)) && exportTokens(c).length === 1, 10000)
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(demo), languageId: "scala", version: 1, text: readFileSync(demo, "utf-8") } })
    const before = await refs()
    rmSync(exportFile)
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(exportFile), type: 3 }] })
    const again = await until(() => runs() === 2 && existsSync(exportFile) && endedAll(c, exportTokens(c)), 20000)
    const after = await refs()
    check("the export under target/teq/ removed and notified: one export again, the project answering again", first && sameLocs(before, greetRefs) && again && sameLocs(after, greetRefs), { first, again, runs: runs(), before, after })
    rmSync(exportFile)
    const found = await until(() => runs() === 3 && existsSync(exportFile) && endedAll(c, exportTokens(c)), 30000)
    await sleep(2500)
    const last = await refs()
    check("the export removed with no notification: the periodic search finds it gone, one export again", found && runs() === 3 && sameLocs(last, greetRefs), { found, runs: runs(), last })
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
  } catch (e) {
    check("the removed export scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// The run's processes end with the server; its output is cut to a tail while it runs.
async function sbtRunLifetime() {
  const hanging = sbtCopy("sbt-hang", "hang")
  const c = new Client(hanging, sbtEnv)
  let pids = []
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(hanging), capabilities: progressCapabilities })
    c.notify("initialized", {})
    const begun = await until(() => exportTokens(c).length === 1 && existsSync(join(hanging, "fake-sbt-child")), 5000)
    const sbtPid = Number(execSync(`pgrep -P ${c.proc.pid}`).toString().trim().split("\n")[0])
    const sleepPid = Number(readFileSync(join(hanging, "fake-sbt-child"), "utf-8").trim())
    pids = [sbtPid, sleepPid]
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
    await sleep(300)
    const alive = pids.filter((pid) => {
      try {
        process.kill(pid, 0)
        return true
      } catch {
        return false
      }
    })
    check("the server's end ends sbt and the processes it started", begun && pids.every((p) => p > 0) && alive.length === 0, { pids, alive })
  } catch (e) {
    check("the lifetime scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  } finally {
    for (const pid of pids) {
      try {
        process.kill(pid, "SIGKILL")
      } catch {}
    }
  }
  // The output cut to a tail during the run: the file under the cache stays small (a sample
  // taken between a write and its cut may see one write past the cap), the failure's
  // diagnostic ends with sbt's last line.
  const verbose = sbtCopy("sbt-verbose", "verbose")
  const cache = join(work, "cache")
  const c2 = new Client(verbose, { ...sbtEnv, TEQ_CACHE_DIR: cache })
  try {
    await c2.result("initialize", { processId: null, rootUri: uriOf(verbose), capabilities: progressCapabilities })
    const m = c2.mark()
    c2.notify("initialized", {})
    let largest = 0
    const deadline = Date.now() + 15000
    let diags
    while (Date.now() < deadline) {
      try {
        for (const f of readdirSync(join(cache, "sbt"))) {
          if (f.startsWith("export-")) largest = Math.max(largest, statSync(join(cache, "sbt", f)).size)
        }
      } catch {}
      diags = c2.diagnosticsSince(m).find((p) => p.uri === uriOf(join(verbose, "build.sbt")) && p.diagnostics.length > 0)?.diagnostics
      if (diags) break
      await sleep(50)
    }
    check("the run's output is cut to a tail: the file stays under the cap, the last line reaches the diagnostic", largest > 0 && largest < 5 * 1024 * 1024 + 64 * 1024 && diags?.[0]?.message.includes("the last line of a verbose failure"), { largest, message: diags?.[0]?.message.slice(-120) })
    await c2.result("shutdown", null)
    c2.notify("exit", null)
    await Promise.race([c2.exited, sleep(5000)])
  } catch (e) {
    check("the verbose scenario ran to its end", false, `${e.stack}\nstderr: ${c2.stderr.slice(-1000)}`)
    c2.proc.kill()
  }
}

// What stops an export before or after it runs, each told on the build file; a failed export run
// again once the build changes; a build that names sbt-teq itself; the folder inside a build; a
// build below the folder.
async function sbtRefusals() {
  const cases = [
    ["a failing build", sbtCopy("sbt-fail", "fail"), sbtEnv, "scalaVersio"],
    ["sbt run as a client", sbtCopy("sbt-client", "client"), sbtEnv, "client of a running server"],
    ["an sbt 1 build", (() => { const r = sbtCopy("sbt-one"); writeFileSync(join(r, "project/build.properties"), "sbt.version=1.10.0\n"); return r })(), sbtEnv, "needs sbt 2"],
    // An empty directory: `/usr/bin` holds a packaged sbt on some machines.
    ["no sbt on the PATH", sbtCopy("sbt-none"), { PATH: (() => { const d = join(work, "empty-path"); mkdirSync(d, { recursive: true }); return d })() }, "sbt is not on the PATH"],
  ]
  for (const [name, root, env, expected] of cases) {
    const c = new Client(root, env)
    try {
      await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: progressCapabilities })
      const m = c.mark()
      c.notify("initialized", {})
      const diags = await c.waitDiagnostics(m, uriOf(join(root, "build.sbt")), (ds) => ds.length > 0, 10000)
      const children = execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
      check(`${name}: the reason on build.sbt, no session`, diags?.length === 1 && diags[0].message.includes(expected) && children === 0 && checkingTokens(c).length === 0, { diags, children })
      if (name === "a failing build") {
        // Fixed and changed: the export runs again once the change rests, and is read.
        writeFileSync(join(root, "fake-sbt-mode"), "one")
        const m2 = c.mark()
        await changeBuild(c, root)
        const cleared = await c.waitDiagnostics(m2, uriOf(join(root, "build.sbt")), (ds) => ds.length === 0, 10000)
        c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(join(root, "src/Demo.scala")), languageId: "scala", version: 1, text: readFileSync(join(root, "src/Demo.scala"), "utf-8") } })
        const typed = await c.waitDiagnostics(m2, uriOf(join(root, "src/Demo.scala")), (ds) => ds.length > 0, 15000)
        check("a failed export runs again once the build changes: the reason withdrawn, the export read", cleared?.length === 0 && exportTokens(c).length === 2 && typed?.length === 1, { cleared, tokens: exportTokens(c).length, typed })
      }
      await c.result("shutdown", null)
      c.notify("exit", null)
      await Promise.race([c.exited, sleep(5000)])
    } catch (e) {
      check(`${name} ran to its end`, false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
      c.proc.kill()
    }
  }
  // A failed export, then the developer's own: the failure withdrawn, nothing run again.
  const manual = sbtCopy("sbt-manual", "fail")
  const c6 = new Client(manual, sbtEnv)
  try {
    await c6.result("initialize", { processId: null, rootUri: uriOf(manual), capabilities: progressCapabilities })
    c6.notify("initialized", {})
    const failed = await c6.waitDiagnostics(0, uriOf(join(manual, "build.sbt")), (ds) => ds.length > 0, 10000)
    const digest = (f) => createHash("sha256").update(readFileSync(join(manual, f))).digest("hex")
    const files = { "build.sbt": digest("build.sbt"), "project/build.properties": digest("project/build.properties") }
    writeFileSync(join(manual, "teq.lock"), lockText({ teq: "0.1.2", format: 1, binaries: {}, inputs: { files }, repositories: [], projects: { root: { platform: "js", configurations: { compile: { sources: ["src"] } } } } }))
    const m = c6.mark()
    c6.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(join(manual, "teq.lock")), type: 1 }] })
    const cleared = await c6.waitDiagnostics(m, uriOf(join(manual, "build.sbt")), (ds) => ds.length === 0, 5000)
    check("the developer's export after a failed one: the failure withdrawn, nothing run again", failed?.length === 1 && cleared?.length === 0 && exportTokens(c6).length === 1, { failed, cleared, tokens: exportTokens(c6).length })
    await c6.result("shutdown", null)
    c6.notify("exit", null)
    await Promise.race([c6.exited, sleep(5000)])
  } catch (e) {
    check("the manual export scenario ran to its end", false, `${e.stack}\nstderr: ${c6.stderr.slice(-1000)}`)
    c6.proc.kill()
  }
  // A build naming sbt-teq itself: its own plugin exports it, no plugin file added.
  const own = sbtCopy("sbt-own")
  writeFileSync(join(own, "project/plugins.sbt"), 'addSbtPlugin("build.teq" % "sbt-teq" % "0.1.6")\n')
  const c5 = new Client(own, sbtEnv)
  try {
    await c5.result("initialize", { processId: null, rootUri: uriOf(own), capabilities: progressCapabilities })
    c5.notify("initialized", {})
    const ended = await until(() => exportTokens(c5).length === 1 && endedAll(c5, exportTokens(c5)), 10000)
    await sleep(200)
    check("a build naming sbt-teq: exported by its own plugin", ended && existsSync(join(own, "teq.lock")) && !c5.diagnosticsSince(0).some((p) => p.diagnostics.length > 0), { ended, published: c5.diagnosticsSince(0) })
    await c5.result("shutdown", null)
    c5.notify("exit", null)
    await Promise.race([c5.exited, sleep(5000)])
  } catch (e) {
    check("the own-plugin scenario ran to its end", false, `${e.stack}\nstderr: ${c5.stderr.slice(-1000)}`)
    c5.proc.kill()
  }
  // The folder inside a build (within its repository): the build above it is exported, and a
  // file of the folder opened typed by its project.
  const inside = sbtCopy("sbt-inside")
  mkdirSync(join(inside, ".git"))
  const c4 = new Client(join(inside, "src"), sbtEnv)
  try {
    await c4.result("initialize", { processId: null, rootUri: uriOf(join(inside, "src")), capabilities: progressCapabilities })
    const m = c4.mark()
    c4.notify("initialized", {})
    await until(() => exportTokens(c4).length === 1 && endedAll(c4, exportTokens(c4)), 10000)
    c4.notify("textDocument/didOpen", { textDocument: { uri: uriOf(join(inside, "src/Demo.scala")), languageId: "scala", version: 1, text: readFileSync(join(inside, "src/Demo.scala"), "utf-8") } })
    const diags = await c4.waitDiagnostics(m, uriOf(join(inside, "src/Demo.scala")), (ds) => ds.length > 0, 15000)
    check("a folder inside a build: the build above exported, its project typing", exportTokens(c4).length === 1 && diags?.length === 1 && existsSync(join(inside, "teq.lock")), { begun: progressOf(c4, "begin"), diags })
    await c4.result("shutdown", null)
    c4.notify("exit", null)
    await Promise.race([c4.exited, sleep(5000)])
  } catch (e) {
    check("the folder-inside-a-build scenario ran to its end", false, `${e.stack}\nstderr: ${c4.stderr.slice(-1000)}`)
    c4.proc.kill()
  }
  // The build a level below the folder: exported where it is.
  const mono = join(work, "link/mono")
  mkdirSync(mono, { recursive: true })
  cpSync(join(here, "sbt"), join(mono, "backend"), { recursive: true })
  const c3 = new Client(mono, sbtEnv)
  try {
    await c3.result("initialize", { processId: null, rootUri: uriOf(mono), capabilities: progressCapabilities })
    const m = c3.mark()
    c3.notify("initialized", {})
    await until(() => exportTokens(c3).length === 1 && endedAll(c3, exportTokens(c3)), 10000)
    const demo = join(mono, "backend/src/Demo.scala")
    c3.notify("textDocument/didOpen", { textDocument: { uri: uriOf(demo), languageId: "scala", version: 1, text: readFileSync(demo, "utf-8") } })
    const diags = await c3.waitDiagnostics(m, uriOf(demo), (ds) => ds.length > 0, 15000)
    check("a build below the folder: exported there, its session typing", exportTokens(c3).length === 1 && diags?.length === 1 && existsSync(join(mono, "backend/teq.lock")), { begun: progressOf(c3, "begin"), diags })
    await c3.result("shutdown", null)
    c3.notify("exit", null)
    await Promise.race([c3.exited, sleep(5000)])
  } catch (e) {
    check("the nested build scenario ran to its end", false, `${e.stack}\nstderr: ${c3.stderr.slice(-1000)}`)
    c3.proc.kill()
  }
}

// --- Sessions on demand: one configuration's ending another's, the cap, the idle stop ----------

async function sessionLimits() {
  const root = join(work, "link/limits")
  for (const d of ["a/src", "a/test", "b/src", "shared/src"]) mkdirSync(join(root, d), { recursive: true })
  const a = join(root, "a/src/A.scala")
  const aTest = join(root, "a/test/ATest.scala")
  const b = join(root, "b/src/B.scala")
  const shared = join(root, "shared/src/S.scala")
  writeFileSync(a, 'object A:\n  def one: Int = 1\n  val wrong: Int = "no"\n')
  writeFileSync(aTest, "object ATest:\n  def check: Boolean = A.one == 1\n")
  writeFileSync(b, "object B:\n  def two: Int = 2\n")
  writeFileSync(shared, "object S:\n  def three: Int = 3\n")
  writeExport(join(root, "teq.lock"), { a: { platform: "js", sources: ["a/src", "shared/src"], test: ["a/test"] }, b: { platform: "js", sources: ["b/src", "shared/src"] } })
  const c = new Client(root)
  const children = () => execSync(`pgrep -P ${c.proc.pid} || true`).toString().trim().split("\n").filter(Boolean).length
  const begun = (name) => progressOf(c, "begin").filter((p) => p.value.message === `checking ${name}`).length
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: progressCapabilities, initializationOptions: { maxSessions: 1, sessionIdleSeconds: 3 } })
    c.notify("initialized", {})
    let r = await c.result("textDocument/hover", at_(shared, "def three", 0, 4))
    await sleep(500)
    check("a file of two configurations none runs, under a cap of one: one session started", r?.contents?.value?.includes("def S.three: Int") && progressOf(c, "begin").length === 1 && children() === 1, { begun: progressOf(c, "begin"), children: children() })
    r = await c.result("textDocument/hover", at_(a, "def one", 0, 4))
    const aDiags = await c.waitDiagnostics(0, uriOf(a), (ds) => ds.length === 1, 10000)
    check("a file of a's main sources: a/compile's session", r?.contents?.value?.includes("def A.one: Int") && begun("a/compile") === 1 && children() === 1 && aDiags?.length === 1, { r, begun: progressOf(c, "begin"), children: children() })
    r = await c.result("textDocument/definition", at_(aTest, "A.one", 0, 2))
    await sleep(300)
    check("a file of a's tests: a/test's session, a/compile's ended, its closure inside", sameLocs(r, [at(a, "def one", 0, 4, 3)]) && begun("a/test") === 1 && children() === 1, { r, begun: progressOf(c, "begin"), children: children() })
    r = await c.result("textDocument/hover", at_(b, "def two", 0, 4))
    await sleep(300)
    check("past the cap: b's session started, a/test's stopped", r?.contents?.value?.includes("def B.two: Int") && begun("b/compile") === 1 && children() === 1, { r, children: children() })
    r = await c.result("textDocument/hover", at_(shared, "def three", 0, 4))
    await sleep(300)
    check("a file of the shared root while one owner runs: served by it, the stopped owner left stopped", r?.contents?.value?.includes("def S.three: Int") && begun("a/test") === 1 && children() === 1, { r, begun: progressOf(c, "begin"), children: children() })
    r = await c.result("textDocument/hover", at_(a, "def one", 0, 4))
    await sleep(300)
    check("a file of a again: a/test's session started again, b's stopped", r?.contents?.value?.includes("def A.one: Int") && begun("a/test") === 2 && begun("a/compile") === 1 && children() === 1, { r, begun: progressOf(c, "begin"), children: children() })
    const m = c.mark()
    const idle = await until(() => children() === 0, 8000)
    await sleep(300)
    check("unused for the idle time: the session stopped, its diagnostics kept", idle && !c.diagnosticsSince(m).some((p) => p.uri === uriOf(a)), { children: children(), published: c.diagnosticsSince(m) })
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
  } catch (e) {
    check("the session limits scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// --- The idle poll: a session left alone for a minute -----------------------------------------

/** An idle session polls the disk 3 s (`IDLE`) after its last build answered, by a deadline of
 * its own, and a poll that finds nothing types nothing and is never shown as work in progress,
 * whatever it costs (docs/TARGETS.md, "The language server"). The export has the application's
 * shape: a buildInfo generator naming the class directory of a project in the closure, and a
 * command generator, both run before every poll: the generated files keep their texts and times.
 * Read from the server's trace after a minute: as many polls as the rule gives, each 3 s after
 * the answer before it. Then a poll the child holds (stopped) past the progress delay is neither
 * shown nor followed by another, the next coming 3 s after its answer; a save while a poll is
 * held, its generators held too, shows the poll; a file written on disk without a notification
 * is published by the next poll; last, the command generator made to fail is tried again every
 * 3 s, not at every wake of the loop, and once it succeeds again its output is typed. tests/lsp.sh
 * runs it once, apart from the passes at each worker count, since it takes a minute. */
async function idleMinute() {
  const root = join(work, "link/idle")
  for (const d of ["a/src", "b/src"]) mkdirSync(join(root, d), { recursive: true })
  const use = join(root, "a/src/Use.scala")
  writeFileSync(use, "object Use:\n  val name: String = idle.build.BuildInfo.name\n  val classes: java.io.File = idle.build.BuildInfo.classes\n  val v: Int = Gen.v + B.one\n")
  writeFileSync(join(root, "b/src/B.scala"), "object B:\n  def one: Int = 1\n")
  const quiet = join(root, "a/src/Quiet.scala")
  writeFileSync(quiet, "object Quiet:\n  val n: Int = 1\n")
  writeFileSync(join(root, "value.txt"), "1\n")
  // Waits while a gate stands, counts its runs, fails while `fail` stands, and writes Gen.scala
  // only when its text changes.
  writeFileSync(join(root, "gen.sh"), 'while [ -e gate ]; do sleep 0.05; done; echo run >> runs.txt; if [ -e fail ]; then exit 1; fi; mkdir -p a/gen && printf "object Gen:\\n  val v = %s\\n" "$(cat value.txt)" > a/gen/Gen.new && if cmp -s a/gen/Gen.new a/gen/Gen.scala; then rm a/gen/Gen.new; else mv a/gen/Gen.new a/gen/Gen.scala; fi\n')
  const infoFile = "target/teq/a/compile/src_managed/sbt-buildinfo/BuildInfo.scala"
  const buildInfo = { kind: "buildInfo", output: infoFile, package: "idle.build", object: "BuildInfo", members: ["name", "version", "classes"], static: { name: "a", version: "0.1.0" }, dynamic: { classes: { classDirectory: { project: "b", configuration: "compile" } } } }
  const command = { kind: "command", run: ["sh", "gen.sh"], cwd: ".", inputs: ["value.txt"], outputs: ["a/gen"] }
  writeExport(join(root, "teq.lock"), {
    a: { platform: "jvm", sources: ["a/src", "a/gen", "target/teq/a/compile/src_managed"], classpath: [{ project: "b", configuration: "compile" }], generators: [buildInfo, command] },
    b: { platform: "jvm", sources: ["b/src"] },
  })
  const trace = join(work, "idle.trace")
  rmSync(trace, { force: true })
  const c = new Client(root, { TEQ_LSP_TRACE_FILE: trace })
  const IDLE = 3000
  // How late a poll may be sent after its deadline: the loop's wake and a loaded machine. A
  // missed poll is 3 s late, a doubled one early.
  const late = 400
  const tokens = () => c.requests.filter((r) => r.method === "window/workDoneProgress/create").length
  const times = () => [infoFile, "a/gen/Gen.scala"].map((f) => statSync(join(root, f)).mtimeMs)
  const runs = () => readFileSync(join(root, "runs.txt"), "utf-8").split("\n").filter(Boolean).length
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: progressCapabilities })
    c.notify("initialized", {})
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(use), languageId: "scala", version: 1, text: readFileSync(use, "utf-8") } })
    const h = await c.result("textDocument/hover", at_(use, "BuildInfo.classes", 0, 10))
    const first = await traced(trace, 0, (e) => e.who === "session" && e.event === "answered", "the first build's answer")
    check("the idle session's first build: the generated sources typed", first.rest[0] === "full" && h?.contents?.value?.includes("val idle.build.BuildInfo.classes: File"), { first, h, stderr: c.stderr.slice(-400) })
    await until(() => c.notifications.some((n) => n.method === "$/progress" && n.params.value.kind === "end"), 5000)
    const [shown, before, ran] = [tokens(), times(), runs()]
    await sleep(60000)
    const end = Date.now()
    const events = traceOf(trace)
    const polls = events.filter((e) => e.who === "server" && e.event === "build" && e.ms > first.ms && e.ms <= end)
    const answers = new Map(events.filter((e) => e.who === "session" && e.event === "answered").map((e) => [e.id, e]))
    // The rule, from the first answer: a poll 3 s after each answer, until the minute is over.
    let ruled = true
    let last = first
    for (const poll of polls) {
      const gap = poll.ms - last.ms
      if (!poll.rest.includes("poll") || gap < IDLE || gap > IDLE + late) ruled = false
      last = answers.get(poll.id) ?? { ms: Infinity }
    }
    const due = last.ms === Infinity || end - last.ms < IDLE + late
    const gaps = polls.map((p, i) => p.ms - (i === 0 ? first : answers.get(polls[i - 1].id))?.ms)
    console.log(`lsp: an idle minute: ${polls.length} polls, ${Math.min(...gaps)}-${Math.max(...gaps)} ms after the answer before each`)
    check("an idle minute, read from the trace: as many polls as the rule gives, each 3 s after the answer before it", polls.length >= 15 && ruled && due, { gaps, polls: polls.map((p) => [p.id, ...p.rest]), end: end - last.ms })
    check("an idle minute's polls type nothing", polls.every((p) => !answers.has(p.id) || same(answers.get(p.id).rest, ["retyped", "0"])), polls.map((p) => answers.get(p.id)))
    check("an idle minute's polls are not shown as work in progress", tokens() === shown, progressOf(c, "begin"))
    check("an idle minute's generators: buildInfo's file and the command's output keep their times, the command not run again", same(times(), before) && runs() === ran, { before, after: times(), ran, runs: runs() })
    // A poll held past the progress delay: the child stopped once an answer has been read, so
    // that the next poll is sent and waits.
    const child = Number(execSync(`pgrep -P ${c.proc.pid} -f @`).toString().trim().split("\n")[0])
    let mark = traceOf(trace).length
    await traced(trace, mark, (e) => e.who === "server" && e.event === "published", "a poll's answer read")
    mark = traceOf(trace).length
    execSync(`kill -STOP ${child}`)
    let held
    try {
      held = await traced(trace, mark, (e) => e.who === "server" && e.event === "build", "the next poll", IDLE + 2000)
      await sleep(IDLE + 1000)
    } finally {
      execSync(`kill -CONT ${child}`)
    }
    const meanwhile = traceOf(trace).slice(mark).filter((e) => e.who === "server" && e.event === "build")
    const answered = await traced(trace, mark, (e) => e.who === "session" && e.event === "answered" && e.id === held.id, "the held poll's answer")
    const next = await traced(trace, mark, (e) => e.who === "server" && e.event === "build" && e.id > held.id, "the poll after the held one", IDLE + 2000)
    check(
      "a poll held past the progress delay: not shown, no other sent meanwhile, the next 3 s after its answer",
      held.rest.includes("poll") && tokens() === shown && meanwhile.length === 1 && next.ms - answered.ms >= IDLE && next.ms - answered.ms <= IDLE + late,
      { held, meanwhile, gap: next.ms - answered.ms, begun: progressOf(c, "begin") },
    )
    // A save while a poll is held, the generators it runs first held by their gate: the change
    // waits behind the poll, which is shown; every token ends once both have answered.
    mark = traceOf(trace).length
    await traced(trace, mark, (e) => e.who === "server" && e.event === "published", "a poll's answer read")
    mark = traceOf(trace).length
    execSync(`kill -STOP ${child}`)
    let behind
    try {
      await traced(trace, mark, (e) => e.who === "server" && e.event === "build", "the next poll", IDLE + 2000)
      writeFileSync(join(root, "gate"), "")
      writeFileSync(join(root, "value.txt"), "2\n")
      c.notify("textDocument/didSave", { textDocument: { uri: uriOf(use) } })
      behind = await until(() => tokens() > shown, 2000)
    } finally {
      rmSync(join(root, "gate"), { force: true })
      execSync(`kill -CONT ${child}`)
    }
    const saved = await traced(trace, mark, (e) => e.who === "server" && e.event === "build" && !e.rest.includes("poll"), "the save's build", 10000)
    await traced(trace, mark, (e) => e.who === "session" && e.event === "answered" && e.id === saved.id, "the save's build answered", 10000)
    const ended = await until(() => endedAll(c, checkingTokens(c)), 5000)
    check("a save while a poll is held, its generators running: the poll shown, the generator run once, every token ended after", behind && ended && runs() === ran + 1, { behind, ended, runs: runs(), ran, begun: progressOf(c, "begin"), end: progressOf(c, "end") })
    // A file written without a notification: the next poll types it, within the idle period and
    // the polls' own time.
    const m = c.mark()
    writeFileSync(quiet, 'object Quiet:\n  val n: Int = "one"\n')
    const written = Date.now()
    const d = await c.waitDiagnostics(m, uriOf(quiet), (ds) => ds.length === 1, 15000)
    check("an edit on disk without a notification: published by the next poll", d?.[0]?.message.includes("type mismatch") && Date.now() - written < IDLE + 3000, { d, ms: Date.now() - written })
    // The command generator failing: its input changed without a notification and the generator
    // made to exit 1. A failed run sends no build, so its attempts are the generator's own runs,
    // each timed by the time runs.txt took: one per poll, 3 s apart, not one at every wake.
    const runsFile = join(root, "runs.txt")
    const m4 = c.mark()
    const failedFrom = runs()
    const attempts = []
    let seen = failedFrom
    const sampler = setInterval(() => {
      const n = runs()
      if (n !== seen) attempts.push({ added: n - seen, ms: statSync(runsFile).mtimeMs })
      seen = n
    }, 20)
    writeFileSync(join(root, "fail"), "")
    writeFileSync(join(root, "value.txt"), "3\n")
    await until(() => attempts.length >= 3, 3 * IDLE + 2000)
    clearInterval(sampler)
    // Right after a failed attempt, the next poll 3 s away: the failure cleared and the input
    // changed again, to a text that types `Use` wrongly.
    rmSync(join(root, "fail"))
    const m5 = c.mark()
    const recoveredFrom = runs()
    writeFileSync(join(root, "value.txt"), '"three"\n')
    const spacing = attempts.slice(1).map((a, i) => Math.round(a.ms - attempts[i].ms))
    const failure = c.diagnosticsSince(m4).find((p) => p.diagnostics.some((x) => x.message.includes("`sh gen.sh` failed")))
    console.log(`lsp: an idle minute's failing generator: ${runs() - failedFrom} attempts, ${Math.min(...spacing)}-${Math.max(...spacing)} ms apart`)
    check(
      "a failing generator: tried once per poll, 3 s apart, its failure published",
      attempts.length === 3 && attempts.every((a) => a.added === 1) && spacing.every((g) => g >= IDLE - 100 && g <= IDLE + late) && runs() === failedFrom + 3 && failure !== undefined,
      { attempts, spacing, runs: runs() - failedFrom, failure },
    )
    const wrong = await c.waitDiagnostics(m5, uriOf(use), (ds) => ds.length === 1, 2 * IDLE + 2000)
    await sleep(IDLE + 500)
    const cleared = failure && !c.latestDiagnostics(failure.uri).some((x) => x.message.includes("`sh gen.sh` failed"))
    check(
      "the generator succeeding again: run once by the next poll, its output typed, its failure withdrawn",
      runs() === recoveredFrom + 1 && wrong?.[0]?.message.includes("type mismatch") && cleared,
      { runs: runs() - recoveredFrom, wrong, cleared, latest: failure && c.latestDiagnostics(failure.uri) },
    )
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(5000)])
  } catch (e) {
    check("the idle minute ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// `LSP_ONLY=<scenario>,...` runs those alone, for a run by hand.
const only = process.env.LSP_ONLY?.split(",")
// --- Unused imports: the hints of a session outside the flag, its warnings under it -----------

async function unusedImports() {
  encoding = "utf-16"
  const dir = join(work, "link/unused")
  mkdirSync(join(dir, "src"), { recursive: true })
  const use = join(dir, "src/Use.scala")
  const crlfUse = join(dir, "src/CrlfUse.scala")
  const sig = join(dir, "src/Sig.scala")
  // An astral character before the import on its line: the range counts UTF-16 units.
  const useText = (body) => `object Lib:\n  val used = 1\n  val unused = 2\n  class T\n\nobject Use:\n  val emoji = "😀"; import Lib.{used, unused}\n  def f = ${body}\n`
  writeFileSync(use, useText("used"))
  writeFileSync(crlfUse, "object CrlfUse:\r\n  import Lib.{used,\r\n    unused}\r\n  def g = used\r\n")
  const sigText = (body) => `object Sig:\n  import Lib.T\n  def g(t: T): Int = 1\n  def h = ${body}\n`
  writeFileSync(sig, sigText("2"))
  // A retype: `def f = n` to `def f = 0` keeps the inferred `Int`, the import's
  // only use gone; and a file import moved by a line above it.
  const retyped = join(dir, "src/Retyped.scala")
  const retypedText = (lead, body) => `${lead}object LibN { val n: Int = 1 }\nimport LibN.n\nobject UseN { def f = ${body} }\n`
  writeFileSync(retyped, retypedText("", "n"))
  // An annotation typed for the marks alone completes `LibA.value` on the way: the navigation
  // records of that completion stay.
  const annotated = join(dir, "src/Annotated.scala")
  writeFileSync(annotated, "class AnnA(x: Int) extends scala.annotation.StaticAnnotation\n@AnnA(LibA.value) class CA\nobject LibA {\n  def value = target\n  def target: Int = 1\n}\n")
  const exportFile = join(dir, "teq.lock")
  // Under --werror and without the flag: the hints count as nothing.
  writeExport(exportFile, { app: { platform: "js", sources: ["src"], flags: { werror: true } } })
  const c = new Client(dir)
  const unusedAt = (sev) => (ds) => ds.length === 1 && ds[0].severity === sev && same(ds[0].tags, [1]) && same(ds[0].range, at(use, "unused", 1).range) && ds[0].message === "unused import"
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(dir), capabilities: { workspace: { didChangeWatchedFiles: { dynamicRegistration: true } } } })
    const m = c.mark()
    c.notify("initialized", {})
    // Sessions start on demand: the open document starts the project's.
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(use), languageId: "scala", version: 1, text: useText("used") } })
    const hints = await c.waitDiagnostics(m, uriOf(use), (ds) => ds.length > 0)
    check("an unused import: a hint (severity 4) tagged Unnecessary at the selector, UTF-16 columns", unusedAt(4)(hints ?? []), hints)
    const crlfHints = await c.waitDiagnostics(m, uriOf(crlfUse), (ds) => ds.length > 0)
    check("an unused import after a CRLF line break inside braces: the selector's range", crlfHints?.length === 1 && crlfHints[0].severity === 4 && same(crlfHints[0].range, at(crlfUse, "unused").range), crlfHints)
    check("a session under --werror without the flag: hints alone, nothing an error or a warning", c.diagnosticsSince(m).every((p) => p.diagnostics.every((d) => d.severity === 4)) && c.latestDiagnostics(uriOf(sig)).length === 0, c.diagnosticsSince(m))
    // An edit removing the use makes the import unused; one adding it back clears it.
    const m2 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(use), version: 2 }, contentChanges: [{ text: useText("1") }] })
    const both = await c.waitDiagnostics(m2, uriOf(use), (ds) => ds.length === 2)
    check("an edit removing a use: its import unused", both?.length === 2 && both.every((d) => d.severity === 4 && same(d.tags, [1])) && same(both.map((d) => d.range).sort((a, b) => a.start.character - b.start.character), [at(use, "{used", 0, 1).range, at(use, "unused", 1).range]), both)
    const m3 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(use), version: 3 }, contentChanges: [{ text: useText("used + 1") }] })
    const again = await c.waitDiagnostics(m3, uriOf(use), (ds) => ds.length === 1)
    check("an edit adding the use back: its hint cleared", unusedAt(4)(again ?? []), again)
    const toTarget = await c.result("textDocument/definition", at_(annotated, "= target", 0, 2))
    check("a definition completed while an annotation is typed for the marks: its records kept", sameLocs(toTarget, [at(annotated, "def target", 0, 4, 6)]), toTarget)
    // A name used in a signature alone survives a retype of another body.
    const m4 = c.mark()
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(sig), languageId: "scala", version: 1, text: sigText("3") } })
    await c.waitDiagnostics(m4, uriOf(sig), () => true, 30000)
    const m5 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(sig), version: 2 }, contentChanges: [{ text: sigText("4") }] })
    const sigAfter = await c.waitDiagnostics(m5, uriOf(sig), () => true, 30000)
    check("an import used by a signature alone: no hint after another body's retype", (sigAfter ?? c.latestDiagnostics(uriOf(sig))).length === 0, sigAfter)
    // The retype of `def f = n`: the import becomes unused, the hint at its selector; moved by a
    // line above it, the import stays used where it now stands.
    const m7 = c.mark()
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(retyped), languageId: "scala", version: 1, text: retypedText("", "0") } })
    const nAt = (lead) => { const text = retypedText(lead, "0"); const at = text.indexOf("LibN.n") + 5; return { start: positionAt(text, at), end: positionAt(text, at + 1) } }
    const nHint = await c.waitDiagnostics(m7, uriOf(retyped), (ds) => ds.length === 1)
    check("the retype of `def f = n` to `def f = 0`: its import a hint", nHint?.length === 1 && nHint[0].severity === 4 && same(nHint[0].range, nAt("")), nHint)
    const m8 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(retyped), version: 2 }, contentChanges: [{ text: retypedText("\n", "n") }] })
    const moved = await c.waitDiagnostics(m8, uriOf(retyped), () => true, 30000)
    check("the import moved by a line and used again: no hint", (moved ?? c.latestDiagnostics(uriOf(retyped))).length === 0, moved)
    const m9 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(retyped), version: 3 }, contentChanges: [{ text: retypedText("\n", "0") }] })
    const movedHint = await c.waitDiagnostics(m9, uriOf(retyped), (ds) => ds.length === 1)
    const movedAt = (() => { const text = retypedText("\n", "0"); const at = text.indexOf("LibN.n") + 5; return { start: positionAt(text, at), end: positionAt(text, at + 1) } })()
    check("the moved import's use removed: the hint where the selector now stands", movedHint?.length === 1 && same(movedHint[0].range, movedAt), movedHint)
    // Under the flag (sbt's -Wunused:imports): the same selector, a warning with the tag.
    const m6 = c.mark()
    writeExport(exportFile, { app: { platform: "js", sources: ["src"], flags: { wunusedImports: true } } })
    c.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: uriOf(exportFile), type: 2 }] })
    const warned = await c.waitDiagnostics(m6, uriOf(use), unusedAt(2))
    check("a session under the flag: the unused import a warning (severity 2) tagged Unnecessary", unusedAt(2)(warned ?? []), warned ?? c.latestDiagnostics(uriOf(use)))
    // An error anywhere withholds every unused import under the flag, as scalac runs no phase
    // after a typer that reported one; its fix gives them back.
    const m10 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(sig), version: 3 }, contentChanges: [{ text: sigText('"no int"').replace("def h =", "def h: Int =") }] })
    const withheld = await c.waitDiagnostics(m10, uriOf(use), (ds) => ds.length === 0, 30000)
    check("an error elsewhere under the flag: the unused import withheld", withheld?.length === 0 && c.latestDiagnostics(uriOf(sig)).some((d) => d.severity === 1), { withheld, sig: c.latestDiagnostics(uriOf(sig)) })
    const m11 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(sig), version: 4 }, contentChanges: [{ text: sigText("5") }] })
    const back = await c.waitDiagnostics(m11, uriOf(use), unusedAt(2))
    check("the error fixed: the unused import a warning again", unusedAt(2)(back ?? []), back ?? c.latestDiagnostics(uriOf(use)))
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the unused imports' scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

// A session whose builds get no mapping for the type store's overlays (an address-space limit the
// server's children inherit, every full build asked of two workers and every build full) is typed by
// one worker, and the server says so in the client's log once, however many builds follow.
async function refusedMapping() {
  const root = join(work, "refused")
  cpSync(join(here, "bare"), root, { recursive: true })
  const c = new Client(root, { TEQ_SESSION_WORKERS: "2", TEQ_COMPACT_EVERY: "1" }, "sh", ["-c", 'ulimit -v 8000000 && exec "$0" lsp', teq])
  try {
    await c.result("initialize", { processId: null, rootUri: uriOf(root), capabilities: {} })
    c.notify("initialized", {})
    const file = join(root, "Hello.scala")
    const text = readFileSync(file, "utf8")
    const m = c.mark()
    c.notify("textDocument/didOpen", { textDocument: { uri: uriOf(file), languageId: "scala", version: 1, text } })
    await c.waitDiagnostics(m, uriOf(file), (ds) => ds.length > 0, 60000)
    const m2 = c.mark()
    c.notify("textDocument/didChange", { textDocument: { uri: uriOf(file), version: 2 }, contentChanges: [{ text: text.replace('"hello "', '"hi "') }] })
    await c.waitDiagnostics(m2, uriOf(file), (ds) => ds.length > 0, 60000)
    const notes = c.notifications.filter((n) => n.method === "window/logMessage" && n.params.message.includes("no memory mapping for the type store's overlays; one worker types the build"))
    check("a refused mapping told once in the client's log, a warning", notes.length === 1 && notes[0].params.type === 2, { notes, stderr: c.stderr.slice(-400) })
    await c.result("shutdown", null)
    c.notify("exit", null)
    await Promise.race([c.exited, sleep(10000)])
  } catch (e) {
    check("the refused mapping's scenario ran to its end", false, `${e.stack}\nstderr: ${c.stderr.slice(-1000)}`)
    c.proc.kill()
  }
}

const scenarios = { mainWorkspace, appliedValues, closedAlias, completion, completionCases, completionSnippets, completionRaces, signatureCases, signatureHelp, indexSize, cacheableAcrossCompletions, stoppedChild, busyChild, closedDocument, overtaken, overtakenAgain, bareRoot, libraries, libraryDocumentParked, libraryDocumentProducerReplaced, leanStd, leanStdSessions, leanStdComplete, stdDocumentRouting, sessionLimits, serverGenerators, sbtBuild, sbtLockGone, sbtRefusals, sbtRunLifetime, unusedImports, refusedMapping, idleMinute }
// Run only when named: the idle minute, which tests/lsp.sh runs once.
const named = new Set(["idleMinute"])
for (const [name, run] of Object.entries(scenarios)) {
  if (only ? only.includes(name) : !named.has(name)) await run()
}
console.log(`${passed} passed, ${failed} failed`)
process.exit(failed === 0 ? 0 : 1)
