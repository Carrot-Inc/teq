// teq: --werror
// A partial function's block that ends in a match it holds in braces of its own, or in a match
// whose value has the type of a class the block defines, is no block ending in a match for
// scalac (the braces make a block, `ensureNoLocalRefs` an ascription): no E211, and cases that
// cover every value give no warning, so the program builds under --werror. Each function is
// defined everywhere.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

object Main:
  def main(args: Array[String]): Unit =
    val nested: PartialFunction[Shape, Any] = s => { val k = 1; { s match { case _ => 1 + k } } }
    val built: PartialFunction[Shape, Any] = s => {
      class C { override def toString = "C" }
      s match { case _ => new C }
    }
    val listed: PartialFunction[Shape, Any] = s => {
      class C { override def toString = "C" }
      s match
        case Circle(r) => List.fill(r)(new C)
        case Square(n) => Nil
    }
    val made: PartialFunction[Shape, Any] = s => {
      case class P(i: Int)
      s match
        case Circle(r) => P(r)
        case Square(n) => P(-n)
    }
    for f <- List(nested, built, listed, made) do
      println(s"${f.isDefinedAt(Square(2))} ${f(Square(2))} ${f(Circle(2))}")
