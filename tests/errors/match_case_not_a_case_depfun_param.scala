// expect: 11:33: error: this match type case is not a match case once its pattern is approximated: Nothing & MatchCase[?, String]
// expect: 1 error found
// scalac 3.8.4 crashes on this program: the approximation of a dependent function type's result avoiding its
// parameter, the match type a parameter's type leaves its match type's case no MatchCase application (`Nothing &
// MatchCase[..]`, the failed distribution of the case pattern's range, derivedAppliedType 6805-6809), and reducing
// the match type over it throws scala.MatchError in MatchTypeCaseSpec.analyze (Types.scala:5427).
// teq is to report that case as no match case
// and to type the match type no further.
class A:
  type T
def h(f: (c: A) => (Int match { case c.T => String }) => Int): A => String => Int = f
@main def main(): Unit = println("ok")
