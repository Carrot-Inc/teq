package gpb

import gpa.{given, *}

@main def run(): Unit =
  println(t.self eq t)
  println(O.u.self eq O.u)
  println(summon[T] eq t)
