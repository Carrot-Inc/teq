//> using platform js
enum Shape:
  case Circle(r: Double)
  case Rect(w: Double, h: Double)
  case Empty

enum Color:
  case Red, Green, Blue

sealed trait Expr
case class Num(value: Int) extends Expr
case class Add(l: Expr, r: Expr) extends Expr
case class Mul(l: Expr, r: Expr) extends Expr
case class Neg(e: Expr) extends Expr

def eval(e: Expr): Int = e match
  case Num(v) => v
  case Add(l, r) => eval(l) + eval(r)
  case Mul(l, r) => eval(l) * eval(r)
  case Neg(inner) => -eval(inner)

def simplify(e: Expr): Expr = e match
  case Add(Num(0), r) => simplify(r)
  case Add(l, Num(0)) => simplify(l)
  case Mul(Num(1), r) => simplify(r)
  case Mul(l, Num(1)) => simplify(l)
  case Mul(Num(0), _) | Mul(_, Num(0)) => Num(0)
  case Add(l, r) => Add(simplify(l), simplify(r))
  case Mul(l, r) => Mul(simplify(l), simplify(r))
  case Neg(Neg(inner)) => simplify(inner)
  case other => other

def area(s: Shape): Double = s match
  case Shape.Circle(r) => 3.0 * r * r
  case Shape.Rect(w, h) => w * h
  case Shape.Empty => 0.0

def colorName(c: Color): String = c match
  case Color.Red => "red"
  case Color.Green => "green"
  case Color.Blue => "blue"

def classify(n: Int): String = n match
  case 0 => "zero"
  case 1 | 2 | 3 => "small"
  case x if x < 0 => "negative"
  case x if x % 2 == 0 => s"even $x"
  case _ => "odd"

def describe(pair: (Int, String)): String = pair match
  case (0, s) => s"zero with $s"
  case (n, "x") => s"$n with x"
  case whole @ (n, s) => s"$whole -> $n, $s"

def sumList(xs: List[Int]): Int = xs match
  case Nil => 0
  case h :: t => h + sumList(t)

def firstTwo(xs: List[String]): String = xs match
  case a :: b :: _ => a + b
  case a :: Nil => a
  case Nil => "none"

@main def run(): Unit =
  val e = Add(Num(1), Mul(Num(2), Num(3)))
  println(e)
  println(eval(e))
  println(simplify(Add(Num(0), Mul(Num(1), Neg(Neg(Num(7)))))))
  println(simplify(Mul(Num(0), Num(5))))
  println(area(Shape.Circle(2.0)))
  println(area(Shape.Rect(2.0, 3.5)))
  println(area(Shape.Empty))
  println(Shape.Circle(1.5))
  println(Shape.Empty)
  println(colorName(Color.Green))
  println(Color.Blue)
  println(Color.Blue.ordinal)
  println(Color.Red == Color.Red)
  println(Color.Red == Color.Blue)
  println(classify(0))
  println(classify(2))
  println(classify(-5))
  println(classify(10))
  println(classify(11))
  println(describe((0, "a")))
  println(describe((5, "x")))
  println(describe((5, "y")))
  println(sumList(List(1, 2, 3, 4)))
  println(firstTwo(List("a", "b", "c")))
  println(firstTwo(List("a")))
  println(firstTwo(Nil))
  println(Num(1) == Num(1))
  println(Num(1) == Num(2))
  println(Add(Num(1), Num(2)) == Add(Num(1), Num(2)))
  val n = Num(4)
  println(n.copy(value = 5))
  println(n.value)
