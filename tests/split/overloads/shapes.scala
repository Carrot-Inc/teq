package shapes

case class Circle(r: Int)
case class Rect(w: Int, h: Int)

trait Painter:
  def paint(c: Circle): String
  def paint(r: Rect): String
  def paint(label: String): String = "label " + label

object Measure:
  def area(c: Circle): Int = 3 * c.r * c.r
  def area(r: Rect): Int = r.w * r.h
  def describe(c: Circle): String = "circle of " + area(c)
  def describe(r: Rect): String = "rect of " + area(r)
