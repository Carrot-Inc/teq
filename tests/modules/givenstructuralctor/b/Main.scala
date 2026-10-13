package gscb

import gsca.*

@main def run(): Unit =
  println(O.t.tag + O.t.extra)
  import O.given
  println(summon[Parent].n)
  given Int = 4
  println(summon[Show[String]].s)
  println(P.withCount(using 5).doubled)
