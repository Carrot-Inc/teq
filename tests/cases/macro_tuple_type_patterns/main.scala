package app
import tlib.Fields

type Count = Int
final case class Point(x: Int, y: Count)
final case class Named(flag: Boolean, tags: List[Int], scale: Double)
case class Empty()

sealed trait Shape
final case class Circle(r: Double) extends Shape
case object Dot extends Shape

@main def run(): Unit =
  println(Fields.describe[Point])
  println(Fields.describe[Named])
  println(Fields.describe[Empty])
  println(Fields.describe[Shape])
