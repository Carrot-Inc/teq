//> using scala 3.8.4
case class Circle(r: Int)
case class Rect(w: Int, h: Int)

object Geometry:
  def area(c: Circle): Int = 3 * c.r * c.r
  def area(r: Rect): Int = r.w * r.h
  def area(w: Int, h: Int): Int = w * h
  def describe(x: Int): String = "int " + x
  def describe(x: Long): String = "long " + x
  def describe(x: String): String = "string " + x
  def describe(x: Any): String = "any " + x

class Counter:
  private var n = 0
  def add(): Counter = { n += 1; this }
  def add(k: Int): Counter = { n += k; this }
  def add(a: Int, b: Int): Counter = { n += a * b; this }
  def value: Int = n

trait Shape:
  def scale(f: Int): Shape
  def scale(fx: Int, fy: Int): Shape
  def name: String

class Box(val w: Int, val h: Int) extends Shape:
  def scale(f: Int): Shape = Box(w * f, h * f)
  def scale(fx: Int, fy: Int): Shape = Box(w * fx, h * fy)
  def name: String = s"Box($w, $h)"

def render(s: String): String = "<" + s + ">"
def render(n: Int): String = "#" + n
def render(s: String, n: Int): String = render(s) * n

@main def run(): Unit =
  println(Geometry.area(Circle(2)))
  println(Geometry.area(Rect(2, 3)))
  println(Geometry.area(4, 5))
  println(Geometry.describe(1))
  println(Geometry.describe(1L))
  println(Geometry.describe("s"))
  println(Geometry.describe(true))
  println(Counter().add().add(2).add(3, 4).value)
  val s: Shape = Box(1, 2)
  println(s.scale(2).name)
  println(s.scale(2, 3).name)
  println(render("a"))
  println(render(7))
  println(render("ab", 2))
