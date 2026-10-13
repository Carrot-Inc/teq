package gtcb

import gtca.*

@main def run(): Unit =
  println(O.gc(using 42).n)
  given Int = 7
  println(summon[T](using O.gc).n)
