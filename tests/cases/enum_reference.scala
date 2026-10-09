import scala.collection.mutable

enum Color:
  case Red, Green, Blue

enum Rgb(val rgb: Int):
  case Red extends Rgb(0xFF0000)
  case Green extends Rgb(0x00FF00)
  case Blue extends Rgb(0x0000FF)
  case Mix(mix: Int) extends Rgb(mix)

enum Planet(mass: Double, radius: Double):
  private final val G = 6.67300e-11
  def surfaceGravity: Double = G * mass / (radius * radius)
  def surfaceWeight(otherMass: Double): Double = otherMass * surfaceGravity

  case Mercury extends Planet(3.303e+23, 2.4397e6)
  case Venus extends Planet(4.869e+24, 6.0518e6)
  case Earth extends Planet(Planet.earthMass, Planet.earthRadius)

object Planet:
  private final val earthMass = 5.976e+24
  private final val earthRadius = 6.37814e6

  def report(earthWeight: Double): List[String] =
    val mass = earthWeight / Earth.surfaceGravity
    values.toList.map(p => s"$p: ${(p.surfaceWeight(mass) * 100).toInt}")

  def heaviest: Planet = values.toList.maxBy(_.surfaceGravity)

enum Opt[+T]:
  case Sm(x: T)
  case Nn

  def isDefined: Boolean = this match
    case Nn => false
    case _ => true

  def map[U](f: T => U): Opt[U] = this match
    case Sm(x) => Sm(f(x))
    case Nn => Nn

  def getOrElse[U >: T](other: U): U = this match
    case Sm(x) => x
    case Nn => other

object Opt:
  def apply[T](x: T, keep: Boolean): Opt[T] = if keep then Sm(x) else Nn
  def flatten[T](o: Opt[Opt[T]]): Opt[T] = o match
    case Sm(inner) => inner
    case Nn => Nn

enum Explicit[+T]:
  case Full(x: T) extends Explicit[T]
  case Empty extends Explicit[Nothing]

enum View[-T]:
  case Refl[R](f: R => R) extends View[R]
  case Const(n: Int) extends View[Any]

enum Expr[A]:
  case IntLit(value: Int) extends Expr[Int]
  case BoolLit(value: Boolean) extends Expr[Boolean]
  case Add(left: Expr[Int], right: Expr[Int]) extends Expr[Int]
  case Not(inner: Expr[Boolean]) extends Expr[Boolean]
  case IfElse(cond: Expr[Boolean], yes: Expr[A], no: Expr[A])
  case Pair[X, Y](first: Expr[X], second: Expr[Y]) extends Expr[(X, Y)]

def eval[T](e: Expr[T]): T = e match
  case Expr.IntLit(v) => v
  case Expr.BoolLit(v) => v
  case Expr.Add(l, r) => eval(l) + eval(r)
  case Expr.Not(inner) => !eval(inner)
  case Expr.IfElse(c, y, n) => if eval(c) then eval(y) else eval(n)
  case Expr.Pair(a, b) => (eval(a), eval(b))

enum Token:
  case Plus, Minus
  case Number(value: Int)
  case LParen, RParen
  case Ident(name: String, quoted: Boolean = false)

  def isOperator: Boolean = this == Plus || this == Minus
  def width: Int = this match
    case Number(v) => v.toString.length
    case Ident(n, q) => if q then n.length + 2 else n.length
    case _ => 1

object Shapes:
  enum Shape:
    case Circle(r: Int)
    case Rect(w: Int, h: Int)
    def area: Int = this match
      case Circle(r) => 3 * r * r
      case Rect(w, h) => w * h

  object Inner:
    enum Level:
      case Low, High
      def flip: Level = if this == Low then High else Low

  def unit: Shape = Shape.Rect(1, 1)

enum Weekday(val short: String) extends Ordered[Weekday]:
  case Mon extends Weekday("Mo")
  case Tue extends Weekday("Tu")
  case Wed extends Weekday("We")

  def compare(that: Weekday): Int = ordinal - that.ordinal
  override def toString: String = "day " + short

trait Ordered[A]:
  def compare(that: A): Int
  def <(that: A): Boolean = compare(that) < 0
  def >(that: A): Boolean = compare(that) > 0

def describe(c: Color): String = c match
  case Color.Red => "warm"
  case Color.Green | Color.Blue => "cold"

@main def main(): Unit =
  println(Color.Red)
  println(Color.Green.ordinal)
  println(Color.valueOf("Blue"))
  println(Color.values.toList)
  println(Color.fromOrdinal(0))
  println(Color.Blue.toString + Color.Blue.productPrefix)
  println(Color.values.map(describe).toList)

  println(Rgb.Green.rgb)
  println(Rgb.Mix(0x123456).rgb)
  println(Rgb.Mix(7))
  println(Rgb.Mix(7).ordinal)
  val asRgb: Rgb = Rgb.Mix(1)
  println(asRgb.rgb)

  println(Planet.report(80.0))
  println(Planet.heaviest)
  println(Planet.values.length)

  val some: Opt[Int] = Opt.Sm(2)
  println(some)
  println(some.isDefined)
  println(Opt.Nn.isDefined)
  println(some.map(_ * 21))
  println((Opt.Nn: Opt[Int]).map(_ * 21))
  println(some.getOrElse(0))
  println((Opt.Nn: Opt[Int]).getOrElse(-1))
  println(Opt(5, true))
  println(Opt("x", false))
  println(Opt.flatten(Opt.Sm(Opt.Sm("deep"))))
  val widened = Opt.Sm(3)
  val list: List[Opt[Int]] = List(widened, Opt.Nn)
  println(list)

  val full: Explicit[String] = Explicit.Full("f")
  val empty: Explicit[String] = Explicit.Empty
  println(List(full, empty))

  val view: View[Int] = View.Refl[Int](_ + 1)
  val const: View[String] = View.Const(3)
  println(view match
    case View.Refl(f) => "refl"
    case View.Const(n) => "const " + n)
  println(const match
    case View.Refl(_) => "refl"
    case View.Const(n) => "const " + n)

  val program = Expr.IfElse(Expr.Not(Expr.BoolLit(false)), Expr.Add(Expr.IntLit(40), Expr.IntLit(2)), Expr.IntLit(0))
  println(eval(program))
  println(eval(Expr.Pair(program, Expr.Not(Expr.BoolLit(true)))))
  println(program)

  val tokens = List(Token.LParen, Token.Number(12), Token.Plus, Token.Ident("x"), Token.Ident("y z", true), Token.RParen)
  println(tokens)
  println(tokens.map(_.width).sum)
  println(tokens.filter(_.isOperator))
  println(tokens.map(_.ordinal))
  println(Token.Ident("a") == Token.Ident("a"))
  println(Token.Ident("a") == Token.Ident("a", true))
  println(Token.Ident("a").hashCode == Token.Ident("a").hashCode)
  println(Token.Plus == Token.Plus)
  println(Token.Plus.equals(Token.Minus))
  println(Token.Plus.hashCode == Token.Plus.hashCode)
  println(Token.Number(3).copy(value = 4))
  println(Token.Number(3).productPrefix)

  println(Shapes.Shape.Circle(2).area)
  println(Shapes.unit.area)
  println(Shapes.Inner.Level.Low.flip)
  println(Shapes.Inner.Level.values.toList)
  val shape: Shapes.Shape = Shapes.Shape.Rect(2, 3)
  println(shape)

  val counts = mutable.Map[Color, Int]()
  for c <- List(Color.Red, Color.Blue, Color.Red, Color.Red) do
    counts(c) = counts.getOrElse(c, 0) + 1
  println(counts(Color.Red))
  println(counts.get(Color.Green))
  val byToken: Map[Token, String] = Map(Token.Plus -> "+", Token.Number(1) -> "one", Token.Ident("k") -> "key")
  println(byToken(Token.Number(1)))
  println(byToken.get(Token.Ident("k")))
  println(byToken.get(Token.Ident("k", true)))
  println(byToken.contains(Token.Minus))
  val seen = Set(Rgb.Red, Rgb.Mix(5), Rgb.Red, Rgb.Mix(5), Rgb.Mix(6))
  println(seen.size)
  println(seen.contains(Rgb.Mix(6)))
  println(seen.contains(Rgb.Blue))
  val planets = Set(Planet.Earth, Planet.Venus) + Planet.Earth
  println(planets.size)

  println(Weekday.Tue)
  println(Weekday.Mon < Weekday.Wed)
  println(Weekday.Wed > Weekday.Tue)
  println(Weekday.values.toList)
  println(Weekday.valueOf("Wed").short)
  println(Weekday.Tue.productPrefix)
