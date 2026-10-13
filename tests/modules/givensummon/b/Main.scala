package gsb

import gsa.{given, *}

@main def run(): Unit =
  println(summon[T].n)
  println(summon[T] eq top)
