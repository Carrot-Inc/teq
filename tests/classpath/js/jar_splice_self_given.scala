// jars: scala-library defaults-lib
// targets: js interp
// A jar's macro whose quote is spliced inside objects nested in a trait that has its `Quotes` through its self
// type (tests/support/defaults_lib.scala's `QuotedExprs`): the body types as scalac typed it, and the macro runs.
import defaultslib.*

@main def main(): Unit =
  println(Sized.size(List(1, 2, 3)))
  println(Sized.size(Set.empty[Int]))
