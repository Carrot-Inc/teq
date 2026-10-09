// tests/wine/lsp.mjs <teq.exe> <workspace>: `teq lsp` of the Windows binary under wine, driven as
// a Windows editor drives it (tests/wine.sh): the workspace's URIs are its Windows paths', the
// drive spelt as VS Code spells it (`file:///z%3A/...`), and the answers must name the files by
// their own Windows URIs (`file:///Z:/...`). The workspace holds Hello.scala (tests/lsp/bare's)
// under a directory whose name has a space and a non-ASCII character. Checks a cold build's
// diagnostics and a definition (also through a case alias of the file's name), then a build held at
// its commit point (TEQ_LSP_TEST_BARRIER) while
// an edit comes: the server writes the build's cancel to the session's stdin, a pipe the server made
// (a Windows pipe, which the session peeks, where wine's over a Unix pipe cannot be), and the session
// stops there; the file deleted, closed and made again; its name's case changed on disk; last a
// diagnostic naming a non-ASCII identifier. With `idle` after the workspace, it writes there an
// export of the application's shape (a buildInfo generator naming the class directory of a project
// in the closure, and 1,500 small files, so that a poll here takes longer than the progress delay)
// and leaves its session idle for a minute: the checks of tests/lsp/driver.mjs's idleMinute, read
// from the server's trace, the class path's jars from COURSIER_CACHE. Prints `lsp: passed: <n> of
// <n>` last, exits 1 on a failure. On Windows itself (node's platform win32) it runs the exe directly on the workspace's
// own paths, a drive's or a share's (`\\server\share\...`, whose URIs are `file://server/share/...`).
import { spawn } from "node:child_process"
import { existsSync, mkdirSync, openSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs"
import { join, resolve } from "node:path"

const [exe, workspace, mode] = process.argv.slice(2)
const idle = mode === "idle"
let passed = 0
let failed = 0
function check(name, ok, detail) {
  if (ok) passed++
  else {
    failed++
    console.log(`FAIL ${name}: ${JSON.stringify(detail)}`)
  }
}
const native = process.platform === "win32"
// A Linux path's Windows one by wine's drive Z:, the Linux root (winepath would start a wine
// server that keeps its stdout, and the call would never end); on Windows the path itself.
const windows = (path) => (native ? resolve(path) : "Z:" + resolve(path).replace(/\//g, "\\"))
// The URI VS Code sends for a Windows path: a drive's in lower case and its colon escaped, a
// share's with the server as the authority.
const editorUri = (path) => {
  const w = windows(path).replace(/\\/g, "/")
  if (w.startsWith("//")) {
    const [server, ...rest] = w.slice(2).split("/")
    return `file://${server}/` + rest.map(encodeURIComponent).join("/")
  }
  return "file:///" + w[0].toLowerCase() + "%3A" + w.slice(2).split("/").map(encodeURIComponent).join("/")
}
// A URI as a comparable string: decoded, the drive's letter and a share's server in upper case.
const plain = (uri) =>
  decodeURIComponent(uri)
    .replace(/^file:\/\/\/([a-z]):/, (_, d) => `file:///${d.toUpperCase()}:`)
    .replace(/^file:\/\/([^/]+)\//, (_, server) => `file://${server.toUpperCase()}/`)

const hello = join(workspace, "Hello.scala")
const helloUri = editorUri(hello)
// wine's program writes nothing through a node pipe on stderr (a Unix socket), and stops there:
// its stderr goes to a file.
const files = idle ? "lsp-idle" : "lsp"
const errFile = join(workspace, "..", `${files}.stderr`)
const trace = join(workspace, "..", `${files}.trace`)
const barrier = join(workspace, "..", `${files}.barrier`)
const env = { ...process.env, TEQ_LSP_TRACE_FILE: windows(trace), TEQ_LSP_TEST_BARRIER: windows(barrier) }
/** A tree as a teq.lock, every key and string quoted (tests/lsp/driver.mjs's). */
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
const infoFile = "target/teq/a/compile/src_managed/sbt-buildinfo/BuildInfo.scala"
const use = join(workspace, "a/src/Use.scala")
const quiet = join(workspace, "a/src/Quiet.scala")
if (idle) {
  for (const d of ["a/src/many", "b/src"]) mkdirSync(join(workspace, d), { recursive: true })
  writeFileSync(use, "object Use:\n  val classes: java.io.File = idle.build.BuildInfo.classes\n  val v: Int = B.one\n")
  writeFileSync(quiet, "object Quiet:\n  val n: Int = 1\n")
  writeFileSync(join(workspace, "b/src/B.scala"), "object B:\n  def one: Int = 1\n")
  for (let k = 0; k < 1500; k++) writeFileSync(join(workspace, `a/src/many/F${k}.scala`), `object F${k}:\n  val v: Int = ${k}\n`)
  const buildInfo = { kind: "buildInfo", output: infoFile, package: "idle.build", object: "BuildInfo", static: { name: "a" }, dynamic: { classes: { classDirectory: { project: "b", configuration: "compile" } } } }
  const project = (name, sources, classpath, generators) => ({ base: name, platform: "jvm", description: {}, configurations: { compile: { sources, classpath: classpath.map((p) => ({ project: p, configuration: "compile" })), flags: {}, generators } } })
  const lock = { teq: "0.1.2", format: 1, binaries: {}, inputs: { files: {} }, repositories: [], jars: {}, projects: { a: project("a", ["a/src", "target/teq/a/compile/src_managed"], ["b"], [buildInfo]), b: project("b", ["b/src"], [], []) } }
  writeFileSync(join(workspace, "teq.lock"), lockText(lock))
}
const [program, args] = native ? [exe, ["lsp"]] : ["wine", [exe, "lsp"]]
// CreateProcess takes no current directory past 260 characters: the server starts in node's then.
const cwd = native && windows(workspace).length >= 248 ? undefined : workspace
const proc = spawn(program, args, { cwd, env, stdio: ["pipe", "pipe", openSync(errFile, "w")] })
// The trace's lines, `<who> <pid> <ms> <event> <id> ...`.
const traceOf = () => (existsSync(trace) ? readFileSync(trace, "utf8").split("\n").filter(Boolean).map((l) => l.split(" ")).map(([who, , ms, event, id, ...rest]) => ({ who, ms: Number(ms), event, id: Number(id), rest })) : [])
async function traced(pred, what) {
  const deadline = Date.now() + 60000
  for (;;) {
    const found = traceOf().find(pred)
    if (found) return found
    if (Date.now() > deadline) throw new Error(`no ${what} in the trace: ${JSON.stringify(traceOf().slice(-8))}`)
    await sleep(50)
  }
}
const stderrText = () => (existsSync(errFile) ? readFileSync(errFile, "utf8") : "")
let buffer = Buffer.alloc(0)
const waiting = new Map()
const notifications = []
const requests = []
proc.stdout.on("data", (d) => {
  buffer = Buffer.concat([buffer, d])
  for (;;) {
    const end = buffer.indexOf("\r\n\r\n")
    if (end < 0) return
    const length = Number(/Content-Length: *(\d+)/i.exec(buffer.subarray(0, end).toString())?.[1])
    if (buffer.length < end + 4 + length) return
    const msg = JSON.parse(buffer.subarray(end + 4, end + 4 + length).toString())
    buffer = buffer.subarray(end + 4 + length)
    if (msg.id !== undefined && msg.method === undefined) waiting.get(msg.id)?.(msg)
    else if (msg.id !== undefined) {
      requests.push(msg)
      write({ jsonrpc: "2.0", id: msg.id, result: null })
    }
    else notifications.push(msg)
  }
})
function write(msg) {
  const body = Buffer.from(JSON.stringify(msg))
  proc.stdin.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`), body]))
}
let nextId = 1
function request(method, params) {
  const id = nextId++
  const answer = new Promise((resolve) => waiting.set(id, resolve))
  write({ jsonrpc: "2.0", id, method, params })
  return Promise.race([
    answer,
    new Promise((_, reject) => setTimeout(() => reject(new Error(`${method}: no answer in 120 s; stderr: ${stderrText().slice(-400)}`)), 120000)),
  ]).then((m) => m.result ?? m)
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

/** The idle minute (tests/lsp/driver.mjs's idleMinute, its held poll left out: wine's processes
 * cannot be stopped from here). */
async function idleMinute() {
  const IDLE = 3000
  const late = 500
  const tokens = () => requests.filter((r) => r.method === "window/workDoneProgress/create").length
  const progress = () => notifications.filter((n) => n.method === "$/progress")
  await request("initialize", { processId: null, rootUri: editorUri(workspace), capabilities: { window: { workDoneProgress: true } } })
  write({ jsonrpc: "2.0", method: "initialized", params: {} })
  const text = readFileSync(use, "utf8")
  write({ jsonrpc: "2.0", method: "textDocument/didOpen", params: { textDocument: { uri: editorUri(use), languageId: "scala", version: 1, text } } })
  const h = await request("textDocument/hover", { textDocument: { uri: editorUri(use) }, position: { line: 1, character: text.split("\n")[1].lastIndexOf("classes") + 2 } })
  const first = await traced((e) => e.who === "session" && e.event === "answered", "the first build's answer")
  check("the idle session's first build: the generated sources typed", first.rest[0] === "full" && JSON.stringify(h ?? "").includes("BuildInfo.classes: File"), { first, h, stderr: stderrText().slice(-400) })
  const by = Date.now() + 10000
  while (Date.now() < by && !progress().some((n) => n.params.value.kind === "end")) await sleep(50)
  const generated = join(workspace, infoFile)
  const [shown, before] = [tokens(), statSync(generated).mtimeMs]
  await sleep(60000)
  const end = Date.now()
  const events = traceOf()
  const polls = events.filter((e) => e.who === "server" && e.event === "build" && e.ms > first.ms && e.ms <= end)
  const answers = new Map(events.filter((e) => e.who === "session" && e.event === "answered").map((e) => [e.id, e]))
  let ruled = true
  let last = first
  for (const poll of polls) {
    const gap = poll.ms - last.ms
    if (!poll.rest.includes("poll") || gap < IDLE || gap > IDLE + late) ruled = false
    last = answers.get(poll.id) ?? { ms: Infinity }
  }
  const due = last.ms === Infinity || end - last.ms < IDLE + late
  const gaps = polls.map((p, i) => p.ms - (i === 0 ? first : answers.get(polls[i - 1].id))?.ms)
  const took = polls.filter((p) => answers.has(p.id)).map((p) => answers.get(p.id).ms - p.ms)
  console.log(`lsp: an idle minute: ${polls.length} polls, ${Math.min(...gaps)}-${Math.max(...gaps)} ms after the answer before each, each taking ${Math.min(...took)}-${Math.max(...took)} ms`)
  check("an idle minute, read from the trace: as many polls as the rule gives, each 3 s after the answer before it", polls.length >= 10 && ruled && due, { gaps, took, polls: polls.map((p) => [p.id, ...p.rest]), end: end - last.ms })
  check("an idle minute's polls type nothing", polls.every((p) => !answers.has(p.id) || JSON.stringify(answers.get(p.id).rest) === '["retyped","0"]'), polls.map((p) => answers.get(p.id)))
  check("an idle minute's polls are not shown as work in progress", tokens() === shown, progress())
  check("an idle minute's buildInfo file keeps its time", statSync(generated).mtimeMs === before, { before, after: statSync(generated).mtimeMs })
  const quietUri = editorUri(quiet)
  const mark = notifications.length
  writeFileSync(quiet, 'object Quiet:\n  val n: Int = "one"\n')
  const written = Date.now()
  let found
  while (Date.now() < written + 20000 && !found) {
    found = notifications.slice(mark).find((n) => n.method === "textDocument/publishDiagnostics" && plain(n.params.uri) === plain(quietUri) && n.params.diagnostics.length === 1)
    await sleep(50)
  }
  check("an edit on disk without a notification: published by the next poll", found && Date.now() - written < IDLE + 2 * Math.max(...took) + 2000, { found: found?.params, ms: Date.now() - written })
  await request("shutdown", null)
  write({ jsonrpc: "2.0", method: "exit" })
}

try {
  if (idle) await idleMinute()
  else await editorChecks()
} catch (e) {
  failed++
  console.log(`FAIL ${e.message}`)
}

async function editorChecks() {
  const text = readFileSync(hello, "utf8")
  const lines = text.split("\n")
  const lineOf = (needle) => lines.findIndex((l) => l.includes(needle))
  const init = await request("initialize", { processId: null, rootUri: editorUri(workspace), capabilities: {} })
  check("initialize", init?.capabilities !== undefined, init)
  write({ jsonrpc: "2.0", method: "initialized", params: {} })
  write({ jsonrpc: "2.0", method: "textDocument/didOpen", params: { textDocument: { uri: helloUri, languageId: "scala", version: 1, text } } })
  // The definition of `greet` at its call.
  const call = lineOf('greet("you")')
  const def = await request("textDocument/definition", { textDocument: { uri: helloUri }, position: { line: call, character: lines[call].indexOf('greet("you")') } })
  const loc = Array.isArray(def) ? def[0] : def
  const declared = lineOf("def greet")
  check("the definition names the file by its Windows URI", loc && plain(loc.uri) === plain(helloUri), def)
  check("the definition's line and column", loc?.range?.start?.line === declared && loc?.range?.start?.character === lines[declared].indexOf("greet"), def)
  // Asked through another case of the file's name, which names the same file on Windows: the
  // answer names it as the file system spells it.
  const alias = helloUri.replace(/Hello\.scala$/, "hello.SCALA")
  const byAlias = await request("textDocument/definition", { textDocument: { uri: alias }, position: { line: call, character: lines[call].indexOf('greet("you")') } })
  const aliasLoc = Array.isArray(byAlias) ? byAlias[0] : byAlias
  check("a request through a case alias answers with the file's own name", aliasLoc && plain(aliasLoc.uri) === plain(helloUri) && aliasLoc.range?.start?.line === declared, byAlias)
  // The cold build's diagnostics, published for the file under its Windows URI.
  const deadline = Date.now() + 60000
  let diags
  while (Date.now() < deadline && !diags) {
    diags = notifications.find((n) => n.method === "textDocument/publishDiagnostics" && plain(n.params.uri) === plain(helloUri) && n.params.diagnostics.length > 0)
    await sleep(50)
  }
  const wrong = lineOf('"not an int"')
  check("diagnostics under the file's Windows URI", diags?.params?.diagnostics?.[0]?.range?.start?.line === wrong, diags ?? notifications.map((n) => n.params?.uri))
  check("an open document's diagnostics under the URI and the version the editor opened it with", diags?.params?.uri === helloUri && diags?.params?.version === 1, diags?.params)
  // The commit point's cancel through the server's pipe.
  let version = 1
  const change = (t) => write({ jsonrpc: "2.0", method: "textDocument/didChange", params: { textDocument: { uri: helloUri, version: ++version }, contentChanges: [{ text: t }] } })
  writeFileSync(`${barrier}.commit`, "")
  change(text.replace('"hello "', '"hi "'))
  const held = await traced((e) => e.who === "session" && e.event === "held" && e.rest[0] === "commit", "build held at its commit point")
  change(text.replace('"hello "', '"hey "'))
  await traced((e) => e.who === "server" && e.event === "wrote-cancel" && e.id === held.id, "cancel written")
  rmSync(`${barrier}.commit`)
  const ended = await traced((e) => e.who === "session" && (e.event === "cancelled" || e.event === "answered") && e.id === held.id, "the held build's end")
  check("a cancel the server writes at the commit point stops the build there", ended.event === "cancelled", traceOf().slice(-10))
  // The file deleted and closed, then made again with another greet: its text on disk is what the
  // server answers from, the closed document's gone with it (one identity for the file, whether
  // canonicalised, named by its URI or gone).
  rmSync(hello)
  write({ jsonrpc: "2.0", method: "textDocument/didClose", params: { textDocument: { uri: helloUri } } })
  await sleep(500)
  const again = text.replace('def greet(name: String): String = "hello " + name', "def greet(name: String): Int = 123").replace('val wrong: Int = "not an int"', "val wrong: Int = 123")
  writeFileSync(hello, again)
  write({ jsonrpc: "2.0", method: "workspace/didChangeWatchedFiles", params: { changes: [{ uri: helloUri, type: 1 }] } })
  const at = again.split("\n").findIndex((l) => l.includes("def greet"))
  let hover
  const until = Date.now() + 60000
  while (Date.now() < until && !JSON.stringify(hover ?? "").includes("Int")) {
    hover = await request("textDocument/hover", { textDocument: { uri: helloUri }, position: { line: at, character: again.split("\n")[at].indexOf("greet") } })
    await sleep(300)
  }
  check("a file deleted, closed and made again: the server answers from its new text", JSON.stringify(hover ?? "").includes("Int") && !JSON.stringify(hover).includes("String ="), { hover, trace: traceOf().slice(-25) })
  // The file's name in another case on disk (Hello.scala to hello.scala), as an editor reports a
  // rename: the old name deleted, the new created and opened. The answers name the file by its
  // new name.
  const lower = join(workspace, "hello.scala")
  const lowerUri = editorUri(lower)
  renameSync(hello, lower)
  write({ jsonrpc: "2.0", method: "workspace/didChangeWatchedFiles", params: { changes: [{ uri: helloUri, type: 3 }, { uri: lowerUri, type: 1 }] } })
  write({ jsonrpc: "2.0", method: "textDocument/didOpen", params: { textDocument: { uri: lowerUri, languageId: "scala", version: 1, text: again } } })
  const use = again.split("\n").findIndex((l) => l.includes('greet("you")'))
  let renamed
  const by = Date.now() + 60000
  while (Date.now() < by && !(renamed && plain(renamed.uri) === plain(lowerUri))) {
    const answer = await request("textDocument/definition", { textDocument: { uri: lowerUri }, position: { line: use, character: again.split("\n")[use].indexOf('greet("you")') } })
    renamed = Array.isArray(answer) ? answer[0] : answer
    await sleep(300)
  }
  check("a file renamed to another case: the definition names it by its new name", renamed && plain(renamed.uri) === plain(lowerUri) && renamed.range?.start?.line === at, renamed)
  // A diagnostic whose message carries non-ASCII text, an identifier not found.
  const accents = join(workspace, "Accents.scala")
  const accentsUri = editorUri(accents)
  const accentsText = 'object Accents:\n  val shown = "é ✓"\n  val n: Int = naïveCount\n'
  writeFileSync(accents, accentsText)
  write({ jsonrpc: "2.0", method: "workspace/didChangeWatchedFiles", params: { changes: [{ uri: accentsUri, type: 1 }] } })
  write({ jsonrpc: "2.0", method: "textDocument/didOpen", params: { textDocument: { uri: accentsUri, languageId: "scala", version: 1, text: accentsText } } })
  let accented
  const accentsBy = Date.now() + 60000
  while (Date.now() < accentsBy && !accented) {
    accented = notifications.find((n) => n.method === "textDocument/publishDiagnostics" && plain(n.params.uri) === plain(accentsUri) && n.params.diagnostics.length > 0)
    await sleep(50)
  }
  check("a diagnostic naming a non-ASCII identifier", accented?.params?.diagnostics?.some((d) => d.message.includes("naïveCount") && d.range.start.line === 2), accented?.params ?? notifications.map((n) => n.params?.uri))
  await request("shutdown", null)
  write({ jsonrpc: "2.0", method: "exit" })
}
const code = await Promise.race([new Promise((r) => proc.on("exit", r)), sleep(20000).then(() => "none")])
check("the server exits on exit", code === 0, { code, stderr: stderrText().slice(-400) })
if (code === "none") proc.kill()
console.log(`lsp: passed: ${passed} of ${passed + failed}`)
process.exit(failed === 0 ? 0 : 1)
