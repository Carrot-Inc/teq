// A page of a `--hot` build without a browser: the modules run under node with the hot API of a
// development server stood in for, and what such a server does after a build is done step by
// step, in the order the steps are given, so that a test decides which file lands when.
//
//   node tests/support/hot-page.mjs <page directory> <step>...
//
//   boot:<build>            the page directory becomes a copy of the build's, and main.mjs is run
//   publish:<build>:<name>  the module <name>.mjs of the build replaces the page directory's, as a
//                           build's rename does; `remove:<name>` takes the file away
//   update:<name>[@<t>]     the update of a module that accepts its own: it is run again
//   swap:<name>,<name>...   the update that runs the named per-file modules and main.mjs again;
//                           an update that fails is a line (`failed: ...`), the page is told
//                           of it as the accepting module's callback tells it under a
//                           development server's client, and the page goes on
//   call:<name>:<export>:<argument>  calls an export of the first instance of a module
//   upto:<n>                a swap of more than n modules is too wide to be one
//   made:<object>           how often the object of that qualified name was constructed
//   show                    a line with the reloads asked for so far and what is pending: the
//                           modules whose text is not the listing's, with a `?` where the text
//                           is and the build that wrote it is not, and the modules whose
//                           update failed
//
// Every module run again gets an address with the time of its update, counted from 1, or the
// time an update names.
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs"
import { join, resolve } from "node:path"
import { pathToFileURL } from "node:url"

const [pageDir, ...steps] = process.argv.slice(2)
const page = resolve(pageDir)
const address = (name, query = "") => pathToFileURL(join(page, `${name}.mjs`)).href + query
// The hot API is `import.meta.hot` in the text; under node the stand-in is a global.
const served = (text) => text.replaceAll("import.meta.hot", "globalThis.$hotApi")
const publish = (build, name) => writeFileSync(join(page, `${name}.mjs`), served(readFileSync(join(build, `${name}.mjs`), "utf8")))

let reloads = 0
let time = 0
globalThis.$hotApi = { accept() {} }
globalThis.location = { reload() { reloads++ } }
const made = []
globalThis.$teqHotObject = (object, id) => made.push(id)
const log = console.log
const failed = (name, t) => (e) => { log(`failed: ${e.message}`); globalThis.__teqHot?.failed_(name, t) }

for (const step of steps) {
  const [what, ...args] = step.split(":")
  if (what === "boot") {
    rmSync(page, { recursive: true, force: true })
    mkdirSync(page, { recursive: true })
    for (const file of readdirSync(args[0])) if (file.endsWith(".mjs")) publish(args[0], file.slice(0, -4))
    await import(address("main"))
  } else if (what === "publish") {
    publish(args[0], args[1])
  } else if (what === "remove") {
    rmSync(join(page, `${args[0]}.mjs`))
  } else if (what === "update") {
    const [name, at] = args[0].split("@")
    const t = at === undefined ? ++time : Number(at)
    await import(address(name, `?t=${t}`)).catch(failed(name, t))
  } else if (what === "swap") {
    // As the server does: the modules run again are imported under the address of the update
    // by every module that is run again with them.
    const t = ++time
    const names = [...args[0].split(","), "main"]
    const texts = new Map(names.map((name) => [name, readFileSync(join(page, `${name}.mjs`), "utf8")]))
    for (const [name, text] of texts) {
      writeFileSync(join(page, `${name}.mjs`), names.reduce((s, n) => s.replaceAll(`"./${n}.mjs"`, `"./${n}.mjs?t=${t}"`), text))
    }
    try {
      await import(address("main", `?t=${t}`)).catch(failed("main", t))
    } finally {
      for (const [name, text] of texts) writeFileSync(join(page, `${name}.mjs`), text)
    }
  } else if (what === "call") {
    const module = await import(address(args[0]))
    log(`${args[1]}: ${module[args[1]](...args.slice(2))}`)
  } else if (what === "upto") {
    globalThis.__teqSwapUpTo = Number(args[0])
  } else if (what === "made") {
    await new Promise((r) => setTimeout(r, 5))
    log(`${args[0]} made ${made.filter((id) => id === args[0]).length} times`)
  } else if (what === "show") {
    await new Promise((r) => setTimeout(r, 5))
    const pending = globalThis.__teqHot?.pending()
    const waits = pending?.waits.map((w) => (w.has.split(" ")[0] !== w.listed.split(" ")[0] ? w.module : `${w.module}?`)).sort().join(",")
    const failed = pending?.failed.length > 0 ? ` failed=${pending.failed.sort().join(",")}` : ""
    log(`reloads=${reloads} ${pending === undefined ? "nothing pending" : `wish="${pending.wish}" waits=${waits}${failed}`}`)
  } else throw new Error(`no such step: ${step}`)
}
