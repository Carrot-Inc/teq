// The type member a parentless refinement operand fixes holds for a path through it
// (tests/cases/refinement_operand.scala): `a.T` of an `A & { type T = Int }` is no `String`.
trait A:
  type T
def f(a: A & { type T = Int })(x: a.T): String = x
// expect: 5:50: error: type mismatch: found Int, required String
