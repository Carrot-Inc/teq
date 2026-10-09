package msb

import msa.Mac

@main def run(): Unit =
  println(Mac.joined("y"))
  val c = true
  println(Mac.kinds("x", "y" -> c, ("z", c), 1))
