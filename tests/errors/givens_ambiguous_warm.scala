// expect: ambiguous given instances for TC[Int]: first, second
// expect: ambiguous given instances for TC[Both]: viaString, viaLong
// expect: no given instance of type TC[String]
// expect: no given instance of type Eq[Circle]
// expect: no given instance of type Eq[Some[Int]]
// expect: given_Show_T is already defined
package demo

trait TC[A]:
  def name: String
final class Impl[A](val name: String) extends TC[A]

trait TextKey[T]
trait LongKey[T]
final class TextKeyOf[T] extends TextKey[T]
final class LongKeyOf[T] extends LongKey[T]

final class Both(v: Int)
object Both:
  given TextKey[Both] = TextKeyOf()
  given LongKey[Both] = LongKeyOf()

object Instances:
  given first: TC[Int] = Impl("first")
  given second: TC[Int] = Impl("second")
  given viaString[T](using TextKey[T]): TC[T] = Impl("string id")
  given viaLong[T](using LongKey[T]): TC[T] = Impl("long id")
  given nested[T](using TC[T]): TC[List[T]] = Impl("nested")

import Instances.given

// The searches below run once where one candidate is in reach, so the ambiguous ones meet what
// the given search keeps (the fits, the declared heads, the package level), and name the same.
object Warm:
  import Instances.first
  val tc = summon[TC[Int]].name
  val nested = summon[TC[List[Int]]].name

trait Eq[A]
final class EqImpl[A] extends Eq[A]
object Eq:
  given [A](using Eq[A]): Eq[Option[A]] = EqImpl()
  given Eq[Int] = EqImpl()

sealed trait Shape
object Shape:
  final case class Circle(r: Int) extends Shape
  given Eq[Shape] = EqImpl()

def same[A](a: A, b: A)(using Eq[A]): Boolean = a == b

trait Show[T]:
  def show(t: T): String

// Structural givens are classes, which cannot share a name the way alias givens can.
given [T](using TextKey[T]): Show[T] with
  def show(t: T): String = "string id"
given [T](using LongKey[T]): Show[T] with
  def show(t: T): String = "long id"

@main def run(): Unit =
  println(summon[TC[Int]].name)
  println(summon[TC[Both]].name)
  println(summon[TC[List[Int]]].name)
  println(summon[TC[String]].name)
  // A type argument that the value arguments determine is not widened to find an instance.
  println(same(Shape.Circle(1), Shape.Circle(1)))
  println(same(Some(1), Some(1)))
