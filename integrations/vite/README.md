# vite-plugin-teq

A vite plugin that builds a Scala.js project with [teq](../../README.md) in the place of the
Scala.js linker, a project of the build's export, `teq.lock`, which [sbt-teq](../sbt)'s
`teqExportAll` writes and the repository commits. With the sbt plugin it is the whole switch: the
export describes the build once, and vite runs teq itself from then on, sbt staying out of the edit
loop.

```js
// vite.config.js
import teq from "vite-plugin-teq"

export default defineConfig({ plugins: [teq({ project: "frontend" })] })
```

```js
// main.js, the page's entry
import "scalajs:main.js"

if (import.meta.hot) import.meta.hot.accept()
```

- `vite` (the dev server) keeps `teq watch <project>` writing the program as one ES module
  per package, and per file for the project's `modulePerFile` packages, with `--hot`. A source change is
  rebuilt, the modules teq rewrote are invalidated in vite's module graph and the entry that
  accepts its own updates is re-imported: the browser re-executes the rewritten modules, the
  per-file modules importing them and `main.mjs`, and React Fast Refresh re-renders the mounted
  components with their state. Which modules those are is in teq's output, not in the plugin:
  under `--hot` a module that is not per file imports no per-file module, so none is on the way
  from an edited one (`docs/TARGETS.md`, "The development loop: `--module-per-file` and `--hot`"),
  and teq writes the footers that
  re-run a re-executed module's `$hot()` once the page has booted. The plugin decides before it
  sends a swap: a rebuild that would re-execute more than `hotSwapUpTo` modules, one that
  rewrote a module that is not per file (shared code, whose classes the page holds instances
  of), or no accepting entry reloads the page; a compile error goes to vite's overlay while the
  last good build stays served. A full build whose parallel attempt gives way is typed again by
  one worker before the session answers, and teq's note saying so is logged as information, its
  other stderr as errors. With every swap the page takes the update of `hot-build.mjs`,
  the listing of the build's modules that teq writes last.
- `vite build` runs `teq build <project> --release` into one file, the production sources
  and excludes of the project's description included, and bundles it.

`teq dev <project>` runs the whole loop with no sbt: the project's generators, the package
manager's install when the lockfile changed, and the export's dev command (this plugin's `vite`),
restarted when the export or the binary changes (`docs/TOOLING.md`, "`teq`").

## Options

| Option | Default | |
|---|---|---|
| `project` | none, required | the project of `teq.lock`, at or above vite's root (at each directory `teq.lock`, else `target/teq/teq.lock`), that the page builds |
| `prefix` | `"scalajs"` | the import prefix: `scalajs:main.js` is the program, `scalajs:<module>` any module of the output, as with the Scala.js vite plugin |
| `outDir` | `target/teq/<root's name>-teq-<port>-staged` in dev, `target/teq/<root's name>-teq-staged` for `vite build` | where teq writes, relative to vite's root; a function of `{ root, command, port }` may name it |
| `generate` | none | sources the application generates: `{ run, watch, on }` (below) |

The default output directory is named like the Scala.js linker's staged output
(`<app>-fastopt-staged`), so a Tailwind content glob written for that output (`**/<app>-*-staged/**`)
keeps finding the class names; each dev server port has its own, so two servers never share one.

`generate` is for code generated from files other than Scala sources that the export does not
name. `run(description)` regenerates and returns, or resolves to, the files it wrote (relative to
the description's `root`, the export's directory, or absolute); it gets the description as loaded
(`root`, `inputs`) and runs before the first dev build and before `vite build`, and again when
vite's watcher reports an event under one of the `watch` paths for which `on(event, file)` returns
true (every event without `on`), the written files joining the rebuild. The export's own
generators (sbt-teq's `teqGenerators` and sbt-buildinfo's object) need none of it: `teq watch`
runs them when it starts, and `teq dev` whenever their inputs change.

## What it reads of the export

teq runs from the export's directory, to which every path of the export is relative. Of the
project the plugin reads its source roots (`description.sources` and `lib`), which its watcher
reports to the session; `modulePerFile`, the packages swapped per file; and `hotSwapUpTo` among
the description's `keys` (`teqDescriptionKeys := Map("hotSwapUpTo" -> "50")`), how many modules a
swap may re-execute before the page reloads instead (100). Everything else of the command lines
(the excludes, the jars, the flags, `--threads`, the production sources) is `teq`'s, which
reads the same file. A JVM project is refused.

The binary: `TEQ` in the environment; else teq's cache (`TEQ_CACHE_DIR`, else
`$XDG_CACHE_HOME/teq`, else `~/Library/Caches/teq` on macOS, `%LOCALAPPDATA%\teq` on Windows,
`~/.cache/teq` elsewhere), `bin/<sha1>/teq-<version>-<classifier>` for the binary the lock's
`binaries` line pins for the platform, which the launchers and sbt-teq fill; else sbt-teq's copy,
`target/teq/bin/teq-<version>-<classifier>` under the project's base or the build's root, when its
size and sha1 are the pinned ones; else coursier's copy of the binary's URL, copied into the
cache; else the plugin fetches it into the cache with curl, as the launchers do: its configuration
on curl's stdin, https alone (plain http to this machine), the size and sha1 checked before the
file is renamed into place.

## Switching an application

1. Add sbt-teq to the build and run `sbt teqExportAll` (again when the build changes:
   dependencies, source roots, options), which writes `target/teq/teq.lock`; with teq as the
   build tool (`teqBuildTool := true`) it writes `teq.lock` at the root, which the
   repository commits.
2. Replace the Scala.js vite plugin in `vite.config.js` with `teq({ project: "..." })`. An entry
   that imports `scalajs:main.js` stays as it is; it needs `import.meta.hot.accept()` for hot
   swaps.
3. A `main()` that renders must not render a second tree when a hot swap re-runs it: it keeps its
   React root across re-runs (on `window`, say) and renders only the first time.

The package is not published yet: an application installs it from a checkout
(`"vite-plugin-teq": "file:<teq>/integrations/vite"` in `devDependencies`, as
`integrations/sbt/example/package.json` does) or imports `<teq>/integrations/vite/index.js` by path.
It depends on the `yaml` package for the lock and on `curl` for the fetch; `vite` is a peer dependency.
npm symlinks a `file:` dependency and installs none of its own, so an install from a checkout with
npm runs `npm install --legacy-peer-deps` in `integrations/vite` once (yarn copies the plugin and needs nothing more). `node test.mjs`
(`npm test`) checks its reading of the export and its search for the binary, a fetch over the
loopback interface among them; `tests/task.sh` runs it.
`integrations/sbt/example/check.sh` builds the example's frontend through it, starts its dev
server, and runs `teq dev browserdemo` under headless Chrome (`check-task-dev.mjs`).

## The refresh runtime

`hot-refresh.mjs` is teq's own file (`docs/TOOLING.md`, "The vite plugin"): `--hot` writes it into
the output directory unchanged, so any server can find and serve it. The plugin injects it ahead of
the page's other scripts in dev, by the URL vite serves the file at; an entry that imports
`scalajs:hot-refresh.mjs` itself has that import rewritten to the same URL, so the page
runs one copy of the runtime either way (on sbt's route the stub `main.js` that sbt-teq writes
imports it). teq's `--hot` output
reports every object it constructs (`$teqHotObject`); the runtime registers the component types
among the object's fields, and one level down, as React Fast Refresh families under the object's
qualified name, and a swap that constructs the objects anew refreshes the mounted instances of the
old types with the new ones. It is the part of react-refresh's runtime a build without hook
signatures needs, so the application needs neither react-refresh nor vite's React plugin.
`window.__teqHotRefresh` holds its state (the families, the renderers, the mounted roots, the
refreshes with their times). Components kept in a `lazy val`, a top-level `val` or created at
render time are not families: a swap remounts them or leaves their old code.
