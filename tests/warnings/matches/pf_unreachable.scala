// A pattern-matching anonymous function's cases are checked for reachability as any match's,
// written as a lambda or as `{ case ... }`, through an overload too.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

object Main:
  def main(args: Array[String]): Unit =
    val twice: PartialFunction[Shape, Int] = s => s match
      case Circle(r) => r
      case Circle(q) => q
    println(twice.isDefinedAt(Square(1)))
    val cases: PartialFunction[Shape, Int] =
      case Circle(r) => r
      case Circle(q) => q
    println(cases.isDefinedAt(Square(1)))
    val m = Map(1 -> Circle(1))
    println(m.collect(kv => kv._2 match
      case Circle(r) => r
      case Circle(q) => q))
    val wildcard: PartialFunction[Shape, Int] = s => s match
      case Circle(r) => r
      case Square(n) => n
      case _ => 0
    println(wildcard.isDefinedAt(Square(1)))
