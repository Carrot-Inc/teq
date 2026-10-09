package rwb

import rwa.Patterns

def same(x: Any, y: Any): Boolean = x == y

@main def run(): Unit =
  println(Patterns.pair(List(1, 2)))
  println(Patterns.parts("a,b,c"))
  println(same(1, 1L))
  println(" x ".trim)
