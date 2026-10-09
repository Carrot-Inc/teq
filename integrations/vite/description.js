import { existsSync, readFileSync } from "node:fs"
import { basename, dirname, join, resolve } from "node:path"
import { FILE, FORMAT, header, parse } from "./lock.js"

export const EXPORT = FILE
/** Where the export lies without the native driver, relative to the build's root. */
export const UNDER_TARGET = `target/teq/${FILE}`

/** The export at or above a directory: at each, its `teq.lock`, else its `target/teq/teq.lock`. */
export function findExport(from) {
  for (let dir = resolve(from); ; dir = dirname(dir)) {
    for (const file of [join(dir, EXPORT), join(dir, UNDER_TARGET)]) if (existsSync(file)) return file
    if (dirname(dir) === dir) return undefined
  }
}

/** The build's root of an export: the directory whose `target/teq/` holds it, else its own. */
export function rootOf(file) {
  const dir = dirname(resolve(file))
  return basename(dir) === "teq" && basename(dirname(dir)) === "target" ? dirname(dirname(dir)) : dir
}

/**
 * A Scala.js project of the export sbt-teq's `teqExportAll` writes, `teq.lock` at or above `base`
 * (docs/TARGETS.md, "The export and the project verbs"), as far as the plugin reads it: teq's
 * command lines are `teq task`'s, which reads the same file, so the plugin takes the project's source roots (the files its
 * watcher reports), its per-file packages and `hotSwapUpTo`, and the compiler the lock's header
 * pins. teq runs from `root`, the build's root (the lock's directory, or the one whose `target/teq/`
 * holds it), to which the source paths and the diagnostics' file names are relative.
 */
export function loadDescription(project, base = process.cwd()) {
  const file = findExport(base)
  if (!file) throw new Error(`teq: no ${EXPORT} at or above ${resolve(base)}, at a directory or under its target/teq/: export the build with sbt teqExportAll`)
  const text = readFileSync(file, "utf-8")
  let lock
  try {
    const { teq, format } = header(text)
    if (format !== FORMAT) throw new Error(`format ${format}, which this vite-plugin-teq does not read (it reads format ${FORMAT}): update vite-plugin-teq to the release of teq ${teq}`)
    lock = parse(text)
  } catch (err) {
    throw new Error(`teq: ${file}: ${err.message}`)
  }
  const projects = lock.projects ?? {}
  const p = Object.hasOwn(projects, project) ? projects[project] : undefined
  if (!p) throw new Error(`teq: ${file} has no project ${project} (it has ${Object.keys(projects).join(", ")})`)
  if (p.platform !== "js") throw new Error(`teq: ${project} of ${file} is a JVM project; the vite plugin builds JavaScript`)
  const d = p.description ?? {}
  return {
    file,
    root: rootOf(file),
    project,
    base: p.base ?? ".",
    pin: { version: lock.teq, binaries: lock.binaries ?? {} },
    inputs: [...(d.lib ? [d.lib] : []), ...(d.sources ?? closureRoots(projects, project))],
    modulePerFile: d.modulePerFile ?? [],
    hotSwapUpTo: Number(d.keys?.hotSwapUpTo ?? 100),
  }
}

/** The compile sources of a project and of the projects its classpath takes products of. */
function closureRoots(projects, project) {
  const seen = new Set()
  const roots = []
  const visit = (name, configuration) => {
    if (seen.has(`${name}/${configuration}`)) return
    seen.add(`${name}/${configuration}`)
    const c = projects[name]?.configurations?.[configuration]
    for (const entry of c?.classpath ?? []) if (entry.project) visit(entry.project, entry.configuration)
    for (const root of c?.sources ?? []) if (!roots.includes(root)) roots.push(root)
  }
  visit(project, "compile")
  return roots
}

/** `teq --export <lock> <verb> <project>` over the description's export, with teq's options after. */
export function taskArgs(description, verb, options) {
  return ["--export", description.file, verb, description.project, ...options]
}

/** The dev session's options: one module per package into `out`, and per file for the `modulePerFile` packages, with `--hot`. */
export function watchOptions(description, { out }) {
  const small = description.modulePerFile.length ? ["--module-per-file", description.modulePerFile.join(",")] : []
  return ["--split", out, ...small, "--hot"]
}

/** Whether a module of the split output holds one file of a `modulePerFile` package. */
export function isSmallModule(description, name) {
  return name !== "main.mjs" && description.modulePerFile.some((prefix) => name.startsWith(`${prefix}.`))
}
