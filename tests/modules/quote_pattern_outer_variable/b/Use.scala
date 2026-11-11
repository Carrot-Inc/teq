package qpov.use

import qpov.lib.Same

@main def run(): Unit =
  println(Same.element[List[Int], Option[Int]])
  println(Same.element[List[Int], Option[String]])
  val both = Same.composed[Int, String, Int]((x: Int) => x.toString, (s: String) => s.length)
  println(both(1234))
