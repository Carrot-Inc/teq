package shapes.geometry

case class Point(x: Double, y: Double)

trait Shape:
  def area: Double
  def name: String

case class Circle(center: Point, radius: Double) extends Shape:
  def area: Double = 3.0 * radius * radius
  def name: String = "circle"

case class Square(corner: Point, side: Double) extends Shape:
  def area: Double = side * side
  def name: String = "square"

object Shapes:
  val unit: Square = Square(Point(0.0, 0.0), 1.0)
  def describe(s: Shape): String = s"${s.name} with area ${s.area}"

  given Ordering[Shape] with
    def compare(a: Shape, b: Shape): Int = if a.area < b.area then -1 else if a.area > b.area then 1 else 0

extension (p: Point)
  def +(other: Point): Point = Point(p.x + other.x, p.y + other.y)
