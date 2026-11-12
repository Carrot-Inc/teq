// A dependent method's result holding a match type in an invariant argument, the case's pattern its parameter's member,
// applied to an unstable argument (the closure adversary of the pass on 861d8acf1, round 22, m30): dotty approximates
// `a.T` for `new A` (a range), the case's distribution fails and the lower end is a match type over a case that is no
// `MatchCase` (`Nothing`); scalac 3.8.4 crashes with `scala.MatchError` in `MatchTypeCaseSpec.analyze` (Types.scala:5427),
// its outcome the .expected. Where teq makes such a case it is to report it
// (`tests/errors/match_case_not_a_case_*`); teq never makes it here, reading the unstable argument's member as the
// projection `A#T`, and rejects the program at `def show: Int = v` alone.
class Inv[X]
class A:
  type T
def m(a: A): Inv[Int match { case a.T => String }] = new Inv
def v = m(new A)
def show: Int = v
@main def main(): Unit = println("ok")
