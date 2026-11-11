// jars: scala-library defaults-lib
// targets: js interp
// Classes of a jar whose parent is a constructor call without type arguments
// (tests/support/defaults_lib.scala's `HeldHook extends HeldInput(input)`): a parent written as a call is typed
// as one, its type arguments inferred, and only a parent that is a type alone has its kind checked
// (Typer.typedParent).
import defaultslib.*

@main def main(): Unit =
  val one = new HeldHook(1, "a")
  println(one.hook + one.input)
  val two = new HeldTwo(2, "b", List(3))
  println(two.hook + two.input + two.second.head)
