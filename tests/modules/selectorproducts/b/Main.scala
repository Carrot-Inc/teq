package spb

import spa.*

@main def run() =
  val b: Has[String] = Nest.Box("ok")
  println(b._1)
  println(Nest.Box(3)._1 + 1)
  val p = Nest.Pair(4, "four")
  println(s"${p._1} ${p._2}")
