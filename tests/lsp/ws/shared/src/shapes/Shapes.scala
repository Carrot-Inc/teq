package shapes

trait Shape:
  def area: Double
  def name: String = "shape"

class Circle(val r: Double) extends Shape:
  def area: Double = 3.0 * r * r
  override def name: String = "circle"

class Square(val side: Double) extends Shape:
  def area: Double = side * side

case class Point(x: Int, y: Int):
  def +(other: Point): Point = Point(x + other.x, y + other.y)

enum Color:
  case Red, Green

type Area = Double

object Geometry:
  def total(shapes: List[Shape]): Area = shapes.foldLeft(0.0)((acc, s) => acc + s.area)
  def overloaded(x: Int): String = "int"
  def overloaded(x: String): String = "string"
  def describe(p: Point, label: String = "p"): String = label + p.x
  inline def twice(x: Int): Int = helper(x) + helper(x)
  def helper(x: Int): Int = x * 2
  def first[T](items: List[T], fallback: T): T = if items.isEmpty then fallback else items.head

  object Nested:
    val depth: Int = 2

extension (p: Point)
  def norm: Int = p.x * p.x + p.y * p.y

trait Show[A]:
  extension (a: A) def show: String

given pointShow: Show[Point] with
  extension (a: Point) def show: String = "(" + a.x + ", " + a.y + ")"

given intToPoint: Conversion[Int, Point] = (i: Int) => Point(i, i)

object Exports:
  export Geometry.overloaded
