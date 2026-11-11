package vcb

import vca.*

// Nothing here makes a `W`, so `W` is completed from module a's products only for the emitter's erasure of `applied`.
object L:
  def applied(x: W): String = "applied"

@main def run(): Unit =
  println(L.applied(null.asInstanceOf[W]))
