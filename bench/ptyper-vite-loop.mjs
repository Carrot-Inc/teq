#!/usr/bin/env node
// node bench/ptyper-vite-loop.mjs <build> <project> <threads|auto> <file> <out.jsonl> [<label>]: the vite plugin's dev
// session over a Scala.js project of a build's teq.lock (<build> the lock or a directory at or below which it lies,
// read by integrations/vite/description.js): `TeqWatch` running `teq --export <lock> watch <project>` with the
// arguments the plugin gives it, the binary the one it resolves (TEQ, else the lock's), the project's workers the
// count given (`--threads` after the description's settings) or the description's own (`auto`, refused when the
// description sets `threads`, which would be no automatic count), driven as `vite` drives it from its file
// watcher, without vite (whose own part does not move with the count). The loop over <file>, a file of the
// project's program with a string literal in a body: the first build, the literal changed, an unknown name in its
// place (an error), the literal back (the recovery), a definition added at the end (the full path) and taken away
// again. A line per step: its answer's `ms`, whether it was incremental and ok, the wall time from the request.
// The file is put back as it was.
import { readFileSync, writeFileSync, mkdtempSync, rmSync, appendFileSync, statSync } from "node:fs"
import { tmpdir } from "node:os"
import { join, resolve, dirname } from "node:path"
import { resolveTeq } from "../integrations/vite/binary.js"
import { loadDescription, taskArgs, watchOptions } from "../integrations/vite/description.js"
import { parse } from "../integrations/vite/lock.js"
import { TeqWatch } from "../integrations/vite/teq.js"

const [build, project, count, fileArg, outFile, label = ""] = process.argv.slice(2)
if (!outFile || !(count === "auto" || /^[1-9][0-9]*$/.test(count))) {
  console.error("usage: node bench/ptyper-vite-loop.mjs <build> <project> <threads|auto> <file> <out.jsonl> [<label>]")
  process.exit(2)
}
// The count is the session's alone: the overrides of the environment are not.
for (const name of ["TEQ_THREADS", "TEQ_SESSION_WORKERS", "TEQ_SESSION_THREADS", "TEQ_FORK"]) delete process.env[name]
const described = loadDescription(project, statSync(build).isDirectory() ? build : dirname(resolve(build)))
const threads = parse(readFileSync(described.file, "utf-8")).projects[project].description?.threads
if (count === "auto" && threads !== undefined) {
  console.error(`${described.file}: ${project}'s description sets threads: ${threads}, which no automatic count overrides`)
  process.exit(2)
}

const file = resolve(fileArg)
const original = readFileSync(file, "utf-8")
const literal = /"([^"\\\n]+)"/.exec(original)
if (!literal) throw new Error(`no string literal in ${file}`)
const edits = [
  ["edit", original.replace(literal[0], `"${literal[1]}~"`)],
  ["error", original.replace(literal[0], "ptyperUnknownName")],
  ["recovery", original],
  ["added", original + "\n\ndef ptyperAdded: Int = 1\n"],
  ["removed", original],
]
const out = mkdtempSync(join(tmpdir(), "teq-vite-loop-"))
const runner = { teq: await resolveTeq(described), cwd: described.root }
const watcher = new TeqWatch(runner, taskArgs(described, "watch", [...watchOptions(described, { out }), ...(count === "auto" ? [] : ["--threads", count])]), { onStderr: () => {} })
const record = (step, result, wall) => {
  const line = { label, count, step, ok: result.ok, incremental: result.incremental ?? false, ms: result.ms ?? {}, wall }
  appendFileSync(outFile, JSON.stringify(line) + "\n")
  return line
}
try {
  let t0 = Date.now()
  const lines = [record("first", await watcher.start(), Date.now() - t0)]
  for (const [step, text] of edits) {
    writeFileSync(file, text)
    t0 = Date.now()
    lines.push(record(step, await watcher.build([file]), Date.now() - t0))
  }
  console.log(`${label} ${count}: ` + lines.map((l) => `${l.step} ${Math.round(l.ms.type ?? 0)}${l.ok ? "" : " (error)"}${l.incremental ? "" : " full"}`).join(", "))
} finally {
  writeFileSync(file, original)
  watcher.stop()
  rmSync(out, { recursive: true, force: true })
}
