import { spawn } from "node:child_process"

/**
 * One build, `teq <args>` run from `cwd` with the binary `teq`. Resolves to `{ ok: true, stderr }`,
 * teq's timing line on stderr, or `{ ok: false, stderr, diagnostics }`.
 */
export async function build(runner, args) {
  const child = spawnTeq(runner, args, ["ignore", "inherit", "pipe"])
  let stderr = ""
  child.stderr.setEncoding("utf-8")
  child.stderr.on("data", (data) => (stderr += data))
  const code = await new Promise((resolve, reject) => {
    child.on("error", (err) => reject(startError(runner, err)))
    child.on("close", resolve)
  })
  if (code !== 0) return { ok: false, stderr, diagnostics: parseDiagnostics(stderr) }
  return { ok: true, stderr }
}

function spawnTeq({ teq, cwd }, args, stdio) {
  return spawn(teq, args, { cwd, stdio })
}

function startError({ teq }, err) {
  return new Error(`teq could not be started as ${teq} (TEQ overrides it).\n${err}`)
}

const DIAGNOSTIC = /^(.+?):(\d+):(\d+): (error|warning): (.*)$/
const SUMMARY = /^\d+ (errors?|warnings?)\b/

/** `file:line:col: error: message`, followed by the source line and the caret line under it. */
/** Whether a session's stderr is teq's note that a full build's parallel attempt gave way to one worker,
 * which is information, not an error: the build goes on. */
export function isGiveWayNote(text) {
  return text.startsWith("teq: a parallel attempt gave way")
}

export function parseDiagnostics(stderr) {
  const diagnostics = []
  for (const line of stderr.split("\n")) {
    const header = DIAGNOSTIC.exec(line)
    if (header) {
      const [, file, row, column, severity, message] = header
      diagnostics.push({ file, line: Number(row), column: Number(column), severity, message, lines: [] })
    } else if (diagnostics.length && line.trim() && !SUMMARY.test(line)) {
      diagnostics.at(-1).lines.push(line)
    }
  }
  return diagnostics
}

/**
 * A resident `teq watch`, run as `teq <args>` from `cwd` (`teq watch <project>` with the
 * options of its output, for the plugin). It builds once when started and again for every
 * `build(paths)`, typing only the bodies of the changed files when nothing else changed. Results
 * come as teq's JSON line, with `diagnostics` and `warnings` in the shape `parseDiagnostics`
 * gives, and `restarted` set when the process had to be started anew (its first build then
 * covers every change). Builds are serialised; `onStderr` receives what teq prints besides, a
 * line at a time.
 */
export class TeqWatch {
  constructor(runner, args, { onStderr = () => {} } = {}) {
    this.runner = runner
    this.args = args
    this.onStderr = onStderr
    this.child = undefined
    this.pending = undefined
    this.queue = Promise.resolve()
  }

  /** Starts the process and resolves to the result of its first build. */
  start() {
    return this.enqueue(() => this.spawn())
  }

  /** Rebuilds after the given files changed; without a list teq checks every file's mtime. */
  build(paths = []) {
    return this.enqueue(async () => {
      if (!this.child) return { ...(await this.spawn()), restarted: true }
      return this.request(paths.length ? `build ${paths.join("\n")}\n\n` : "build\n")
    })
  }

  stop() {
    if (this.child) {
      this.child.stdin.end("quit\n")
      this.child = undefined
    }
  }

  enqueue(task) {
    const run = this.queue.then(task, task)
    this.queue = run.catch(() => {})
    return run
  }

  spawn() {
    const child = spawnTeq(this.runner, this.args, ["pipe", "pipe", "pipe"])
    this.child = child
    let buffered = ""
    child.stdout.setEncoding("utf-8")
    child.stdout.on("data", (data) => {
      buffered += data
      let newline
      while ((newline = buffered.indexOf("\n")) >= 0) {
        const line = buffered.slice(0, newline)
        buffered = buffered.slice(newline + 1)
        if (line.trim()) this.settle(JSON.parse(line))
      }
    })
    child.stderr.setEncoding("utf-8")
    let partial = ""
    child.stderr.on("data", (data) => {
      const lines = (partial + data).split("\n")
      partial = lines.pop()
      for (const line of lines) this.onStderr(line + "\n")
    })
    child.stderr.on("end", () => {
      if (partial) this.onStderr(partial)
      partial = ""
    })
    // a write to a process that has exited: `close` reports it
    child.stdin.on("error", () => {})
    child.on("error", (err) => this.fail(startError(this.runner, err)))
    child.on("close", (code) => {
      if (this.child === child) this.child = undefined
      this.fail(new Error(`teq watch exited with code ${code}`))
    })
    return this.request(undefined)
  }

  request(command) {
    return new Promise((resolve, reject) => {
      this.pending = { resolve, reject }
      if (command !== undefined) this.child.stdin.write(command)
    })
  }

  settle(result) {
    const pending = this.pending
    this.pending = undefined
    if (result.errors) result.diagnostics = result.errors.map((d) => diagnosticOf(d, "error"))
    result.warnings = (result.warnings ?? []).map((d) => diagnosticOf(d, "warning"))
    pending?.resolve(result)
  }

  fail(err) {
    const pending = this.pending
    this.pending = undefined
    pending?.reject(err)
  }
}

function diagnosticOf({ file, line, col, message, source, caret }, severity) {
  const lines = source === undefined ? [] : [`  ${source}`, `  ${caret}`]
  return { file: file ?? "", line: line ?? 1, column: col ?? 1, severity, message, lines }
}
