package sb

import sa.*

object Use:
  def area(s: Shape): Double = s match
    case Circle(r) => 3.0 * r * r
    case Square(side) => side * side
    case Dot => 0.0
  def eval(e: Expr): Int = e match
    case Expr.Num(n) => n
    case Expr.Add(l, r) => eval(l) + eval(r)
  def main(args: Array[String]): Unit =
    println(area(Circle(1.0)) + area(Dot))
    println(eval(Expr.Add(Expr.Num(1), Expr.Num(2))))
