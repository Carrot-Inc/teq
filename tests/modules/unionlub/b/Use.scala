package ulb

import ula.*

@main def run(): Unit =
  println(L.call(null))
  println(L.pure(null))
  println(L.matched[String](null))
  println(L.nullable(null))
  println(L.pureNull(new PT))
  println(L.pureLate(new PZ))
  println(L.pureUnionNull(new TU1))
