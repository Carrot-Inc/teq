package gopb

import gopa.*

@main def run(): Unit =
  println(A.insideExternal)
  println(A.insideInternal)
  println(cmp(A.external(1), A.external(2)))
  println(cmp(A.internal(1), A.internal(2)))
  println(B.insideApplied)
  println(B.insideBare)
  println(cmp(B.applied(1), B.applied(2)))
