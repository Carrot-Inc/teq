package sab

import saa.*

@main def run(): Unit =
  val p = Pipeline(List(Ops.twice, Ops.plain, Ops.offset(10)))
  println(p.apply(3))
  println(Ops.label.text(4))
  println(p.tagged.text(1))
