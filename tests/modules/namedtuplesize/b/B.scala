package ntb

import nta.*

@main def run(): Unit =
  val n: 2 = count((a = 1, b = "x"))
  println(n)
  println(count((left = (1, "a"), right = List(2), z = 0)))
