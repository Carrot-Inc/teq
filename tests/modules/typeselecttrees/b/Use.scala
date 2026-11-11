package tsb

import tsa.*

object IntBox extends Box:
  type T = Int
  def get: Int = 41

@main def run(): Unit =
  println(Paths.twice(3))
  println(Paths.k(K(5)))
  println(Paths.len(List(1, 2)))
  println(Paths.inc(9))
  println(Paths.got(IntBox) + 1)
  println(Paths.plain(List(1, 2, 3)))
  println(Paths.rooted(List(1, 2, 3, 4)))
  println(Paths.deep(5) + 2)
  println(Paths.plainDeep(List(1)))
