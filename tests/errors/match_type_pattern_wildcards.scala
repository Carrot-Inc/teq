class B[+T <: AnyVal]
type M[X] = X match
  case B[_] => Int
  case List[_] => Long
type H[X <: Tuple] = X match
  case x *: _ => x
val w: M[Option[Int]] = 2
def h: H[Int] = 1
// scalac 3.8.4 binds a match type pattern's wildcard to a pattern-bound type of its own, which its messages print `_`.
// expect: case B[_] => Int
// expect: case List[_] => Long
// expect: case x *: _ => x
