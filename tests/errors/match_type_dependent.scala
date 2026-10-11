// A match typed against a match type is typed dependently only where it has its shape
// (tests/cases/match_type_dependent.scala): a guard, another pattern type or another number of
// cases leaves the body against the unreduced match type, which it does not conform to, as under
// scalac; a body of the wrong type is refused against its case's body.
type Elem[X] = X match
  case String => Char
  case Array[t] => t

def guarded[X](x: X): Elem[X] = x match
  case s: String if s.nonEmpty => s.charAt(0)
  case a: Array[t] => a(0)
def wider[X](x: X): Elem[X] = x match
  case s: CharSequence => s.charAt(0)
  case a: Array[t] => a(0)
def fewer[X](x: X): Elem[X] = x match
  case s: String => s.charAt(0)
def wrongBody[X](x: X): Elem[X] = x match
  case s: String => s.length
  case a: Array[t] => a(0)
// expect: 10:35: error: type mismatch: found Char, required Elem[X]
// expect: 11:23: error: type mismatch: found t, required Elem[X]
// expect: 13:27: error: type mismatch: found Char, required Elem[X]
// expect: 14:23: error: type mismatch: found t, required Elem[X]
// expect: 16:21: error: type mismatch: found Char, required Elem[X]
// expect: 18:21: error: type mismatch: found Int, required Char
