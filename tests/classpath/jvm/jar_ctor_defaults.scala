// jars: scala-library defaults-lib
// std: scala-library
// A jar class's constructor defaults (tests/support/defaults_lib.scala, circe-yaml's `Printer`
// in the application's API docs): scalac's getters take the clauses before the default's only,
// on the companion, forwarded statically from a top-level class and not from a nested one.
import defaultslib.*

@main def main(): Unit =
  println(Printer(dropNullKeys = true, preserveOrder = true))
  println(Printer(indent = 4))
  println(new Printer(name = "q"))
  val box = new Box(1)()
  println((box.a, box.b, box.c))
  println(new Box(1, 2)().c)
  println(Holder.Inner(y = "z"))
