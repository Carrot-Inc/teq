// A lambda whose body is a match is a partial function wherever the expected type gives it
// one: a value's declared type, an ascription, an alias, a by-name parameter, a type argument
// instantiated by the expected type, a later parameter list, a constructor's parameter. Its
// cases decide `isDefinedAt`; `apply` outside them throws `MatchError`, and `applyOrElse` takes
// the default (dotc's `ExpandSAMs`).
import scala.util.Try

sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

type ShapePf = PartialFunction[Shape, Int]

class Holder[T](val pf: PartialFunction[Shape, T])

def byName(pf: => PartialFunction[Shape, Int]): Boolean = pf.isDefinedAt(Square(1))
def same[T](t: T): T = t
def defined[A](a: A)(pf: PartialFunction[A, Int]): Boolean = pf.isDefinedAt(a)
def lifted[A, B](pf: PartialFunction[A, B]): A => Option[B] = pf.lift

object Main:
  def main(args: Array[String]): Unit =
    val declared: PartialFunction[Shape, Int] = s => s match
      case Circle(r) => r
    println(declared.isDefinedAt(Circle(1)))
    println(declared.isDefinedAt(Square(1)))
    println(declared.applyOrElse(Square(1), _ => -1))
    println(declared.lift(Circle(7)))
    println(Try(declared(Square(1))).failed.get.getClass.getSimpleName)
    val ascribed = (s => s match { case Circle(r) => r }): PartialFunction[Shape, Int]
    println(ascribed.isDefinedAt(Square(1)))
    val typedParam = ((s: Shape) => s match { case Circle(r) => r }): PartialFunction[Shape, Int]
    println(typedParam.isDefinedAt(Square(1)))
    val aliased: ShapePf = s => s match { case Square(n) => n }
    println(aliased.isDefinedAt(Circle(1)))
    println(byName(s => s match { case Circle(r) => r }))
    println(same[PartialFunction[Shape, Int]](s => s match { case Circle(r) => r }).isDefinedAt(Square(1)))
    val inferred: PartialFunction[Shape, Int] = same(s => s match { case Circle(r) => r })
    println(inferred.isDefinedAt(Square(1)))
    println(defined(Square(1): Shape)(s => s match { case Circle(r) => r }))
    println(lifted((s: Shape) => s match { case Circle(r) => r })(Square(2)))
    println(Holder(s => s match { case Circle(r) => r.toString }).pf.isDefinedAt(Square(1)))
    val total: PartialFunction[Shape, Int] = s => s.hashCode & 0
    println(total.isDefinedAt(Square(1)))
    val typedBody: PartialFunction[Int, Int] = x => (x match { case 1 => 2 }): Int
    println(typedBody.isDefinedAt(3))
