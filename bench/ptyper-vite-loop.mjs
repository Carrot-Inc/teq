#!/usr/bin/env node
// node bench/ptyper-vite-loop.mjs <build.json> <threads|auto> <file> <out.jsonl> [<label>]: the vite plugin's dev
// session over a description, the per-project `target/teq/build.json` sbt-teq wrote before teq-build.json
// (integrations/vite: `TeqWatch` running `teq compiler watch` with the arguments the plugin gave it then),
// the description's `threads` set to the count given or left out for the automatic count, driven as `vite`
// drives it from its file watcher, without vite (whose own part does not move with the count). The loop over
// <file>, a file of the description's program with a string literal in a body: the first build, the literal
// changed, an unknown name in its place (an error), the literal back (the recovery), a definition added at the
// end (the full path) and taken away again. A line per step: its answer's `ms`, whether it was incremental and
// ok, the wall time from the request. The file is put back as it was.
import { readFileSync, writeFileSync, mkdtempSync, rmSync, appendFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join, resolve, dirname } from "node:path"
import { TeqWatch } from "../integrations/vite/teq.js"

const [buildJson, count, fileArg, outFile, label = ""] = process.argv.slice(2)
// The count is the description's alone: the overrides of the environment are not the session's.
for (const name of ["TEQ_THREADS", "TEQ_SESSION_WORKERS", "TEQ_SESSION_THREADS", "TEQ_FORK"]) delete process.env[name]
const json = JSON.parse(readFileSync(buildJson, "utf-8"))
const root = resolve(dirname(resolve(buildJson)), json.root ?? ".")
const words = (flags) => (flags === undefined ? [] : Array.isArray(flags) ? flags : flags.split(/\s+/).filter(Boolean))
const watchArgs = (out) => [
  "compiler",
  "watch",
  ...(json.lib ? [json.lib] : []),
  ...(json.sources ?? []),
  ...(json.excludes ?? []).flatMap((path) => ["--exclude", path]),
  ...(json.classpath?.length ? ["--classpath", json.classpath.map((jar) => resolve(root, jar)).join(":")] : []),
  ...(json.cacheableState ?? []).flatMap((name) => ["--cacheable-state", name]),
  ...(json.macroState === "per-worker" ? ["--macro-state", "per-worker"] : []),
  ...(json.maxInlines !== undefined ? ["--max-inlines", String(json.maxInlines)] : []),
  ...(json.strictEquality ? ["--strict-equality"] : []),
  ...(json.kindProjector ? ["--kind-projector"] : []),
  ...(json.werror ? ["--werror"] : []),
  ...words(json.flags),
  ...(count === "auto" ? [] : ["--threads", count]),
  "--split",
  out,
  ...(json.modulePerFile?.length ? ["--module-per-file", json.modulePerFile.join(",")] : []),
  "--hot",
]
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
const watcher = new TeqWatch({ teq: process.env.TEQ || json.teq, cwd: root }, watchArgs(out), { onStderr: () => {} })
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
