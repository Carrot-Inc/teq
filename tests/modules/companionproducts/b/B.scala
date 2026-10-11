package cpb

import cpa.*

// The companions of case classes of the upstream products, named or selected where a function
// is expected, are their `apply` (Typer.adaptNoArgsOther, `caseCompanion`).
@annotation.nowarn
@main def run(): Unit =
  val f: Int => Scope.C = Scope.C
  println(f(3).n)
  val g: (String, Int) => Scope.Pair[String] = Scope.Pair
  println(g("z", 2).n)
  println(List("a", "b").map(Top))
