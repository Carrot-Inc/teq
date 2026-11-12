// A block's value whose type holds a match type in an invariant argument,
// the case's pattern the block's local class: dotty's avoidance of `L`
// (`TypeOps.avoid`, an `ApproximatingTypeMap`) ranges the pattern, the case's
// distribution fails and the lower end is a match type over a case that is
// no `MatchCase` (`Nothing`); scalac 3.8.4 crashes with `scala.MatchError` in
// `MatchTypeCaseSpec.analyze` (Types.scala:5427), its outcome the .expected. Where
// teq makes such a case it is to report it (`tests/errors/match_case_not_a_case_*`).
class Inv[X](val x: Int)
def mk = { class L; new Inv[Int match { case L => String }](1) }
@main def main(): Unit =
  val v = mk
  val w: Int = v
  println(v.x)
