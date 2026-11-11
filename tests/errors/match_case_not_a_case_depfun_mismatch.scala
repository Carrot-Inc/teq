// expect: 12:36: error: this match type case is not a match case once its pattern is approximated: Nothing
// HideNonSensicalMessages suppresses the erroneous-type follow-on after the recovery diagnostic.
// expect: 1 error found
// scalac 3.8.4 crashes on this program: the approximation of a dependent function type's result avoiding its
// parameter (to `A => Int`, a mismatch as well) leaves its match type's case no MatchCase application (`Nothing`, the
// failed distribution of the case pattern's range, derivedAppliedType 6805-6809), and reducing the match type over it
// throws scala.MatchError in MatchTypeCaseSpec.analyze (Types.scala:5427). teq reports that case as no match case and
// types the match type no further (the coordinator's ruling of 27 Oct; row C.11 of rows-9c-closure.md).
class Inv[X]
class A:
  type T
def h(f: (c: A) => Inv[Int match { case c.T => String }]): A => Int = f
@main def main(): Unit = println("ok")
// HideNonSensicalMessages suppresses the erroneous-type follow-on after the recovery diagnostic.
// TypeMismatch divergence owner (coordinator ruling, 2026-11-05): Typer.typedTypeCase / Worker::typed_type_case.
