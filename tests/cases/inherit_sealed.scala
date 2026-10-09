sealed abstract class Expr
final case class Num(value: Int) extends Expr
final case class Add(left: Expr, right: Expr) extends Expr
final case class Neg(inner: Expr) extends Expr
case object Zero extends Expr

sealed abstract class Shape(val sides: Int):
  def area: Int
object Shape:
  final case class Square(side: Int) extends Shape(4):
    def area = side * side
  final case class Tri(base: Int, height: Int) extends Shape(3):
    def area = base * height / 2
  case object Dot extends Shape(0):
    def area = 0

def eval(e: Expr): Int = e match
  case Num(v) => v
  case Add(l, r) => eval(l) + eval(r)
  case Neg(i) => -eval(i)
  case Zero => 0

def describe(s: Shape): String = s match
  case Shape.Square(side) => s"square $side"
  case t: Shape.Tri => s"tri ${t.base} ${t.sides}"
  case Shape.Dot => "dot"

@main def run() =
  println(eval(Add(Num(1), Neg(Add(Num(2), Zero)))))
  println(describe(Shape.Square(2)))
  println(describe(Shape.Tri(2, 3)))
  println(describe(Shape.Dot))
  val shapes: List[Shape] = List(Shape.Square(1), Shape.Dot)
  println(shapes.map(_.sides))
  println(shapes.map(_.area))
  println(shapes)
  println(Add(Num(1), Zero) == Add(Num(1), Zero))
  println(Add(Num(1), Zero).hashCode == Add(Num(1), Zero).hashCode)
