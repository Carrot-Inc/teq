// expect: 12:36: error: this match type case is not a match case once its pattern is approximated: Nothing
// expect: 1 error found
// scalac 3.8.4 crashes in MatchTypeCaseSpec.analyze (Types.scala:5427), with no diagnostic before its MatchError.
// Whole-file crash log:
// (scalac's stack trace of the MatchError; the log is not kept in the tree)
// The expectation is teq's own, scalac having no diagnostic to record: it crashes.
// Require the primary diagnostic at 12:36, a failing compilation and recovery without a crash;
// no secondary diagnostic is required.
class Inv[X]
class A:
  type T
def h(f: (c: A) => Inv[Int match { case c.T => String }]): Int = f.andThen(x => x)
@main def main(): Unit = println("ok")
