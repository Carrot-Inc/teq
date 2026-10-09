package glb

import gla.Concat

def both[A, B, AB](a: A, b: B)(using c: Concat[A, B, AB]): AB = c.join(a, b)

@main def run(): Unit =
  println(both("x", 1))
  println(both((), ()))
