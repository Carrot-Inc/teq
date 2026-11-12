// expect: 12:36: error: this match type case is not a match case once its pattern is approximated: Nothing
// No erroneous-type follow-on is expected after the recovery diagnostic (HideNonSensicalMessages).
// expect: 1 error found
// scalac 3.8.4 crashes on this program: the approximation of a dependent function type's result avoiding its
// parameter (to `A => Int`, a mismatch as well) leaves its match type's case no MatchCase application (`Nothing`, the
// failed distribution of the case pattern's range, derivedAppliedType 6805-6809), and reducing the match type over it
// throws scala.MatchError in MatchTypeCaseSpec.analyze (Types.scala:5427). teq is to report that case as no match
// case and to type the match type no further.
class Inv[X]
class A:
  type T
def h(f: (c: A) => Inv[Int match { case c.T => String }]): A => Int = f
@main def main(): Unit = println("ok")
// No erroneous-type follow-on is expected after the
// recovery diagnostic (HideNonSensicalMessages).
