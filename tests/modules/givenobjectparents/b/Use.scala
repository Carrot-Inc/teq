package gopu

import gop.*

// scalac 3.8.4 prints `p5`, `p6`, `s5` and `s61`.
@main def run(): Unit =
  val c = C[Int]()
  locally:
    import c.given
    println(summon[P[Int]].p(5))
  locally:
    import O.given
    println(summon[P[Int]].p(6))
  val d = D[Int]()
  locally:
    import d.given
    println(summon[S[Int]].s(5))
  locally:
    import E.given
    println(summon[S[Int]].s(6) + summon[Q & S[Int]].x)
