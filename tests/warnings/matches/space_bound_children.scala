// A type pattern over an abstract type bounded by a sealed type covers the bound's children (dotty's
// SpaceEngine decomposing through the bound), so a later case of a child it covers is unreachable.
sealed trait A
case class B() extends A
case class C() extends A
def test[T <: A](x: A): Int = x match
  case B() => 1
  case _: T => 2
  case C() => 3
