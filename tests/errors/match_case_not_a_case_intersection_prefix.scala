// expect: 10:28: error: this match type case is not a match case once its pattern is approximated: Nothing
// expect: 1 error found
// scalac 3.8.4 crashes on this program: the approximation of a member's type seen from the intersection `R[Int] &
// R[String]` leaves its match type's case no MatchCase application (`Nothing`, the failed distribution of the case
// pattern's range, derivedAppliedType 6805-6809), and reducing the match type over it throws scala.MatchError in
// MatchTypeCaseSpec.analyze (Types.scala:5427). teq reports that case as no match case and types the match type no
// further.
class Inv[X]
class R[K]:
  def get: Inv[Int match { case K => String }] = new Inv
def f(r: R[Int] & R[String]) = r.get
@main def main(): Unit = println("ok")
