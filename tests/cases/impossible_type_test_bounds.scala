// A type test over an abstract type is judged by the class of its whole bound, as scalac 3.8.4's
// `classSymbol` reads it: an intersection bound whose operands derive neither from the other
// (`T <: Parent & Mark`, in either order, a type parameter or a type member) and a union bound
// (`T <: A | B`, whose join is `AnyRef`) give no class to judge, so these build and run. Their
// cases are unreachable, which scalac warns (E030, tests/warnings/matches/space_open_world.scala).
class Parent
trait Mark
class Other
class Both extends Parent with Mark
@annotation.nowarn
def parentFirst[T <: Parent & Mark](x: T): Int = x match
  case _: Other => 1
  case _ => 0
@annotation.nowarn
def markFirst[T <: Mark & Parent](x: T): Int = x match
  case _: Other => 1
  case _ => 0
trait Holder:
  type T <: Parent & Mark
  @annotation.nowarn
  def member(x: T): Int = x match
    case _: Other => 1
    case _ => 0
object BothHolder extends Holder:
  type T = Both
class A
class B
class C
@annotation.nowarn
def unionBound[T <: A | B](x: T): Int = x match
  case _: C => 1
  case _ => 0

@main def run(): Unit =
  println(List(parentFirst(Both()), markFirst(Both()), BothHolder.member(Both()), unionBound(A())))
