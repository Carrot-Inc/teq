package motb

import mota.*

@main def run(): Unit =
  val h = Holder()
  println(h.In.toString)
  println(h.Plain.toString.nonEmpty)
