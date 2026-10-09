package web

import shapes.*
import shapes.given
import shapes.Geometry.{describe as describePoint}
import scala.language.implicitConversions
import shapes.Geometry.Nested.*

object Page:
  def render(): String =
    val local = 40
    val circle = new Circle(1.0)
    val sum = local + circle.area.toInt
    val p = Point(1, 2)
    val q = p + Point(3, 4)
    val chosen = Geometry.overloaded("x")
    val n = q.norm
    val shown = q.show
    val converted = 5.x
    val named = Geometry.describe(q, label = "q")
    val alias: Area = 1.0
    val first = Geometry.first(List(1), 0)
    val color = Color.Red
    val exported = Exports.overloaded(1)
    val summoned = summon[Show[Point]].show(p)
    val via = pointShow.show(p)
    val doubled = Geometry.twice(3)
    val expanded = Macros.plusOne(2)
    val all = Geometry.total(List(circle))
    val nestedDepth = depth
    val kind = p match
      case Point(a, b) => a + b
    describePoint(q) + sum + chosen + n + shown + converted + named + alias + first + color + exported + summoned + via + doubled + expanded + kind + Geometry.Nested.depth + all + nestedDepth

  def area(shape: Shape): Double = shape.area
