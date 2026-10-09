# Scala with teq, for Zed

A Zed extension that brings the Scala language (file types, highlighting, brackets, outline,
indentation, sbt tasks) with `teq lsp` as its language server, so that Zed can go to definitions,
find references and implementations, read hovers and document and workspace symbols, walk the
call hierarchy, complete members, names in scope, types and imports, and show teq's diagnostics
after each edit; while a project's first build runs, or a build takes longer than a moment, Zed's
status bar shows `teq: checking <project>` at the bottom left. What the server answers and its
limits are in `docs/TOOLING.md`, "The language server and Zed"; code actions, rename and formatting are not
offered yet.

The language definition (`languages/scala`, the grammar entry) is that of
[scalameta/metals-zed](https://github.com/scalameta/metals-zed) (commit `5fe61bd8`, Apache 2.0,
as this repository), copyright the metals-zed contributors, whose license names no holder; its
`highlights.scm`, `locals.scm` and `tags.scm` are in large part
[tree-sitter-scala](https://github.com/tree-sitter/tree-sitter-scala)'s queries (MIT, Copyright (c)
2018 Max Brunsfeld and GitHub). The repository's `NOTICE` has both. Metals is not part of this
extension: Zed runs one Scala language at a time, so it is this extension or Zed's Scala extension.

## Install

1. Uninstall Zed's own Scala extension if it is installed: two extensions defining the Scala
   language collide.
2. Install this directory as a dev extension: `zed: install dev extension` from the command
   palette, then select `integrations/zed`. Zed compiles it with the machine's Rust toolchain
   (rustup's `wasm32-wasip2` target, which Zed adds itself) and the grammar with the wasi-sdk,
   which it downloads. After the directory changes, `Rebuild` on the extensions page.

On macOS on Apple silicon nothing else: the extension finds a binary or fetches one. For a
project folder it takes, in this order,

- the binary named in Zed's settings, which Zed starts itself with the arguments given there,
  the extension never asked:

  ```json
  "lsp": { "teq": { "binary": { "path": "/path/to/teq", "arguments": ["lsp"] } } }
  ```

- `TEQ` in the folder's shell environment, the variable every teq tool honours;
- the binary the folder's lock pins for the platform (`teq.lock`, else `target/teq/teq.lock`), the compiler the branch builds
  with: fetched from the export's repository into the extension's work directory as
  `teq-<sha1>/teq` and kept only when its sha1 and size are the pinned ones (without
  credentials, which the extension cannot read: a repository that wants them is passed over);
- `teq` on the `PATH`: an installed binary is taken as a choice over the release's;
- its own copy of the newest release of teq for the platform: teq's GitHub release, which the
  extension finds through GitHub's releases API, the latest release, or, when there is
  none but pre-releases, the newest pre-release that is no draft by its tag's version (three pages
  of the listing at most), taking its asset `teq-<version>-<platform>` (`.exe` for Windows alone)
  and `SHA256SUMS` by their exact names and checking the download against the SHA-256 there. The
  releases before 0.1.7 are not served: a lock pinning one is refused, naming the release to pin
  and `TEQ`, nothing else run in its place; neither a copy nor a search of one counts. What
  GitHub's search found is kept in the work
  directory with its time and asked again only after a day, or when its copy is missing, and a
  request GitHub does not answer is made again once: GitHub limits an address's unauthenticated
  requests (sixty an hour, shared by every machine behind it), and when it refuses more, the copies
  present and what was found before serve, a cold start without either saying so. The fetch goes
  into Zed's extension work directory (`~/Library/Application Support/Zed/extensions/work/teq/`
  on macOS) as `teq-<version>/teq` while the status bar says "Downloading teq", kept only when its
  digest and size are the release's.
  A release never changes, so a copy is reused while its digest and size, read again at each
  start, are still its release's (a copy changed in place, whatever its size, is fetched again); a
  release shipped after the extension was built is taken at the next start, and the copies of older
  releases go then. When the repositories are out of reach, or list nothing (the search's index can
  lag a publish), the newest copy present whose digest and size are its stamp's serves. A ship stages macOS on Apple silicon and Intel
  (`osx-aarch_64`, `osx-x86_64`), Linux on x86-64 and aarch64 (`linux-x86_64`, `linux-aarch_64`,
  glibc 2.28 and later) and Windows on x86-64 (`windows-x86_64`), and publishes those of them its
  qualification file allows (docs/TARGETS.md, "Releases"): the newest release that serves the
  platform's binary is taken; elsewhere the fetch fails with a message saying so, and `teq` goes on
  the `PATH` or into the settings.

The language-server button at the bottom right shows which binary runs (hover its `teq` entry).

`zed: open log` shows the server starting; `debug: open language server logs` shows what it
writes on stderr and every message exchanged.

## Installing the bundle

For a machine without the Rust toolchain. `package.sh` here packs what Zed wrote into this
directory when it built the dev extension (`extension.wasm`, `grammars/scala.wasm`; it refuses
when either is older than the sources it was built from) with the manifest, the language files,
the licences (the grammar's MIT notice, `LICENSE-tree-sitter-scala`, included) and the repository's `NOTICE` into
`out/zed-teq-<version>.tar.gz`: WebAssembly and text, so one bundle serves macOS and Linux alike.
Unpack it where Zed keeps its installed extensions, under a directory named after the extension,
replacing an earlier one so that Zed notices the change, and restart Zed:

```sh
d=~/Library/Application\ Support/Zed/extensions/installed/teq   # Linux: ~/.local/share/zed/extensions/installed/teq
rm -rf "$d" && mkdir -p "$d" && tar -xzf zed-teq-0.5.0.tar.gz -C "$d"
```

Zed reads every directory there when it starts (a plain directory is an installed extension, a
symbolic link a dev one), so the extensions page then lists it; Zed's update check finds nothing
for it, a newer bundle is installed the same way. Zed's own Scala extension is uninstalled first,
as above. The bundle is built against extension API 0.7.0 (Zed 0.205 and later; this one with Zed
1.22.0).

## Installing with the macOS installer

For Apple silicon without the Terminal. `installer.sh` here builds `out/zed-teq-<version>.pkg`
from the bundle (`pkgbuild` and `productbuild`, which every Mac has): a package that installs
under the user's home, so it asks for no administrator password, after removing an earlier copy
(`installer/preinstall`; nothing else is touched). It has Zed quit first, and it stops when Zed's
own Scala extension is installed, with the message to uninstall that one from Zed's extensions
page (and to drop a `"scala"` entry from `auto_install_extensions`, or Zed brings it back): an
extension removed behind Zed's back would come back that way. Open the package, continue, done;
the extension is listed after Zed's next start, and the first Scala file fetches teq.

The package is not signed. macOS refuses a downloaded unsigned installer at first: on macOS 15 and
later, after the refusal, System Settings › Privacy & Security shows the package with an "Open
Anyway" button, once; on older versions, Control-click the package and choose Open. Signing it
away takes a Developer ID Installer certificate of the organisation (`productbuild --sign
"Developer ID Installer: …"`) and notarisation (`xcrun notarytool submit --wait`, then `xcrun
stapler staple`), both tied to an Apple developer account.

## The projects it finds

Zed starts one server per project folder, and the server works from that folder and the build's
one description, `teq.lock` (written by `sbt teqExportAll` under the build's `target/teq/`, or at
its root, committed, with teq as the build tool, `teqBuildTool := true`; `docs/TOOLING.md`,
"The lockfile and the launchers"):

- the export at or above the folder, else those at the roots of the sbt builds up to three
  levels below it, describes the projects. Each configuration (a project's `compile`, its `test`)
  is a session, started when a file of its own source roots is opened, or served by a running
  session whose closure already holds the file, so that switching from one application to
  another is opening a file; at most four run at once and one unused for half an hour stops,
  which the folder's `.zed/settings.json` can change:

  ```json
  { "lsp": { "teq": { "initialization_options": { "maxSessions": 2, "sessionIdleSeconds": 600 } } } }
  ```

  An export changed after the server started (a branch switch, a new export) is read again
  within seconds; a build definition changed since the export was written is a warning on
  `build.sbt` naming `sbt teqExportAll`;
- an sbt build without an export is exported by the server itself, once: it runs
  `sbt teqExportAll` with the published sbt-teq loaded for that run alone (sbt's
  `-addPluginSbtFile`; a build whose plugins name sbt-teq runs its own), the status bar showing
  "exporting the sbt build of <name>" meanwhile, and writes `teq.lock` where the build's setting
  says: under its `target/teq/`, adding nothing to the repository, or at its root under
  `teqBuildTool`; an export under `target/teq/` that goes (`sbt clean`) is run again once. The build is the folder, the nearest parent that is one within the same
  repository, or the builds up to three levels below. A run that fails is a diagnostic on
  `build.sbt`, and runs again once a build file changes; an export that exists is never
  refreshed. Needs sbt 2 and `sbt` on the `PATH`, run in the foreground: a `--client` or
  `--jvm-client` in `.sbtopts` sends the run to a server JVM that has no sbt-teq, and the export
  fails saying so (take the flag out, or export with `sbt teqExportAll` yourself); on Windows it
  does not run, and the diagnostic names the command;
- a folder without any export that is no sbt build is one session of every `.scala` file under
  it, on the lean std without a class path. To keep the server out of a folder altogether, in
  its `.zed/settings.json`:

  ```json
  { "languages": { "Scala": { "language_servers": ["!teq"] } } }
  ```

Each project is typed by its own `teq compiler watch --check --index` process, started when the server
starts; a file in several projects (a shared source root) is answered by each of them together.
