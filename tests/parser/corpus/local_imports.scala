enum Color:
  case Red, Green, Blue

enum Status:
  case None, Pending, Done

object Shapes:
  case class Circle(r: Int)
  case class Square(side: Int)
  val unit: Circle = Circle(1)
  def area(c: Circle): Int = 3 * c.r * c.r

  object Deep:
    val answer: Int = 42
    def twice(n: Int): Int = n * 2

object Syntax:
  extension (n: Int)
    def squared: Int = n * n
    def plus(m: Int): Int = n + m

trait Show[A]:
  def show(a: A): String

object Instances:
  given intShow: Show[Int] with
    def show(a: Int): String = s"int:$a"
  given Show[String] with
    def show(a: String): String = s"str:$a"

def show[A](a: A)(using s: Show[A]): String = s.show(a)

object Palette:
  import Color.*
  val primary: List[Color] = List(Red, Green, Blue)
  def warm(c: Color): Boolean = c == Red
  def name(c: Color): String = c match
    case Red => "red"
    case Green => "green"
    case Blue => "blue"

object Tracker:
  import Status.*
  val initial: Status = None
  def next(s: Status): Status = s match
    case None => Pending
    case Pending => Done
    case Done => Done
  def optional: Option[Int] = scala.None

class Painter(base: Color):
  import Color.*
  def isBase(c: Color): Boolean = c == base
  def other: Color = if base == Red then Green else Red

trait Describer:
  import Shapes.*
  def describe(c: Circle): String = s"circle of ${c.r} with area ${area(c)}"
  def default: Circle = unit

object Describer extends Describer

enum Planet:
  import Syntax.*
  case Mercury, Venus
  def weight: Int = (ordinal + 1).squared.plus(1)

given Show[Color] with
  import Color.*
  def show(c: Color): String = c match
    case Red => "R"
    case Green => "G"
    case Blue => "B"

object Nested:
  object Inner:
    val greeting: String = "hello"
    object Innermost:
      val farewell: String = "bye"
  import Inner.*
  val first: String = greeting
  import Innermost.*
  val second: String = farewell

object Selectors:
  import Shapes.{Circle as Round, area}
  import Shapes.Deep.{answer as _, *}
  val answer: String = "own answer"
  def run(): Unit =
    println(area(Round(2)))
    println(twice(4))
    println(answer)

object Positions:
  val before: String = show(1)(using Instances.intShow)
  import Instances.given
  val after: String = show(2)
  val text: String = show("x")

def inDef(c: Color): String =
  import Color.*
  c match
    case Red => "def red"
    case _ => "def other"

def inBlock(): Unit =
  val outside = Some(1)
  val inside =
    import Status.*
    val s: Status = None
    s
  val after = None
  println(s"$outside $inside $after")

def inLambda(xs: List[Int]): List[Int] =
  xs.map: x =>
    import Syntax.*
    x.squared

def givensInDef(): Unit =
  import Instances.given
  println(show(7))
  println(show("seven"))

def typesInDef(): Unit =
  import Shapes.*
  val c: Circle = Circle(3)
  val s: Square = Square(4)
  println(s"${area(c)} ${s.side}")
  import Deep.twice
  println(twice(21))

def shadowing(): Unit =
  import Status.*
  val s: Status = None
  val inner =
    import scala.None
    val o: Option[Int] = None
    o
  println(s"$s $inner ${Pending}")

@main def main(): Unit =
  println(Palette.primary)
  println(Palette.warm(Color.Red))
  println(Palette.name(Color.Blue))
  println(Tracker.initial)
  println(Tracker.next(Tracker.next(Status.None)))
  println(Tracker.optional)
  println(Painter(Color.Green).isBase(Color.Green))
  println(Painter(Color.Red).other)
  println(Describer.describe(Describer.default))
  println(Planet.Venus.weight)
  println(show(Color.Green))
  println(Nested.first + " " + Nested.second)
  Selectors.run()
  println(Positions.before + " " + Positions.after + " " + Positions.text)
  println(inDef(Color.Red))
  println(inDef(Color.Blue))
  inBlock()
  println(inLambda(List(1, 2, 3)))
  givensInDef()
  typesInDef()
  shadowing()
