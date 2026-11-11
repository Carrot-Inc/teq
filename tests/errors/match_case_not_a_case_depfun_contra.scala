// expect: 11:39: error: this match type case is not a match case once its pattern is approximated: Nothing & MatchCase[?, String]
// expect: 1 error found
// scalac 3.8.4 crashes on this program: the approximation of a dependent function type's result avoiding its
// parameter, under `Contra` leaves its match type's case no MatchCase application (`Nothing & MatchCase[..]`, the
// failed distribution of the case pattern's range, derivedAppliedType 6805-6809), and reducing the match type over it
// throws scala.MatchError in MatchTypeCaseSpec.analyze (Types.scala:5427). teq reports that case as no match case and
// types the match type no further (the coordinator's ruling of 27 Oct; row C.11 of rows-9c-closure.md).
class Contra[-X]
class A:
  type T
def h(f: (c: A) => Contra[Int match { case c.T => String }]): A => Contra[String] = f
@main def main(): Unit = println("ok")
