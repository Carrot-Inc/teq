/**
 * A reader of `teq.lock` (docs/TARGETS.md, "The export and the project verbs"): the subset of YAML 1.2
 * sbt-teq writes, read as teq's `src/task/lock.rs` reads it, the rest refused with its line.
 */

export const FILE = "teq.lock"
/** The format this reader reads, the lock's second line. */
export const FORMAT = 1
const MAX_KEY = 1024
const MAX_DEPTH = 64
const RESERVED = new Set("null Null NULL true True TRUE false False FALSE yes Yes YES no No NO on On ON off Off OFF y Y n N .inf .Inf .INF .nan .NaN .NAN".split(" "))

/** A word of a plain string: ASCII letters, digits and `_ . / + - : @`, led by a letter, a digit, `_`, `.` or `/`, not ended by `:`. */
export const word = (s) => /^[A-Za-z0-9_./][A-Za-z0-9_./+\-:@]*$/.test(s) && !s.endsWith(":")
const numberLike = (s) => /^(?:[0-9]|\.[0-9])[0-9._:\-+eEtTzZ]*$/.test(s) && s.split(".").length <= 2
const radix = (s) => /^0[xob][0-9a-fA-F_]+$/.test(s)
/** A string written plain as a key or a flow mapping's value. */
export const plain = (s) => word(s) && !RESERVED.has(s) && !numberLike(s) && !radix(s)
/** A string written plain on a line of its own: words joined by single spaces, the first plain. */
export const plainLine = (s) => {
  const words = s.split(" ")
  return plain(words[0]) && words.every(word)
}
const unprinted = (c) => {
  const o = c.codePointAt(0)
  return o < 0x20 || (o >= 0x7f && o <= 0x9f) || o === 0x2028 || o === 0x2029 || o === 0xfeff || o === 0xfffe || o === 0xffff
}
const shown = (k) => (plain(k) ? k : JSON.stringify(k))

export class LockError extends Error {}

/** An own property of the tree, whatever its name (`__proto__` among them). */
const put = (o, k, v) => Object.defineProperty(o, k, { value: v, enumerable: true, writable: true, configurable: true })

const fail = (line, message) => {
  throw new LockError(`line ${line}: ${message}`)
}

/** The compiler a lock names and its format, from its first two lines alone. */
export function header(text) {
  const lines = text.split("\n").map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l))
  const entry = (n, name) => {
    const content = lines[n - 1] ?? ""
    if (!content) fail(n, `nothing where \`${name}:\` stands`)
    const [k, rest] = key(content, n)
    if (k !== name) fail(n, `\`${content}\` where \`${name}:\` stands`)
    if (!rest.startsWith(" ")) fail(n, `\`${name}:\` with no value`)
    return inline(rest.slice(1), n, 0)
  }
  const teq = entry(1, "teq")
  const format = entry(2, "format")
  if (typeof teq !== "string") fail(1, "the compiler's version is no string")
  if (typeof format !== "number") fail(2, "the format is no integer")
  return { teq, format }
}

function quoted(s, line) {
  let out = ""
  for (let i = 1; i < s.length; ) {
    const c = String.fromCodePoint(s.codePointAt(i))
    if (c === '"') return [out, i + 1]
    if (c === "\\") {
      const e = s[i + 1]
      const simple = { '"': '"', "\\": "\\", n: "\n", r: "\r", t: "\t" }[e]
      if (simple !== undefined) {
        out += simple
        i += 2
      } else if (e === "u") {
        const h = s.slice(i + 2, i + 6)
        const code = /^[0-9a-fA-F]{4}$/.test(h) ? Number.parseInt(h, 16) : -1
        if (code < 0 || (code >= 0xd800 && code <= 0xdfff)) fail(line, `the escape \\u${h}, which is no character`)
        out += String.fromCharCode(code)
        i += 6
      } else if (e === undefined) break
      else fail(line, `the escape \\${e}, which the lock does not use`)
    } else if (unprinted(c)) {
      fail(line, `the character U+${c.codePointAt(0).toString(16).toUpperCase().padStart(4, "0")} in quotes, which the lock escapes`)
    } else {
      out += c
      i += c.length
    }
  }
  fail(line, "a quoted string not closed on its line")
}

function bare(token, line) {
  if (token === "true" || token === "false") return token === "true"
  if (/^-?(?:0|[1-9][0-9]*)$/.test(token) && token !== "-0") {
    const n = Number(token)
    if (!Number.isSafeInteger(n)) fail(line, `the integer ${token} is out of range`)
    return n
  }
  if (plain(token)) return token
  fail(line, `the bare \`${token}\`, which YAML reads as other than this text: quote it`)
}

function flow(s, at, line, depth) {
  if (depth > MAX_DEPTH) fail(line, `nested deeper than ${MAX_DEPTH} levels`)
  let pos = at + 1
  const entries = {}
  if (s.startsWith("}", pos)) return [entries, pos + 1]
  for (;;) {
    let k
    let end
    if (s.startsWith('"', pos)) {
      const [q, n] = quoted(s.slice(pos), line)
      k = q
      end = pos + n
    } else {
      const n = s.indexOf(": ", pos)
      end = n >= 0 ? n : s.length
      k = s.slice(pos, end)
      if (!plain(k)) fail(line, `the bare key \`${k}\` in a flow mapping, which YAML may read otherwise: quote it`)
    }
    if (!s.startsWith(": ", end)) fail(line, `no \`: \` after the key ${shown(k)} in a flow mapping`)
    if ([...s.slice(pos, end)].length > MAX_KEY) fail(line, `a key of more than YAML's ${MAX_KEY} characters`)
    if (Object.hasOwn(entries, k)) fail(line, `the key ${shown(k)} a second time`)
    pos = end + 2
    let v
    if (s.startsWith("{", pos)) [v, pos] = flow(s, pos, line, depth + 1)
    else if (s.startsWith('"', pos)) {
      const [q, n] = quoted(s.slice(pos), line)
      v = q
      pos += n
    } else {
      const stops = [",", "}", " "].map((c) => s.indexOf(c, pos)).filter((i) => i >= 0)
      const n = stops.length ? Math.min(...stops) : s.length
      v = bare(s.slice(pos, n), line)
      pos = n
    }
    put(entries, k, v)
    if (s.startsWith(", ", pos)) pos += 2
    else if (s.startsWith("}", pos)) return [entries, pos + 1]
    else fail(line, "a flow mapping not closed by `}`")
  }
}

function inline(s, line, depth) {
  if (s.startsWith("{")) {
    const [v, end] = flow(s, 0, line, depth + 1)
    if (end !== s.length) fail(line, `\`${s.slice(end)}\` after a flow mapping`)
    return v
  }
  if (s.startsWith("[")) {
    if (s === "[]") return []
    fail(line, "a flow list, which the lock writes as `[]` alone")
  }
  if (s.startsWith('"')) {
    const [v, end] = quoted(s, line)
    if (end !== s.length) fail(line, `\`${s.slice(end)}\` after a quoted string`)
    return v
  }
  if (s.includes(" ")) {
    if (plainLine(s)) return s
    fail(line, `the bare text \`${s}\`, which YAML may read otherwise: quote it`)
  }
  return bare(s, line)
}

function key(content, line) {
  let k
  let end
  if (content.startsWith('"')) [k, end] = quoted(content, line)
  else {
    const token = content.split(" ")[0]
    if (!token.endsWith(":")) fail(line, `\`${content}\` where a key and its colon stand`)
    k = token.slice(0, -1)
    end = k.length
    if (!plain(k)) fail(line, `the bare key \`${k}\`, which YAML reads as other than this text: quote it`)
  }
  if ([...content.slice(0, end)].length > MAX_KEY) fail(line, `a key of more than YAML's ${MAX_KEY} characters`)
  if (!content.startsWith(":", end)) fail(line, `no colon after the key ${shown(k)}`)
  return [k, content.slice(end + 1)]
}

const isElement = (content) => content === "-" || content.startsWith("- ")

function isEntry(s) {
  if (s.startsWith('"')) {
    try {
      const [, end] = quoted(s, 0)
      return s.startsWith(":", end)
    } catch {
      return false
    }
  }
  return !s.startsWith("{") && s.split(" ")[0].endsWith(":")
}

/** The tree of a lock's text: mappings as objects, lists as arrays, strings, integers and truth values. */
export function parse(text) {
  if (!text.endsWith("\n")) fail(Math.max(text.split("\n").length, 1), text ? "no newline at the end" : "an empty file")
  const lines = text
    .slice(0, -1)
    .split("\n")
    .map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l))
  let pos = 0
  let inner
  const current = () => {
    if (pos >= lines.length) return undefined
    const line = lines[pos]
    if (inner !== undefined) return [inner, line.slice(inner)]
    const content = line.replace(/^ +/, "")
    if (!content) fail(pos + 1, "a blank line")
    if (content[0] === "\t") fail(pos + 1, "a tab in the indentation")
    return [line.length - content.length, content]
  }
  const advance = () => {
    pos++
    inner = undefined
  }
  const block = (column, depth) => {
    if (depth > MAX_DEPTH) fail(pos + 1, `nested deeper than ${MAX_DEPTH} levels`)
    const found = current()
    if (!found) fail(pos + 1, "the end where a block stands")
    if (found[0] !== column) fail(pos + 1, `indented ${found[0]} spaces where its block is at ${column}`)
    return isElement(found[1]) ? list(column, depth) : mapping(column, depth)
  }
  const mapping = (column, depth) => {
    const entries = {}
    for (let found = current(); found; found = current()) {
      const [indent, content] = found
      const line = pos + 1
      if (indent < column) break
      if (indent > column) fail(line, `indented ${indent} spaces where its mapping's entries are at ${column}`)
      if (isElement(content)) fail(line, "a list's element among a mapping's entries")
      const [k, rest] = key(content, line)
      if (Object.hasOwn(entries, k)) fail(line, `the key ${shown(k)} a second time`)
      advance()
      if (!rest) {
        const below = current()
        if (!below || below[0] <= column) fail(line, `${shown(k)} with nothing below it`)
        put(entries, k, block(column + 2, depth + 1))
      } else if (rest.startsWith(" ")) put(entries, k, inline(rest.slice(1), line, depth))
      else fail(line, "no space after the key's colon")
    }
    return entries
  }
  const list = (column, depth) => {
    const items = []
    for (let found = current(); found; found = current()) {
      const [indent, content] = found
      const line = pos + 1
      if (indent < column) break
      if (indent > column) fail(line, `indented ${indent} spaces where its list's elements are at ${column}`)
      if (!content.startsWith("- ")) fail(line, content === "-" ? "an element with nothing after its dash" : "a mapping's entry among a list's elements")
      const rest = content.slice(2)
      if (isElement(rest) || isEntry(rest)) {
        inner = column + 2
        items.push(block(column + 2, depth + 1))
      } else {
        advance()
        items.push(inline(rest, line, depth))
      }
    }
    return items
  }
  const first = current()
  if (!first || first[0] !== 0 || first[1].startsWith("-")) fail(1, "the lock is no mapping at its first column")
  return mapping(0, 0)
}
