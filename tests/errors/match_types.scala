// expect: 26:23: error: Found:    1⏎Required: Code[3]⏎⏎Note: a match type could not be fully reduced:⏎⏎  trying to reduce  Code[3]⏎  failed since selector 3⏎  matches none of the cases⏎⏎    case 1 => Int⏎    case 2 => String
// expect: failed since selector 3
// expect: matches none of the cases
// expect: 27:25: error: Found:    1⏎Required: Code[Int]⏎⏎Note: a match type could not be fully reduced:⏎⏎  trying to reduce  Code[Int]⏎  failed since selector Int⏎  does not match  case 1 => Int⏎  and cannot be shown to be disjoint from it either.⏎  Therefore, reduction cannot advance to the remaining case⏎⏎    case 2 => String
// expect: does not match  case 1 => Int
// expect: and cannot be shown to be disjoint from it either.
// expect: 28:37: error: Found:    1⏎Required: Area[Shape]⏎⏎Note: a match type could not be fully reduced:⏎⏎  trying to reduce  Area[Shape]⏎  failed since selector Shape⏎  does not match  case Circle => Double⏎  and cannot be shown to be disjoint from it either.⏎  Therefore, reduction cannot advance to the remaining case⏎⏎    case Square => Int
// expect: 31:25: error: Found:    true⏎Required: Elem[Holder.this.X]⏎⏎Note: a match type could not be fully reduced:⏎⏎  trying to reduce  Elem[Holder.this.X]⏎  failed since selector Holder.this.X⏎  does not uniquely determine parameter t in⏎    case List[t] => t⏎  The computed bounds for the parameter are:⏎    t <: Boolean
// expect: does not uniquely determine parameter t
// `Tuple.Head[Int]`'s bound is not reported at 32:24: PostTyper does not run after the typer's errors above.
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
