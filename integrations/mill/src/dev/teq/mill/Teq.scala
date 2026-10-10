package dev.teq.mill

import mill.*
import mill.api.{BuildCtx, Evaluator}

/** mill-teq's external module (docs/TOOLING.md, "The export of a Mill build"), which a build runs without
  * naming it in its scripts:
  *
  * {{{
  * mill --import mvn:build.teq::mill-teq::<version> dev.teq.mill.Teq/teqExportAll
  * }}}
  *
  * `teqExportAll` writes `teq.lock` (`Export`), under the build's `target/teq/`, or at its root with the
  * launchers beside it when `teq.mill.json` says `"buildTool": true`. */
object Teq extends mill.api.ExternalModule:
  lazy val millDiscover = mill.api.Discover[this.type]

  /** Writes teq.lock from what Mill resolves of the build's modules, compiling nothing. */
  def teqExportAll(evaluator: Evaluator) = Task.Command(exclusive = true) {
    val log = Task.log
    Export.run(evaluator, BuildCtx.workspaceRoot, line => log.info(line), line => log.warn(line)) match
      case Right(file) => mill.api.Result.Success(file.toString)
      case Left(why) => mill.api.Result.Failure(why)
  }
