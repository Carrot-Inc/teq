// expect: 17:36: error: unreachable case: type String is not a subclass of class Bar
// expect: 18:38: error: unreachable case: type Box[Int] is not a subclass of class Bar
// expect: 2 errors found
// A constructor pattern's class test is judged as a typed pattern's, by dotty's
// `TypeTestsCasts.checkSensical` at erasure: a final scrutinee class that does not derive from
// the pattern's class, or two unrelated classes, is an unreachable case; a primitive scrutinee,
// an abstract type constructor's application (`M[T]`, its class `Any`'s) and an opaque type
// seen through its bound are judged as their classes allow, never refused as unrelated types.
case class Bar[T](x: T)
final class Box[T](val y: T)
sealed trait Foo
case class FooA() extends Foo
object O:
  opaque type OpaqueFoo <: Foo = Foo
  def make(): OpaqueFoo = FooA()
def g1(x: Int) = x match { case Bar(y) => y }
def g2(x: String) = x match { case Bar(y) => y }
def g3(x: Box[Int]) = x match { case Bar(y) => y }
def g4[M[_], T](x: M[T]) = x match { case Bar(y) => y }
def g5 = O.make() match { case FooA() => 1 }
