// A member whose type holds a match type in an invariant argument, the case's pattern the class's parameter, selected
// through an unstable prefix with a wildcard argument (the closure adversary of the pass on 861d8acf1, round 22):
// dotty's `asSeenFrom` maps `K` to the range of the wildcard (`AsSeenFromMap`, an `ApproximatingTypeMap`), which makes
// `Inv[>: Match(Any; Int; Nothing) <: Match(..)]` (Types.scala 6807-6809, 6862-6865); `Typer.simplify` of the
// selection's type reduces the lower end and `MatchTypeCaseSpec.analyze`'s `@unchecked` `MatchCase` extractor (5427)
// throws: scalac 3.8.4 crashes in the typer with `scala.MatchError` (its outcome, the .expected). teq substitutes the
// wildcard into the pattern (`Inv[Int match { case _ => String }]`) and accepts the program.
// Where teq makes a case the approximation leaves no match case, it is to report it
// (`tests/errors/match_case_not_a_case_*`); teq never makes that case here, its member type through the prefix
// substituting the wildcard where dotty's `AsSeenFromMap` ranges it, and the same holds once that map ranges it.
class Inv[X]
class R[K]:
  def get: Inv[Int match { case K => String }] = new Inv
def mk: R[?] = new R[Int]
def f = mk.get
@main def matchCaseRangeWildPrefix(): Unit = println("ok")
