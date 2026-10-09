// jars: fixtures
// targets: js jvm interp
// An extension whose first clause is a using clause, called from a body of the jar (`W6Use.viaLib`, the shape of
// microlibs' `MacroEnv.inlineConstOrNull`): its receiver is the clause after the using one, and the jar's call
// `tagged(c)(t)[Int](ev)` takes its type argument first; the top-level extension is a member of the jar's
// package object, called on it. The expectation is scalac 3.8.4's, run over tests/tasty/src/wave6.scala and
// this file.
import fix.wave6.*

@main def main(): Unit =
  given c: W6Ctx = new W6Ctx
  println(W6Use.viaLib(c.term("a")))
