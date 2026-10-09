package sa

sealed trait Shape
final case class Circle(r: Double) extends Shape
final case class Square(side: Double) extends Shape
case object Dot extends Shape

sealed abstract class Expr
object Expr:
  final case class Num(n: Int) extends Expr
  final case class Add(l: Expr, r: Expr) extends Expr
