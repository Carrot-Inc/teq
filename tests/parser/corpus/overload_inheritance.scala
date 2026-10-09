//> using scala 3.8.4
trait Shape:
  def scale(f: Int): String = "Shape.scale(" + f + ")"
  def scale(fx: Int, fy: Int): String = "Shape.scale(" + fx + "," + fy + ")"
  def area: Int

trait Named:
  def describe(prefix: String): String = prefix + " named"

trait Tagged:
  def describe(level: Int): String = "tagged " + level

class Square(side: Int) extends Shape, Named, Tagged:
  def area: Int = side * side
  override def scale(f: Int): String = "Square.scale(" + f + ")"
  def scale(label: String): String = "Square.scale(" + label + ")"

class Circle(r: Int) extends Shape:
  def area: Int = 3 * r * r
  override def scale(fx: Int, fy: Int): String = "Circle.scale2"

trait Visitor[R]:
  def visit(s: Square): R
  def visit(c: Circle): R

object AreaVisitor extends Visitor[Int]:
  def visit(s: Square): Int = s.area
  def visit(c: Circle): Int = c.area

object NameVisitor extends Visitor[String]:
  def visit(s: Square): String = "square"
  def visit(c: Circle): String = "circle"

trait Logger:
  def log(msg: String): String = "log: " + msg
class FileLogger extends Logger:
  def log(msg: String, level: Int): String = log(msg) + " @" + level
  def log(code: Int): String = log("code " + code)

def both[R](v: Visitor[R]): List[R] = List(v.visit(Square(2)), v.visit(Circle(1)))

@main def run(): Unit =
  val sq = Square(2)
  println(sq.scale(2))
  println(sq.scale(2, 3))
  println(sq.scale("big"))
  println(sq.describe("it is"))
  println(sq.describe(3))
  val sh: Shape = sq
  println(sh.scale(4))
  println(sh.scale(4, 5))
  val c = Circle(1)
  println(c.scale(1))
  println(c.scale(1, 2))
  println(both(AreaVisitor))
  println(both(NameVisitor))
  val fl = FileLogger()
  println(fl.log("a"))
  println(fl.log("b", 2))
  println(fl.log(7))
  val lg: Logger = fl
  println(lg.log("c"))
