package vcb

import vca.*

// Nothing here makes a `W`: `W` is needed from module a's products only for the erasure of `applied`.
object L:
  def applied(x: W): String = "applied"

@main def run(): Unit =
  println(L.applied(null.asInstanceOf[W]))
