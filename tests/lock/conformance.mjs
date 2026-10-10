// node tests/lock/conformance.mjs: the YAML 1.2 core-schema reader (the `yaml` package this
// directory pins) and vite-plugin-teq's reader (integrations/vite/lock.js) read the corpus to the
// tree corpus.json holds and the example's lock to one tree; of refused.txt's documents, lock.js
// refuses the ones the core schema refuses or reads as no mapping, each on its line, and reads
// the others to the core schema's tree. A line per check, FAIL for a failure, exit 1 on any.
import { readFileSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { parse as yaml } from "yaml"
import { parse } from "../../integrations/vite/lock.js"

const here = dirname(fileURLToPath(import.meta.url))
const read = (p) => readFileSync(join(here, p), "utf-8")
let failed = 0
const check = (ok, what, found) => {
  if (ok) console.log(`lock: ${what}`)
  else {
    console.error(`FAIL lock: ${what}${found === undefined ? "" : `: ${found}`}`)
    failed++
  }
}
/** A tree with its mappings' keys sorted, for a comparison that order does not decide. */
const canon = (v) => (Array.isArray(v) ? v.map(canon) : v !== null && typeof v === "object" ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])])) : v)
const same = (a, b) => JSON.stringify(canon(a)) === JSON.stringify(canon(b))
const core = (text) => yaml(text, { version: "1.2", schema: "core", uniqueKeys: true, strict: true })

const corpus = read("corpus.lock")
const tree = JSON.parse(read("corpus.json"))
check(same(core(corpus), tree), "yaml 2.9.1 (YAML 1.2, core schema) reads the corpus to corpus.json's tree")
check(same(parse(corpus), tree), "lock.js reads the corpus to corpus.json's tree")
const example = readFileSync(join(here, "../../integrations/sbt/example/teq.lock"), "utf-8")
check(same(core(example), parse(example)), "yaml and lock.js read the example's lock to one tree")

const isMapping = (v) => v !== null && typeof v === "object" && !Array.isArray(v)
const attempt = (run) => {
  try {
    return { tree: run() }
  } catch (err) {
    return { message: err.message }
  }
}
const cases = read("refused.txt").split(/^=== /m).slice(1)
let refuses = 0
let reads = 0
for (const c of cases) {
  const [head, ...rest] = c.split("\n")
  const [line, ...why] = head.split(" ")
  const text = rest.join("\n")
  const oracle = attempt(() => core(text))
  const found = attempt(() => parse(text))
  if (isMapping(oracle.tree)) {
    if (found.tree !== undefined && same(found.tree, oracle.tree)) reads++
    else check(false, `lock.js reads ${why.join(" ")} to the core schema's tree`, found.message ?? "another tree")
  } else if (found.message?.startsWith(`line ${line}: `)) refuses++
  else check(false, `lock.js refuses ${why.join(" ")} on line ${line}`, found.message ?? "it read")
}
check(refuses + reads === cases.length, `lock.js refuses the ${refuses} documents of refused.txt the core schema refuses or reads as no mapping, each on its line, and reads the other ${reads} to its tree`)
console.log(failed ? `lock: ${failed} failed` : "lock: all passed")
process.exit(failed ? 1 : 0)
