// A lambda whose body is a match is a partial function wherever the expected type gives it one,
// its match not checked for exhaustivity; a guard is one of its cases' conditions; a match
// nested in a case's body is an ordinary match, checked.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
case class Item(id: String, shape: Shape)

type ShapePf = PartialFunction[Shape, Int]
class Holder[T](val pf: PartialFunction[Shape, T])
def byName(pf: => PartialFunction[Shape, Int]): Boolean = pf.isDefinedAt(Square(1))
def same[T](t: T): T = t
def defined[A](a: A)(pf: PartialFunction[A, Int]): Boolean = pf.isDefinedAt(a)

object Main:
  def main(args: Array[String]): Unit =
    val declared: PartialFunction[Shape, Int] = s => s match
      case Circle(r) => r
    val ascribed = (s => s match { case Circle(r) => r }): PartialFunction[Shape, Int]
    val typedParam = ((s: Shape) => s match { case Circle(r) => r }): PartialFunction[Shape, Int]
    val aliased: ShapePf = s => s match { case Square(n) => n }
    val inferred: PartialFunction[Shape, Int] = same(s => s match { case Circle(r) => r })
    println(List(declared, ascribed, typedParam, aliased, inferred).map(_.isDefinedAt(Square(1))))
    println(byName(s => s match { case Circle(r) => r }))
    println(same[PartialFunction[Shape, Int]](s => s match { case Circle(r) => r }).isDefinedAt(Square(1)))
    println(defined(Square(1): Shape)(s => s match { case Circle(r) => r }))
    println(Holder(s => s match { case Circle(r) => r.toString }).pf.isDefinedAt(Square(1)))
    val guarded: PartialFunction[Item, Int] = it => it.shape match
      case Circle(r) if r > 2 && it.id != "skip" => r
    println(guarded.isDefinedAt(Item("a", Circle(3))))
    val nested: PartialFunction[Item, Int] = it => it match
      case Item(id, s) if id != "c" => s match
        case Circle(r) => r
    println(nested.isDefinedAt(Item("a", Square(1))))
