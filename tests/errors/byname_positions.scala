// expect: 5:8: error: Implementation restriction: PolyFunction apply must have exactly one parameter list and optionally type arguments. No by-name nor varags are allowed.
// expect: 6:22: error: a by-name type can only be the type of a parameter
// A by-name parameter is left to monomorphic function types: scalac rejects one in a polymorphic function type
// (tests/neg/i21652.scala) and as a self type (a syntax error there, E040).
def k: [A] => (=> A) => A = [A] => a => a
trait X1[T] { self: (=> String) => }
