// One resident session kept open over many edits, its memory sampled after every build:
// `node driver.mjs <name> <teq> <edits> <edit root>... -- <watch arguments>` for `teq compiler watch`,
// `... -- lsp <workspace root>` for `teq lsp` over a workspace, whose server and whose child
// (the resident check it drives) are sampled both. The edits go round a handful of files under
// the edit roots (a root may be one file), in the workload `WATCH_MEMORY_WORKLOAD` names:
//
//   body   a body edit each time: a string literal given a text no build has seen
//   tenth  the same, and every tenth edit a definition added, which the next edit of that
//          file takes away again (two full builds in ten)
//   day    what a day of work makes, twenty edits that repeat: body edits, the same text saved
//          twice, a type error and its fix, a parse error, a parse error carrying names no
//          build has seen and its fix, a definition added under a new name, its signature
//          changed, the definition removed, a file added and removed
//   dev    body edits, and every tenth edit the signature of a definition changed (one full
//          build in ten)
//   full   the signature of a definition changed with every edit: full builds alone
//   peak   no edit: the first build's peak, a fresh process's full build (no edit is made)
//
// A session that answers queries (`--index`, the language server) is asked between the builds:
// the definition, the references and the hover of a name of the edited file after every build,
// the file's symbols and the workspace's every fifth. The language server gets an edit as the
// text of an open document, which the file on disk does not follow. Under `WATCH_MEMORY_STD=1` a
// resident check is asked after those the references of the std's `List` the edited file names,
// which demand every std file, and fails when the first
// such answer holds no use in a std method's body (`List.from(suffix)` of collections.scala).
//
// The memory is the physical footprint on macOS (the kernel's ledger, compressed pages
// included, as `footprint`, `top` and Activity Monitor show it) and the resident size plus swap
// of `smaps_rollup` on Linux; the peak is the ledger's lifetime maximum and `VmHWM`. The language
// server's own process is measured on Linux by its anonymous memory plus swap (`Anonymous` and
// `Swap` of `smaps_rollup`): it types nothing, and the rest of its resident size is the binary's
// file-backed pages, which follow the file's page-cache state and not the revision (its child, a
// resident check, keeps the resident size). A line names the metric its memory is in (`metric`),
// which its budget's row has to name. Beside it
// the session's own account (`stats`) every `WATCH_MEMORY_STATS_EVERY` edits (default 10, 0 for
// a binary without the command). Prints one line of name and value pairs per process; exits
// non-zero when the session fails to answer or a build answers otherwise than its edit has to
// (an error edit without an error, a fix with one). `WATCH_MEMORY_SERIES=<file>` keeps every
// sample, `WATCH_MEMORY_PHASES=1` prints the median of every phase of the incremental builds,
// `WATCH_MEMORY_AT_END=<command>` runs a command on the session before it ends, its process id
// appended (`vmmap -summary`). Under `TEQ_SESSION_WORKERS_LOG` (with `TEQ_SESSION_WORKERS=<n>`,
// or `TEQ_FORK=1` for one worker through the fork) every full build the session's processes
// logged has to say that its `n` workers joined (`workersJoined`); the line then says `workers
// <n> joined <builds>`.
import { spawn, spawnSync } from "node:child_process"
import { readdirSync, readFileSync, writeFileSync, rmSync, mkdtempSync } from "node:fs"
import { dirname, join } from "node:path"
import { tmpdir } from "node:os"
import { fileURLToPath, pathToFileURL } from "node:url"

const [name, teq, editsText, ...rest] = process.argv.slice(2)
const split = rest.indexOf("--")
const roots = rest.slice(0, split)
const watchArgs = rest.slice(split + 1)
const edits = Number(editsText)
const WORKLOAD = process.env.WATCH_MEMORY_WORKLOAD ?? "day"
const STATS_EVERY = Number(process.env.WATCH_MEMORY_STATS_EVERY ?? 10)
const WARM = Math.min(20, Math.floor(edits / 4))
const FILES = 8
// A string literal that an edit may give another text: the first of a file whose text
// `WATCH_MEMORY_LITERAL` matches (a regular expression; any text without it), for sources
// where a macro reads some literals and refuses what an edit would put there.
const LITERALS = /(^|[^A-Za-z0-9_"\\])"([^"\\\n$]*)"(?!")/g
const WANTED = new RegExp(process.env.WATCH_MEMORY_LITERAL ?? "")
function literal(text) {
  for (const found of text.matchAll(LITERALS)) if (WANTED.test(found[2])) return found
}
function withLiteral(text, put) {
  const found = literal(text)
  return text.slice(0, found.index) + found[1] + put + text.slice(found.index + found[0].length)
}
const scratch = mkdtempSync(join(tmpdir(), "watch-memory-"))

function scalaFiles(dir) {
  if (dir.endsWith(".scala")) return [dir]
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name)
    if (entry.isDirectory()) return scalaFiles(path)
    return entry.name.endsWith(".scala") ? [path] : []
  })
}

function fail(message) {
  console.error(`FAIL ${name}: ${message}`)
  process.exitCode = 1
  throw new Error(message)
}

const candidates = roots.flatMap(scalaFiles).sort().filter((file) => literal(readFileSync(file, "utf-8")))
if (candidates.length === 0) fail("no file with a string literal under the edit roots")
const step = Math.max(1, Math.floor(candidates.length / FILES))
const files = candidates.filter((_, i) => i % step === 0).slice(0, FILES).map((path) => ({ path, original: readFileSync(path, "utf-8") }))
const addedFile = join(roots[0].endsWith(".scala") ? dirname(roots[0]) : roots[0], "WatchMemoryAdded.scala")

const body = (file, i) => withLiteral(file.original, `"edit ${i}"`)
const definition = (i) => `\ndef watchMemoryAdded${i}(x: Int): Int = x + ${i}\n`

// The edit of a step: the file written (or removed, without a text) and whether the build has
// to succeed.
function bodyEdit(i) {
  const file = files[i % files.length]
  return { path: file.path, text: body(file, i), ok: true }
}

function tenthEdit(i) {
  const edit = bodyEdit(i)
  if (i % 10 === 0) edit.text += definition(i)
  return edit
}

// The file a run of steps of the day works on, and the definition it added there.
let day = { file: files[0], added: 0 }
function dayEdit(i) {
  const at = (i - 1) % 20
  if (at === 0) day = { file: files[Math.floor(i / 20) % files.length], added: i }
  const { file, added } = day
  const other = files[(files.indexOf(file) + 1) % files.length]
  switch (at) {
    case 6:
      return { path: file.path, text: body(file, i - 1), ok: true }
    case 7:
      return { path: file.path, text: withLiteral(file.original, `watchMemoryUndefined${i}("edit ${i}")`), ok: false }
    case 10:
      return { path: file.path, text: body(file, i) + `\ndef watchMemoryBroken(x: Int = (\n`, ok: false }
    case 11:
      return { path: file.path, text: body(file, i) + `\ndef watchMemoryBroken${i}(fresh${i}: WatchMemoryFresh${i} = (\n`, ok: false }
    case 13:
      return { path: file.path, text: body(file, i) + definition(added), ok: true }
    case 14:
      return { path: file.path, text: body(file, i) + `\ndef watchMemoryAdded${added}(x: Int, y: Int = ${i}): Int = x + y\n`, ok: true }
    case 15:
      return { path: other.path, text: body(other, i), ok: true }
    case 16:
      return { path: file.path, text: body(file, i), ok: true }
    case 17:
      return { path: addedFile, text: `package watchmemory\n\nobject WatchMemoryAdded${i}:\n  def value: Int = ${i}\n`, ok: true }
    case 18:
      return { path: other.path, text: body(other, i), ok: true }
    case 19:
      return { path: addedFile, ok: true }
    default:
      return { path: file.path, text: body(file, i), ok: true }
  }
}

// A definition that every file has from its first edit on, its signature changing with
// `turn`, which names its second parameter.
const signed = (file, i, turn) => body(file, i) + `\ndef watchMemorySigned${files.indexOf(file)}(x: Int, y${turn}: Int): Int = x + y${turn}\n`
const turns = new Map()
function signatureEdit(i, every) {
  const file = files[i % files.length]
  if (i % every === 0) turns.set(file.path, i)
  return { path: file.path, text: signed(file, i, turns.get(file.path) ?? 0), ok: true }
}
// The session starts from files that have the definition.
if (WORKLOAD === "dev" || WORKLOAD === "full") for (const file of files) writeFileSync(file.path, signed(file, 0, 0))

const editOf = { body: bodyEdit, tenth: tenthEdit, day: dayEdit, dev: (i) => signatureEdit(i, 10), full: (i) => signatureEdit(i, 1), peak: bodyEdit }[WORKLOAD]
if (!editOf) fail(`no workload ${WORKLOAD}`)

// Where a query of an edit points: a name that something is selected from.
function queried(edit) {
  const at = /\b[a-z][A-Za-z0-9_]*(?=\.[a-z])/.exec(edit.text ?? "")
  return at ? at.index : undefined
}

// Where a std query of an edit points (`WATCH_MEMORY_STD`): an applied `List`.
const STD = process.env.WATCH_MEMORY_STD === "1"
function stdQueried(edit) {
  const at = /\bList(?=\()/.exec(edit.text ?? "")
  return at ? at.index : undefined
}

// Whether locations hold a use in the body of a std method, which only a demand records.
function inStdBody(locations) {
  const line = (l) => readFileSync(fileURLToPath(l.uri), "utf-8").split("\n")[l.range.start.line] ?? ""
  return (locations ?? []).some((l) => /\/attached\/std-[^/]+\/collections\.scala$/.test(fileURLToPath(l.uri)) && line(l).includes("List.from(suffix)"))
}

function within(seconds, promise, what) {
  let timer
  const timeout = new Promise((_, reject) => (timer = setTimeout(() => reject(new Error(`${what}: no answer in ${seconds} s`)), seconds * 1000)))
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer))
}

// `teq compiler watch` over its lines.
class Watch {
  constructor() {
    this.child = spawn(teq, ["compiler", "watch", ...watchArgs], { stdio: ["pipe", "pipe", "pipe"] })
    this.stderr = ""
    this.child.stderr.on("data", (data) => (this.stderr = (this.stderr + data).slice(-2000)))
    // The session's answers, a JSON line each, handed to whoever waits for the next one.
    this.lines = []
    this.waiting = []
    this.ended = false
    let pending = ""
    this.child.stdout.setEncoding("utf-8")
    this.child.stdout.on("data", (data) => {
      pending += data
      let at
      while ((at = pending.indexOf("\n")) >= 0) {
        this.lines.push(pending.slice(0, at))
        pending = pending.slice(at + 1)
      }
      while (this.lines.length > 0 && this.waiting.length > 0) this.waiting.shift()(this.lines.shift())
    })
    this.child.on("exit", () => {
      this.ended = true
      while (this.waiting.length > 0) this.waiting.shift()(undefined)
    })
    this.answers = watchArgs.includes("--index")
    this.accounts = STATS_EVERY > 0
  }
  async answer(seconds) {
    const next = this.lines.length > 0 ? Promise.resolve(this.lines.shift()) : this.ended ? Promise.resolve(undefined) : new Promise((resolve) => this.waiting.push(resolve))
    const line = await within(seconds, next, "the session")
    if (line === undefined) throw new Error(`the session ended: ${this.stderr.trim().slice(-300)}`)
    return JSON.parse(line)
  }
  first() {
    return this.answer(180)
  }
  async build(edit) {
    if (edit.text === undefined) rmSync(edit.path, { force: true })
    else writeFileSync(edit.path, edit.text)
    this.child.stdin.write(`build ${edit.path}\n\n`)
    const built = await this.answer(120)
    return { ok: built.ok, incremental: built.incremental, ms: built.ms?.total ?? 0, phases: built.ms, errors: built.errors }
  }
  async ask(edit, i) {
    const at = queried(edit)
    const lines = at === undefined ? [] : [`definition ${edit.path} ${at}`, `references ${edit.path} ${at} 1`, `hover ${edit.path} ${at}`]
    if (i % 5 === 0 && edit.text !== undefined) lines.push(`symbols ${edit.path}`, "workspace-symbols watchMemory", "index-stats")
    for (const line of lines) {
      this.child.stdin.write(line + "\n")
      await this.answer(60)
    }
    const std = STD ? stdQueried(edit) : undefined
    if (std === undefined) return lines.length
    this.child.stdin.write(`references ${edit.path} ${std} 0\n`)
    const answer = await this.answer(60)
    if (!this.stdDemanded) {
      if (!inStdBody(answer.result)) fail(`the references of the std's List hold no use in a std body: ${JSON.stringify(answer).slice(0, 300)}`)
      this.stdDemanded = true
    }
    return lines.length + 1
  }
  async stats() {
    this.child.stdin.write("stats\n")
    return (await this.answer(60)).stats
  }
  processes() {
    return [{ name, pid: this.child.pid, metric: RETAINED }]
  }
  stop() {
    if (this.stopping) return
    this.stopping = true
    this.child.stdin.write("quit\n")
    this.child.stdin.end()
    setTimeout(() => this.child.kill(), 2000).unref()
  }
  // Ends the session and waits for its process to end, bounded: what it logs as it ends is read
  // afterwards.
  async stopAndWait() {
    const gone = new Promise((resolve) => (this.ended ? resolve() : this.child.on("exit", resolve)))
    this.stop()
    await within(10, gone, "the session's end").catch(() => {})
  }
}

// `teq lsp` over its messages, the edited files open as documents.
class Server {
  constructor() {
    this.root = watchArgs[1]
    this.child = spawn(teq, ["lsp"], { cwd: this.root, stdio: ["pipe", "pipe", "pipe"] })
    this.stderr = ""
    this.child.stderr.on("data", (data) => (this.stderr = (this.stderr + data).slice(-2000)))
    this.buffer = Buffer.alloc(0)
    this.nextId = 1
    this.waiting = new Map()
    this.versions = new Map()
    // The diagnostics last published per document.
    this.published = new Map()
    this.child.stdout.on("data", (data) => {
      this.buffer = Buffer.concat([this.buffer, data])
      for (;;) {
        const end = this.buffer.indexOf("\r\n\r\n")
        if (end < 0) return
        const length = Number(/Content-Length: *(\d+)/i.exec(this.buffer.subarray(0, end).toString())?.[1])
        if (this.buffer.length < end + 4 + length) return
        const message = JSON.parse(this.buffer.subarray(end + 4, end + 4 + length).toString())
        this.buffer = this.buffer.subarray(end + 4 + length)
        if (message.method === "textDocument/publishDiagnostics") this.published.set(message.params.uri, message.params.diagnostics)
        else if (message.id !== undefined && message.method !== undefined) this.write({ jsonrpc: "2.0", id: message.id, result: null })
        else if (message.id !== undefined) this.waiting.get(message.id)?.(message)
      }
    })
    this.child.on("exit", () => {
      for (const resolve of this.waiting.values()) resolve({ gone: true })
    })
    this.answers = true
    this.accounts = false
  }
  write(message) {
    const text = Buffer.from(JSON.stringify(message))
    this.child.stdin.write(Buffer.concat([Buffer.from(`Content-Length: ${text.length}\r\n\r\n`), text]))
  }
  async request(method, params, seconds = 120) {
    const id = this.nextId++
    const answer = new Promise((resolve) => this.waiting.set(id, resolve))
    this.write({ jsonrpc: "2.0", id, method, params })
    const message = await within(seconds, answer, method)
    this.waiting.delete(id)
    if (message.gone) throw new Error(`the server ended: ${this.stderr.trim().slice(-300)}`)
    if (message.error) throw new Error(`${method}: ${JSON.stringify(message.error)}`)
    return message.result
  }
  notify(method, params) {
    this.write({ jsonrpc: "2.0", method, params })
  }
  uri(path) {
    return pathToFileURL(path).href
  }
  // A request is answered by the program of the build before it: its answer says the build is done.
  settled(path) {
    return this.request("textDocument/documentSymbol", { textDocument: { uri: this.uri(path) } })
  }
  failed() {
    return [...this.published.values()].some((diagnostics) => diagnostics.some((d) => d.severity === 1))
  }
  async first() {
    await this.request("initialize", { processId: null, rootUri: this.uri(this.root), capabilities: { general: { positionEncodings: ["utf-16"] }, workspace: { didChangeWatchedFiles: { dynamicRegistration: true } } } }, 180)
    this.notify("initialized", {})
    for (const file of files) this.change(file.path, file.original)
    await this.settled(files[0].path)
    return { ok: !this.failed(), ms: { total: 0 }, errors: [...this.published.entries()].filter(([, d]) => d.length > 0) }
  }
  change(path, text) {
    const version = (this.versions.get(path) ?? 0) + 1
    this.versions.set(path, version)
    if (version === 1) this.notify("textDocument/didOpen", { textDocument: { uri: this.uri(path), languageId: "scala", version, text } })
    else this.notify("textDocument/didChange", { textDocument: { uri: this.uri(path), version }, contentChanges: [{ text }] })
  }
  async build(edit) {
    const started = Date.now()
    if (this.versions.has(edit.path)) {
      this.change(edit.path, edit.text)
    } else {
      // A file of the disk that comes and goes.
      if (edit.text === undefined) rmSync(edit.path, { force: true })
      else writeFileSync(edit.path, edit.text)
      this.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: this.uri(edit.path), type: edit.text === undefined ? 3 : 1 }] })
    }
    await this.settled(files[0].path)
    return { ok: !this.failed(), incremental: false, ms: Date.now() - started, errors: [...this.published.entries()].filter(([, d]) => d.length > 0) }
  }
  async ask(edit, i) {
    const at = queried(edit)
    if (at === undefined || !this.versions.has(edit.path)) return 0
    const before = edit.text.slice(0, at)
    const line = before.split("\n").length - 1
    const position = { line, character: at - (before.lastIndexOf("\n") + 1) }
    const textDocument = { uri: this.uri(edit.path) }
    await this.request("textDocument/definition", { textDocument, position })
    await this.request("textDocument/references", { textDocument, position, context: { includeDeclaration: true } })
    await this.request("textDocument/hover", { textDocument, position })
    if (i % 5 !== 0) return 3
    await this.request("workspace/symbol", { query: "watchMemory" })
    return 4
  }
  processes() {
    const children = spawnSync("pgrep", ["-P", String(this.child.pid)], { encoding: "utf-8" }).stdout.split("\n").filter(Boolean)
    return [{ name, pid: this.child.pid, metric: SERVER_RETAINED }, ...children.map((pid) => ({ name: `${name}-child`, pid: Number(pid), metric: RETAINED }))]
  }
  stop() {
    if (this.stopping) return
    this.stopping = true
    this.request("shutdown", null, 5)
      .catch(() => {})
      .then(() => this.notify("exit", null))
    setTimeout(() => this.child.kill(), 4000).unref()
  }
  // The server's shutdown asks its children to quit; they are waited for before the exit, which
  // would kill the ones still there, and the server after it, each bounded.
  async stopAndWait() {
    if (this.stopping) return
    this.stopping = true
    const children = this.processes().slice(1).map((p) => p.pid)
    const gone = new Promise((resolve) => this.child.on("exit", resolve))
    await this.request("shutdown", null, 5).catch(() => {})
    const alive = (pid) => {
      try {
        process.kill(pid, 0)
        return true
      } catch {
        return false
      }
    }
    for (let i = 0; i < 100 && children.some(alive); i++) await new Promise((resolve) => setTimeout(resolve, 100))
    this.notify("exit", null)
    await within(10, gone, "the server's end").catch(() => {})
  }
}

// What `peak` is on this platform: the kernel's lifetime maximum of the footprint, or the
// resident size's high-water mark.
const PEAK_METRIC = process.platform === "darwin" ? "phys_footprint_peak" : "VmHWM"
// What a process's memory is: the footprint, or the resident size plus swap; the language
// server's own process, the footprint, or its anonymous memory plus swap.
const RETAINED = process.platform === "darwin" ? "phys_footprint" : "Rss+Swap"
const SERVER_RETAINED = process.platform === "darwin" ? "phys_footprint" : "Anonymous+Swap"

// The workers' log checked against the session's full builds once the session has ended: what
// the line adds, or a failure. A line is `<pid> <parent's pid> <k> <how>` for a process's k-th
// full build and `<pid> <parent's pid> end <full builds>` as it ends: a watch session's own lines
// (as many full builds as it answered), the language server's its children's; every process's
// builds logged in order from its first and as many as its own count, each typed by the n workers
// joined; at the automatic count (`WATCH_MEMORY_WORKERS=auto`) by the one count of two or more the
// session chose for every full build, which the line reports. A line of any other process fails the
// session.
function workersJoined(session, fullBuilds) {
  const log = process.env.TEQ_SESSION_WORKERS_LOG
  if (!log) return ""
  const auto = process.env.WATCH_MEMORY_WORKERS === "auto"
  let wanted = Number(process.env.TEQ_SESSION_WORKERS ?? (process.env.TEQ_FORK === "1" ? 1 : 0))
  let lines = []
  try {
    lines = readFileSync(log, "utf-8").split("\n").filter(Boolean)
  } catch {}
  const rows = lines.map((line) => {
    const [pid, parent, k, ...how] = line.split(" ")
    return { line, pid: Number(pid), parent: Number(parent), end: k === "end", k: Number(k === "end" ? how[0] : k), how: how.join(" ") }
  })
  const watch = session instanceof Watch
  const own = (r) => (watch ? r.pid === session.child.pid : r.parent === session.child.pid)
  const foreign = rows.find((r) => !own(r))
  if (foreign) fail(`a line of another process: ${foreign.line}`)
  const builds = rows.filter((r) => !r.end)
  if (auto) {
    const counts = new Set(builds.map((r) => /^joined (\d+)$/.exec(r.how)?.[1]))
    wanted = counts.size === 1 ? Number([...counts][0]) : NaN
    if (!(wanted >= 2)) fail(`the full builds were not typed by one automatic count of two workers or more: ${builds.map((r) => r.how).join(", ")}`)
  }
  const other = builds.find((r) => r.how !== `joined ${wanted}`)
  if (other) fail(`a full build was not typed by ${wanted} workers joined: ${other.line}`)
  const logged = new Map()
  for (const r of builds) {
    if (r.k !== (logged.get(r.pid) ?? 0)) fail(`process ${r.pid} logged its full build ${r.k} where ${logged.get(r.pid) ?? 0} was next`)
    logged.set(r.pid, r.k + 1)
  }
  const ends = new Map(rows.filter((r) => r.end).map((r) => [r.pid, r.k]))
  for (const pid of new Set(rows.map((r) => r.pid))) {
    if (!ends.has(pid)) fail(`process ${pid} logged no end, so its count of full builds is not known`)
    if (ends.get(pid) !== (logged.get(pid) ?? 0)) fail(`process ${pid} made ${ends.get(pid)} full builds and logged ${logged.get(pid) ?? 0}`)
  }
  if (builds.length === 0) fail("no full build logged its workers")
  if (watch && builds.length !== fullBuilds) fail(`${builds.length} full builds logged their workers, the session answered ${fullBuilds}`)
  return ` workers ${wanted} joined ${builds.length}`
}

// A process's memory in a metric and its peak so far, in KB.
function memory(pid, metric = RETAINED) {
  if (process.platform === "darwin") {
    const json = join(scratch, "footprint.json")
    spawnSync("footprint", ["-f", "bytes", "-j", json, String(pid)], { encoding: "utf-8" })
    let ledger
    try {
      ledger = JSON.parse(readFileSync(json, "utf-8")).processes[0].auxiliary
    } catch {
      fail(`process ${pid} of the session is gone`)
    }
    return { kb: ledger.phys_footprint / 1024, peak: ledger.phys_footprint_peak / 1024 }
  }
  const field = (text, key) => Number(new RegExp(`^${key}:\\s+(\\d+) kB`, "m").exec(text)?.[1] ?? 0)
  let rollup, status
  try {
    rollup = readFileSync(`/proc/${pid}/smaps_rollup`, "utf-8")
    status = readFileSync(`/proc/${pid}/status`, "utf-8")
  } catch {
    fail(`process ${pid} of the session is gone`)
  }
  const resident = metric === "Anonymous+Swap" ? field(rollup, "Anonymous") : field(rollup, "Rss")
  return { kb: resident + field(rollup, "Swap"), peak: field(status, "VmHWM") }
}

function residentKb(pid) {
  return Number(spawnSync("ps", ["-o", "rss=", "-p", String(pid)], { encoding: "utf-8" }).stdout.trim() || 0)
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.floor(sorted.length / 2)]
}

// Least squares over (edit, memory): the growth per edit once the session is warm.
function slope(samples) {
  const n = samples.length
  const mx = samples.reduce((s, [x]) => s + x, 0) / n
  const my = samples.reduce((s, [, y]) => s + y, 0) / n
  const sxy = samples.reduce((s, [x, y]) => s + (x - mx) * (y - my), 0)
  const sxx = samples.reduce((s, [x]) => s + (x - mx) * (x - mx), 0)
  return sxx === 0 ? 0 : sxy / sxx
}

const session = watchArgs[0] === "lsp" ? new Server() : new Watch()
try {
  const first = await session.first()
  if (!first.ok) fail(`the first build failed: ${JSON.stringify(first.errors ?? first).slice(0, 300)}`)
  const processes = session.processes().map((p) => ({ ...p, first: memory(p.pid, p.metric).kb, samples: [], peak: 0 }))
  if (WORKLOAD === "peak") {
    const mb = (kb) => (kb / 1024).toFixed(1)
    const out = processes.map((p) => {
      const sample = memory(p.pid)
      return `${p.name} workload peak edits 0 metric ${PEAK_METRIC} memory_first_mb ${mb(sample.kb)} peak_mb ${mb(sample.peak)} first_build_ms ${(first.ms?.total ?? 0).toFixed(1)}`
    })
    await session.stopAndWait()
    const joined = workersJoined(session, 1)
    for (const line of out) console.log(line + joined)
    throw { done: true }
  }
  const times = []
  // The phases of the incremental builds, for `WATCH_MEMORY_PHASES`.
  const phases = []
  let incremental = 0
  let failed = 0
  // The full builds the session answered, the first included.
  let fullBuilds = 1
  let queries = 0
  let held
  for (let i = 1; i <= edits; i++) {
    const edit = editOf(i)
    const built = await session.build(edit)
    if (built.ok !== edit.ok) fail(`edit ${i} of ${edit.path} answered ok ${built.ok}: ${JSON.stringify(built.errors ?? built).slice(0, 300)}`)
    if (built.incremental) incremental++
    if (!built.ok) failed++
    if (!built.incremental) fullBuilds++
    times.push(built.ms)
    if (built.incremental && built.ok && built.phases) phases.push(built.phases)
    if (session.answers) queries += await session.ask(edit, i)
    const account = session.accounts && (i === edits || i % STATS_EVERY === 0) ? await session.stats() : undefined
    held = account ?? held
    for (const p of processes) {
      const sample = memory(p.pid, p.metric)
      // The kernel keeps no high-water mark of the anonymous memory: the samples' largest.
      p.peak = p.metric === "Anonymous+Swap" ? Math.max(p.peak, p.first, sample.kb) : sample.peak
      p.samples.push([i, sample.kb, p === processes[0] ? account : undefined])
    }
  }
  if (process.env.WATCH_MEMORY_AT_END) {
    const [command, ...args] = process.env.WATCH_MEMORY_AT_END.split(" ")
    console.error(spawnSync(command, [...args, String(processes.at(-1).pid)], { encoding: "utf-8" }).stdout)
  }
  if (process.env.WATCH_MEMORY_SERIES) {
    const series = processes[0].samples.map(([i, kb, account], at) => `${i} ${[kb, ...processes.slice(1).map((p) => p.samples[at][1])].join(" ")}${account ? " " + JSON.stringify(account) : ""}\n`)
    writeFileSync(process.env.WATCH_MEMORY_SERIES, series.join(""))
  }
  if (process.env.WATCH_MEMORY_PHASES && phases.length > 0) {
    console.error(["read", "parse", "type", "reach", "emit", "write", "total"].map((phase) => `${phase} ${median(phases.map((p) => p[phase])).toFixed(2)}`).join(" "))
  }
  const mb = (kb) => (kb / 1024).toFixed(1)
  const allocator = held?.allocator ?? { reserved: 0, live: 0, large: 0 }
  const out = []
  for (const p of processes) {
    const warm = p.samples.filter(([i]) => i >= WARM)
    const hundredth = p.samples.length >= 100 ? ` memory_100_mb ${mb(p.samples[99][1])}` : ""
    const account = p === processes[0] ? ` reserved_mb ${mb(allocator.reserved / 1024)} live_mb ${mb(allocator.live / 1024)} large_mb ${mb(allocator.large / 1024)}` : ""
    out.push(
      `${p.name} workload ${WORKLOAD} metric ${p.metric} edits ${edits} incremental ${incremental} full ${edits - incremental} failed ${failed} queries ${queries} ` +
        `memory_first_mb ${mb(p.first)} memory_warm_mb ${mb(warm[0][1])}${hundredth} memory_end_mb ${mb(p.samples.at(-1)[1])} peak_mb ${mb(p.peak)} ` +
        `growth_kb_per_edit ${slope(warm).toFixed(1)}${account} rss_end_mb ${mb(residentKb(p.pid))} ` +
        `first_build_ms ${(first.ms?.total ?? 0).toFixed(1)} build_ms_median ${median(times).toFixed(1)} build_ms_max ${Math.max(...times).toFixed(1)}`,
    )
  }
  await session.stopAndWait()
  const joined = workersJoined(session, fullBuilds)
  for (const line of out) console.log(line + joined)
} catch (error) {
  if (error?.done) {
  } else if (!process.exitCode) {
    console.error(`FAIL ${name}: ${error.message}`)
    process.exitCode = 1
  }
} finally {
  for (const file of files) writeFileSync(file.path, file.original)
  rmSync(addedFile, { force: true })
  rmSync(scratch, { recursive: true, force: true })
  session.stop()
}
