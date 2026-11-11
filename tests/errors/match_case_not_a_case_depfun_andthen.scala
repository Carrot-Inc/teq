// expect: 12:36: error: this match type case is not a match case once its pattern is approximated: Nothing
// expect: 1 error found
// scalac 3.8.4 crashes in MatchTypeCaseSpec.analyze (Types.scala:5427), with no diagnostic before its MatchError.
// Whole-file crash log:
// /home/agent/pi-runs/tmp/codex/cut-errors-head-c1/scalac-match_case_not_a_case_depfun_andthen.log
// Coordinator ruling, 2026-11-04 04:55 UTC: exception to the capture rule because scalac crashes.
// Require the primary diagnostic at 12:36, a failing compilation and recovery without a crash;
// the recovered case has ErrorType, so no secondary diagnostic is required.
class Inv[X]
class A:
  type T
def h(f: (c: A) => Inv[Int match { case c.T => String }]): Int = f.andThen(x => x)
@main def main(): Unit = println("ok")
