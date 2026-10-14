package xviews.fide

// lila's views/fide.scala: an object's export, whose path's lookup resolves the file's imports, and
// a top-level export through an inferred lazy val that needs them. The imports' qualifiers reach
// the file's own package, whose forwarders, this file's among them, a lookup makes: the file whose
// imports are being resolved is passed over, as dotty's `computeMembersNamed` passes over a
// package object under completion (before, `helpers` was not found).
import xlib.UiEnv.*

lazy val ui = xlib.FideUi(helpers)
export ui.federation

object player:
  lazy val playerUi = xlib.PlayerUi(helpers)
  export playerUi.index

@main def run(): Unit =
  println(federation(1))
  println(player.index)
