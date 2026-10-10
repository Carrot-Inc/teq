/**
 * A reader of `teq.lock` (docs/TARGETS.md, "The export and the project verbs"): YAML 1.2 under the
 * core schema, read by the `yaml` package, a document that is no mapping refused with its line; the
 * header from the first two lines alone, as the launchers read them.
 */
import { parseDocument, visit } from "yaml"

export const FILE = "teq.lock"
/** The format this reader reads, the lock's second line. */
export const FORMAT = 1
const YAML_1_2_CORE = { version: "1.2", schema: "core", uniqueKeys: true, strict: true, prettyErrors: false }

export class LockError extends Error {}

const fail = (line, message) => {
  throw new LockError(`line ${line}: ${message}`)
}
const isMapping = (v) => v !== null && typeof v === "object" && !Array.isArray(v)
const lineAt = (text, pos) => Math.min(text.slice(0, pos).split("\n").length, text.trimEnd().split("\n").length)

/** The tree of a YAML text whose first line is the lock's line `from`, a parse error as a LockError with its line. */
function tree(text, from = 1) {
  const at = (pos) => from - 1 + lineAt(text, pos)
  const doc = parseDocument(text, YAML_1_2_CORE)
  for (const err of doc.errors) fail(at(err.pos[0]), err.message)
  visit(doc, { Alias: (_, node) => (node.resolve(doc) === undefined ? fail(at(node.range[0]), `the alias *${node.source} names no anchor`) : undefined) })
  return doc.toJS()
}

/** The compiler a lock names and its format, from its first two lines alone. */
export function header(text) {
  const lines = text.split(/\r?\n/)
  const entry = (n, name) => {
    const content = lines[n - 1] ?? ""
    if (!content) fail(n, `nothing where \`${name}:\` stands`)
    const line = tree(`${content}\n`, n)
    if (!isMapping(line) || Object.keys(line).join() !== name) fail(n, `\`${content}\` where \`${name}:\` stands`)
    if (line[name] === null) fail(n, `\`${name}:\` with no value`)
    return line[name]
  }
  const teq = entry(1, "teq")
  const format = entry(2, "format")
  if (typeof teq !== "string") fail(1, "the compiler's version is no string")
  if (!Number.isInteger(format)) fail(2, "the format is no integer")
  return { teq, format }
}

/** The tree of a lock's text: mappings as objects, lists as arrays, the core schema's scalars. */
export function parse(text) {
  if (!text) fail(1, "an empty file")
  const lock = tree(text)
  if (!isMapping(lock)) fail(1, "the lock is no mapping")
  return lock
}
