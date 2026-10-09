import { mkdirSync, renameSync, statSync } from "node:fs"
import { basename, dirname, isAbsolute, join, posix, relative, resolve, sep } from "node:path"
import { resolveTeq } from "./binary.js"
import { isSmallModule, loadDescription, taskArgs, watchOptions } from "./description.js"
import { build, isGiveWayNote, TeqWatch } from "./teq.js"

export { loadDescription } from "./description.js"

const PLUGIN = "teq"
// The listing of a --hot build's modules, which teq writes when every module is in place: it
// accepts its own update, and the page takes it with every swap (docs/TARGETS.md, "Module
// splitting").
const LISTING = "hot-build.mjs"

/**
 * Builds a Scala.js project of the build's export, `teq.lock`, with teq in the place of the
 * Scala.js linker. The dev server keeps `teq watch <project>` writing one ES module per
 * package, and per file for the project's `modulePerFile` packages, and applies each rebuild as a
 * hot swap; `vite build` bundles the single-file output of `teq build <project> --release`.
 * README.md has the options.
 */
export default function teq({ project, prefix = "scalajs", outDir, generate, description } = {}) {
  if (description !== undefined) throw new Error('teq: the description option went with target/teq/build.json: name the project of teq.lock, teq({ project: "..." })')
  if (!project) throw new Error('teq: name the project of teq.lock the page builds, teq({ project: "..." })')
  const importPrefix = `${prefix}:`
  let described = undefined
  let runner = undefined
  let isDev = undefined
  let logger = undefined
  let outputDir = undefined
  let root = undefined
  let base = undefined
  let lastError = undefined
  let watcher = undefined

  // Named like the Scala.js linker's staged output (target/scala-*/<app>-fastopt-staged/), so that
  // the content globs of a Tailwind config written for it keep matching; one per dev server port.
  const outputDirOf = (root, command, port) => {
    if (typeof outDir === "function") return resolve(root, outDir({ root, command, port }))
    if (outDir) return resolve(root, outDir)
    const name = command === "serve" ? `${basename(root)}-teq-${port}-staged` : `${basename(root)}-teq-staged`
    return join(root, "target", "teq", name)
  }

  const entry = () => (isDev ? join(outputDir, "main.mjs") : join(outputDir, "main.js"))

  // The URL vite serves a file of the output directory at: by its path under the root, through
  // /@fs/ elsewhere, under the configured base.
  const servedUrl = (file) => {
    const rel = relative(root, file)
    const posixPath = (p) => p.split(sep).join("/")
    const outside = rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel)
    return outside ? posix.join(base, "@fs", posixPath(file)) : posix.join(base, posixPath(rel))
  }

  const regenerate = async () => (generate ? ((await generate.run(described)) ?? []).map((file) => resolve(described.root, file)) : [])

  function reportError(diagnostics, text) {
    lastError = overlayError(diagnostics, text, described.root)
    logger.error(text.trimEnd(), { timestamp: true })
  }

  function reportWatch(result) {
    if (result.ok) {
      lastError = undefined
      const ms = result.ms
      const phases = `type ${ms.type.toFixed(1)}, reach ${ms.reach.toFixed(1)}, emit ${ms.emit.toFixed(1)}, write ${ms.write.toFixed(1)}`
      const how = result.incremental ? "incremental" : `full build (${result.fallback})`
      logger.info(`teq: ${how} in ${ms.total.toFixed(0)} ms (${phases}); ${result.changed.length} of ${result.modules} modules written`, {
        timestamp: true,
      })
    } else {
      reportError(result.diagnostics, result.diagnostics.map(diagnosticText).join("\n"))
    }
    for (const w of result.warnings ?? []) logger.warn(`teq: ${diagnosticText(w)}`, { timestamp: true })
  }

  return {
    name: PLUGIN,

    // The modules land one by one and the plugin applies each build itself, so vite's own
    // watcher must not react to them.
    config(userConfig, { command }) {
      const dir = outputDirOf(resolve(userConfig.root ?? process.cwd()), command, userConfig.server?.port ?? 5173)
      return { server: { watch: { ignored: [`${dir}/**`] } } }
    },

    async configResolved(resolved) {
      isDev = resolved.command === "serve"
      logger = resolved.logger
      root = resolved.root
      base = resolved.base
      outputDir = outputDirOf(resolved.root, resolved.command, resolved.server.port ?? 5173)
      described = loadDescription(project, resolved.root)
      runner = { teq: await resolveTeq(described, (line) => logger.info(line, { timestamp: true })), cwd: described.root }
    },

    // teq writes hot-refresh.mjs into the output directory under --hot (docs/TARGETS.md, "Module
    // splitting"). The injected script reaches the browser as written, so it imports the runtime
    // by the URL vite serves that file at, the one an entry's own `${prefix}:hot-refresh.mjs`
    // import is rewritten to: one module either way.
    transformIndexHtml() {
      if (!isDev) return
      // The plugin decides how many modules a swap may run again (`hotSwapUpTo`) before it
      // sends one, so the bound that main.mjs has for a server without one is lifted.
      return [{ tag: "script", attrs: { type: "module" }, children: `import "${servedUrl(join(outputDir, "hot-refresh.mjs"))}"\nglobalThis.__teqSwapUpTo = Infinity`, injectTo: "head-prepend" }]
    },

    async buildStart() {
      await regenerate()
      if (isDev) {
        const onStderr = (text) => (isGiveWayNote(text) ? logger.info(text.trimEnd(), { timestamp: true }) : logger.error(`teq: ${text.trimEnd()}`))
        watcher = new TeqWatch(runner, taskArgs(described, "watch", watchOptions(described, { out: outputDir })), { onStderr })
        // the dev server starts anyway: the last good build is served and the overlay says why
        reportWatch(await watcher.start())
        return
      }
      const target = `${entry()}.tmp`
      mkdirSync(dirname(target), { recursive: true })
      const result = await build(runner, taskArgs(described, "build", ["-o", target, "--release", "--time"]))
      if (!result.ok) throw new Error(`teq build failed:\n${result.stderr}`)
      renameSync(target, entry())
      logger.info(`teq:\n${result.stderr.trimEnd()}`, { timestamp: true })
    },

    buildEnd() {
      watcher?.stop()
      watcher = undefined
    },

    resolveId(source) {
      if (!source.startsWith(importPrefix)) return null
      const name = source.slice(importPrefix.length)
      return name === "main.js" ? entry() : join(outputDir, name)
    },

    // An empty map tells vite's import analysis not to build one: for megabytes of modules it
    // would otherwise inline source maps four times the code on every reload. The eager-init
    // footer that re-runs a re-executed module's `$hot()` once the page has booted is teq's own
    // (docs/TARGETS.md, "Module splitting"), written into the module text under --hot.
    transform(code, id) {
      if (!outputDir || !id.startsWith(outputDir + sep)) return null
      return { code, map: { mappings: "" } }
    },

    // Updates of the output's modules are the plugin's (below); an event the watcher reports for
    // one anyway, say after the ignore pattern missed a rename, must not start a second update.
    hotUpdate({ file }) {
      if (isDev && outputDir && file.startsWith(outputDir + sep)) return []
    },

    configureServer(server) {
      const scalaRoots = described.inputs.map((dir) => resolve(described.root, dir))
      const generatorRoots = (generate?.watch ?? []).map((dir) => resolve(described.root, dir))
      server.watcher.add([...scalaRoots, ...generatorRoots])
      const graph = server.environments?.client?.moduleGraph ?? server.moduleGraph
      const hot = server.environments?.client?.hot ?? server.hot ?? server.ws
      const swapped = new Map()

      // What vite's `invalidateModule` does to one node. The browser re-executes exactly the
      // modules whose URL changes: the per-file modules teq rewrote, the per-file modules
      // importing them (`importingModules`), and main.mjs and its accepting importer above them
      // (`hotSwap`). A module that is not per file imports no per-file module under --hot
      // (docs/TARGETS.md, "Module splitting"), so none is on the way and none keeps an address
      // of before the swap.
      function invalidate(mod, timestamp) {
        mod.invalidationState = "HARD_INVALIDATED"
        mod.lastHMRTimestamp = timestamp
        mod.lastHMRInvalidationReceived = false
        const etag = mod.transformResult?.etag
        if (etag) graph.etagToModuleMap.delete(etag)
        mod.transformResult = null
        mod.ssrModule = null
        mod.ssrError = null
      }

      function invalidateFiles(files, timestamp) {
        for (const file of files) for (const mod of graph.getModulesByFile(file) ?? []) invalidate(mod, timestamp)
      }

      // A reload needs no new addresses: vite's own invalidation, which leaves them as they
      // are and has every importer of a rewritten module read anew.
      function reload(files, why) {
        for (const file of [...files, join(outputDir, LISTING)]) for (const mod of graph.getModulesByFile(file) ?? []) graph.invalidateModule(mod)
        hot.send({ type: "full-reload", path: "*" })
        return `page reload: ${why}`
      }

      // The module importing `<prefix>:main.js` accepts its own updates: re-imported with the
      // timestamp, it imports main.mjs anew, which imports the rewritten modules anew and every
      // other module as loaded, registers the fresh classes and re-runs main(). A file teq
      // reported with the modification time of its last swap is one the page already has.
      function hotSwap(files) {
        const timestamp = Date.now()
        const listing = join(outputDir, LISTING)
        const fresh = files.filter((file) => {
          const mtime = statSync(file, { throwIfNoEntry: false })?.mtimeMs
          if (mtime === undefined || swapped.get(file) === mtime) return false
          swapped.set(file, mtime)
          return file !== listing
        })
        if (fresh.length === 0) return "nothing new"
        // Running shared code again makes classes anew whose instances the page holds.
        const shared = fresh.map((file) => file.slice(outputDir.length + 1)).filter((name) => name !== "main.mjs" && !isSmallModule(described, name))
        if (shared.length > 0) return reload(fresh, `${shared.join(", ")} ${shared.length === 1 ? "is" : "are"} not per file`)
        const chain = importingModules(fresh)
        const reexecuted = fresh.length + chain.modules.size
        if (reexecuted > described.hotSwapUpTo) return reload(fresh, `${reexecuted} modules would re-execute`)
        invalidateFiles(fresh, timestamp)
        for (const mod of chain.modules) invalidate(mod, timestamp)
        const boundaries = new Set()
        for (const mod of graph.getModulesByFile(entry()) ?? []) {
          invalidate(mod, timestamp)
          for (const importer of mod.importers) {
            if (!importer.isSelfAccepting) continue
            invalidate(importer, timestamp)
            boundaries.add(importer.url)
          }
        }
        if (boundaries.size === 0) return reload([], `no importer of ${importPrefix}main.js accepts its own updates`)
        for (const mod of graph.getModulesByFile(listing) ?? []) {
          invalidate(mod, timestamp)
          boundaries.add(mod.url)
        }
        const update = (url) => ({ type: "js-update", timestamp, path: url, acceptedPath: url, explicitImportRequired: false, isWithinCircularImport: false })
        hot.send({ type: "update", updates: [...boundaries].map(update) })
        const stopped = chain.stoppedAt.size > 0 ? `; ${[...chain.stoppedAt].join(", ")} keep their instances` : ""
        return `hot swap, ${reexecuted} module${reexecuted === 1 ? "" : "s"} re-executed${stopped}`
      }

      // A module that imported a rewritten one holds bindings to the instance it was loaded with,
      // so it has to run again to see the new code: a component in one file calling a plain def
      // of another. The importers of a per-file module are per-file modules and main.mjs, which
      // `hotSwap` takes with its own importer. The output of a teq from before such modules
      // were taken through the runtime has modules that are not per file among them, where
      // the walk stops as it did then: they keep their instances, and the bindings they have.
      function importingModules(files) {
        const modules = new Set()
        const stoppedAt = new Set()
        const entryFile = entry()
        const queue = files.flatMap((file) => [...(graph.getModulesByFile(file) ?? [])])
        const changed = new Set(queue)
        while (queue.length > 0) {
          const mod = queue.pop()
          for (const importer of mod.importers) {
            const file = importer.file
            if (!file || file === entryFile || !file.startsWith(outputDir + sep)) continue
            if (changed.has(importer) || modules.has(importer)) continue
            const name = file.slice(outputDir.length + 1)
            if (!isSmallModule(described, name)) {
              stoppedAt.add(name)
              continue
            }
            modules.add(importer)
            queue.push(importer)
          }
        }
        return { modules, stoppedAt }
      }

      let timer = undefined
      let running = false
      let queued = false
      let generatorTriggered = false
      const changedFiles = new Set()
      async function rebuild() {
        if (running) {
          queued = true
          return
        }
        running = true
        const paths = [...changedFiles]
        changedFiles.clear()
        try {
          if (generatorTriggered) {
            generatorTriggered = false
            paths.push(...(await regenerate()))
          }
          const result = await watcher.build(paths)
          if (result.restarted) logger.warn("teq: watch process restarted", { timestamp: true })
          const hadError = lastError !== undefined
          reportWatch(result)
          if (lastError) {
            hot.send({ type: "error", err: lastError })
          } else {
            const changed = result.changed.map((name) => join(outputDir, name))
            const modules = changed.length - (result.changed.includes(LISTING) ? 1 : 0)
            const count = `${modules} module${modules === 1 ? "" : "s"} changed`
            if (modules > described.hotSwapUpTo) {
              logger.info(`teq: ${count}, ${reload(changed, "over hotSwapUpTo")}`, { timestamp: true })
            } else if (changed.length > 0) {
              logger.info(`teq: ${count}, ${hotSwap(changed)}`, { timestamp: true })
            } else if (hadError) {
              // a fix that gives back the modules of the last good build still has an overlay to clear
              hot.send({ type: "update", updates: [] })
            }
          }
        } catch (err) {
          logger.error(`teq: ${err}`, { timestamp: true })
        }
        running = false
        if (queued) {
          queued = false
          rebuild()
        }
      }
      const schedule = () => {
        clearTimeout(timer)
        timer = setTimeout(rebuild, 10)
      }

      const under = (roots, file) => roots.some((root) => file.startsWith(root + sep))
      for (const event of ["change", "add", "unlink"]) {
        server.watcher.on(event, (file) => {
          if (file.endsWith(".scala") && under(scalaRoots, file)) {
            changedFiles.add(file)
            schedule()
          } else if (under(generatorRoots, file) && (generate.on?.(event, file) ?? true)) {
            generatorTriggered = true
            schedule()
          }
        })
      }
      hot.on("connection", () => {
        if (lastError) hot.send({ type: "error", err: lastError })
      })
    },
  }
}

export { teq }

function diagnosticText({ file, line, column, message, lines, severity = "error" }) {
  return [`${file}:${line}:${column}: ${severity}: ${message}`, ...lines].join("\n")
}

/** vite's overlay shows the first error the way teq prints it, and counts the rest. */
function overlayError(diagnostics, text, root) {
  const [first, ...rest] = diagnostics ?? []
  if (!first) return { message: text, stack: "", plugin: PLUGIN }
  const file = resolve(root, first.file)
  const more = rest.length ? `\n\n${rest.length} more error${rest.length === 1 ? "" : "s"} in the terminal` : ""
  return {
    message: [first.message, ...first.lines].join("\n") + more,
    stack: "",
    id: file,
    loc: { file, line: first.line, column: first.column },
    plugin: PLUGIN,
  }
}
