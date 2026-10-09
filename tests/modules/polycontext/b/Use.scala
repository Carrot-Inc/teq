package pcb

import pca.P

@main def run(): Unit =
  println(P.call(P.context))
  println(P.instantiate(P.context)(using 8))
  println(P.inferred(P.two))
  println(P.expected([A] => (as: List[A]) => as))
