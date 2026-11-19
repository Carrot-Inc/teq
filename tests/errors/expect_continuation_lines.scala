// A diagnostic of several lines held whole: the type mismatch's note on a match type that does not reduce, its
// blank lines and its indented lines each a `// expect+` line, line for line after the message's first line.
// scalac gives the same note after "Found:    (false : Boolean)" / "Required: IsX[Int]" at the argument.
type IsX[T] = T match
  case 42 => true
  case _ => false
def k(a: Int): IsX[a.type] = ???
def widened = { val q: IsX[Int] = k(1) }
// expect: 8:35: error: Found:    IsX[1]
// expect+ Required: IsX[Int]
// expect+
// expect+ Note: a match type could not be fully reduced:
// expect+
// expect+   trying to reduce  IsX[Int]
// expect+   failed since selector Int
// expect+   does not match  case 42 => true
// expect+   and cannot be shown to be disjoint from it either.
// expect+   Therefore, reduction cannot advance to the remaining case
// expect+
// expect+     case _ => false
