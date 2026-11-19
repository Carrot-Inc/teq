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
// expect: 10:35: error: Found:    Char
// expect+ Required: Elem[X]
// expect+
// expect+ Note: a match type could not be fully reduced:
// expect+
// expect+   trying to reduce  Elem[X]
// expect+   failed since selector X
// expect+   does not match  case String => Char
// expect+   and cannot be shown to be disjoint from it either.
// expect+   Therefore, reduction cannot advance to the remaining case
// expect+
// expect+     case Array[t] => t
// expect: 11:23: error: Found:    t
// expect+ Required: Elem[X]
// expect+
// expect+ Note: a match type could not be fully reduced:
// expect+
// expect+   trying to reduce  Elem[X]
// expect+   failed since selector X
// expect+   does not match  case String => Char
// expect+   and cannot be shown to be disjoint from it either.
// expect+   Therefore, reduction cannot advance to the remaining case
// expect+
// expect+     case Array[t] => t
// expect: 13:27: error: Found:    Char
// expect+ Required: Elem[X]
// expect+
// expect+ Note: a match type could not be fully reduced:
// expect+
// expect+   trying to reduce  Elem[X]
// expect+   failed since selector X
// expect+   does not match  case String => Char
// expect+   and cannot be shown to be disjoint from it either.
// expect+   Therefore, reduction cannot advance to the remaining case
// expect+
// expect+     case Array[t] => t
// expect: 14:23: error: Found:    t
// expect+ Required: Elem[X]
// expect+
// expect+ Note: a match type could not be fully reduced:
// expect+
// expect+   trying to reduce  Elem[X]
// expect+   failed since selector X
// expect+   does not match  case String => Char
// expect+   and cannot be shown to be disjoint from it either.
// expect+   Therefore, reduction cannot advance to the remaining case
// expect+
// expect+     case Array[t] => t
// expect: 16:21: error: Found:    Char
// expect+ Required: Elem[X]
// expect+
// expect+ Note: a match type could not be fully reduced:
// expect+
// expect+   trying to reduce  Elem[X]
// expect+   failed since selector X
// expect+   does not match  case String => Char
// expect+   and cannot be shown to be disjoint from it either.
// expect+   Therefore, reduction cannot advance to the remaining case
// expect+
// expect+     case Array[t] => t
// expect: 18:21: error: Found:    Int
// expect+ Required: Char
