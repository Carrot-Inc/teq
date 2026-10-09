// expect: 26:23: error: type mismatch: found Int, required Code[3]
// expect: failed since selector 3
// expect: matches none of the cases
// expect: 27:25: error: type mismatch: found Int, required Code[Int]
// expect: does not match  case 1 => Int
// expect: and cannot be shown to be disjoint from it either.
// expect: 28:37: error: type mismatch: found Int, required Area[Shape]
// expect: 31:25: error: type mismatch: found Boolean, required Elem[Holder.this.X]
// expect: does not uniquely determine parameter t
// expect: 32:24: error: type argument Int does not conform to upper bound Tuple
// expect: 24:6: error: Recursion limit exceeded.
// expect: Maybe there is an illegal cyclic reference?
type Code[X] = X match
  case 1 => Int
  case 2 => String
sealed trait Shape
final class Circle extends Shape
final class Square extends Shape
type Area[S] = S match
  case Circle => Double
  case Square => Int
type Elem[X] = X match
  case List[t] => t
type Loop[X] = X match
  case Int => Loop[X]
def wrong1: Code[3] = 1
def wrong2: Code[Int] = 1
def wrong3(s: Shape): Area[Shape] = 1
trait Holder:
  type X <: List[Boolean]
  def wrong4: Elem[X] = true
def wrong5: Tuple.Head[Int] = 1
def wrong6: Loop[Int] = 1
