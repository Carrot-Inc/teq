// A scrutinee that conforms to the pattern's type is tested as dotty's `TypeTestsCasts.transformTypeTest` tests it,
// on the pattern's erasure: a refined type by its parent's, an intersection by each side's, a union each side
// by its own. The JVM makes the class test where the scrutinee's type does not conform to the erasure (a generic
// class, an opaque alias of one inside its owner and outside, an array, a union's side) and a null test of the
// rest (tests/classfile.scala pins the bytecode); JavaScript makes the null test where the erasures conform, as
// Scala.js's optimizer does. The results are the same everywhere (`null` taken first, so that no case is
// unreachable but for it).
class Box[T]:
  def foo: Int = 1
trait Mark
class Plain
class Both extends Box[Int] with Mark
class PlainMark extends Plain with Mark
class V(val value: Int) extends AnyVal
class A
class B

object O:
  opaque type Hidden = Box[Int]
  def hide(b: Box[Int]): Hidden = b
  def inside(x: Hidden): Boolean = x match
    case null => false
    case _: Hidden => true

def refined(x: Box[Int] { def foo: Int }): Boolean = x match
  case null => false
  case _: Box[Int] { def foo: Int } => true
def intersect(x: Box[Int] & Mark): Boolean = x match
  case null => false
  case _: (Box[Int] & Mark) => true
def marks(x: Plain & Mark): Boolean = x match
  case null => false
  case _: (Plain & Mark) => true

// Outside its owner `null` is no value of the opaque type to compare with.
def opaqueTest(x: O.Hidden): Boolean = x match
  case _: O.Hidden => true
def values(x: Array[V]): Boolean = x match
  case null => false
  case _: Array[V] => true
def union(x: A | B): Boolean = x match
  case null => false
  case _: (A | B) => true

@main def main(): Unit =
  println(refined(new Box[Int]))
  println(refined(null))
  println(intersect(new Both))
  println(intersect(null))
  println(marks(new PlainMark))
  println(marks(null))
  println(O.inside(O.hide(new Box[Int])))
  println(O.inside(O.hide(null)))
  println(opaqueTest(O.hide(new Box[Int])))
  println(values(Array(V(1))))
  println(values(null))
  println(union(new B))
  println(union(null))
