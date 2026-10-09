// A lambda that is no `{ case ... }` given to a method overloaded on a function and a partial
// function goes to the function, whose match is checked; a `{ case ... }` literal goes to the
// partial function, whose cases are not.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

object Over:
  def shape(f: Shape => Int): String = "fn " + f(Circle(1))
  def shape(pf: PartialFunction[Shape, Int]): String = "pf " + pf.isDefinedAt(Square(1))

class Built(val tag: String):
  def this(f: Shape => Int) = this("fn " + f(Circle(1)))
  def this(pf: PartialFunction[Shape, Int], d: DummyImplicit) = this("pf " + pf.isDefinedAt(Square(1)))

object Main:
  def main(args: Array[String]): Unit =
    println(Over.shape(s => s match { case Circle(r) => r }))
    println(Over.shape { s => s match { case Circle(r) => r } })
    println(Over.shape { case Circle(r) => r })
    println(Built(s => s match { case Circle(r) => r }).tag)
    println(Built({ case Circle(r) => r }, summon[DummyImplicit]).tag)
    val f: Shape => Int = s => s match
      case Circle(r) => r
    println(f(Circle(2)))
