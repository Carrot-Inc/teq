# teq-lsp: teq's language server for Claude Code

A Claude Code plugin that starts `teq lsp` for `.scala` files, so that Claude Code's LSP tool can
go to definitions, find references and implementations, read hovers and document and workspace
symbols, walk the call hierarchy, and see teq's diagnostics after each edit. What the server
answers and its limits are in `docs/TOOLING.md`, "The language server and Zed".

## Install

1. Put the `teq` binary on the `PATH` (`cargo build --release` in the repository, then link or
   copy `target/release/teq`). The plugin runs `teq lsp`.
2. Add this directory as a marketplace and install the plugin:

   ```
   claude plugin marketplace add /path/to/teq/integrations/claude-code
   claude plugin install teq-lsp@teq
   ```

`claude plugin validate integrations/claude-code` checks both manifests.

## The projects it finds

The server works from the workspace root Claude Code gives it, and from the build's one
description, `teq.lock` (written by `sbt teqExportAll` and committed; `docs/TOOLING.md`,
"The lockfile and the launchers"):

- the export at or above the root, else those at the roots of the sbt builds up to three levels
  below it, describes the projects; each configuration (a project's `compile`, its `test`) is a
  session, its own `teq compiler watch --check --index` process, started when a file of its source roots
  is opened or asked about, or served by a running session whose closure already holds the
  file; at most four run at once and one unused for half an hour stops, both settable through
  the client's initialization options (`maxSessions`, `sessionIdleSeconds`); an export changed
  after the server started is read again within seconds, and a build definition changed since
  the export was written is a warning on `build.sbt` naming `sbt teqExportAll`;
- an sbt build without an export is exported by the server itself, once (`sbt teqExportAll` with
  the published sbt-teq loaded for that run alone unless the build names sbt-teq, sbt 2 and
  `sbt` on the `PATH` needed; a failure is a diagnostic on `build.sbt`), the file written at the
  build's root for you to commit; it never runs again to refresh one;
- a root without any export that is no sbt build is one session of every `.scala` file under it,
  on the lean std without a class path.

`workspace/symbol` answers from the sessions running; open a file of a project first to have it
answer from that project. A file of several sessions (a shared source root) is answered by each
of them together.
