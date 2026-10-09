package dma

final case class Point(x: Int, y: Int)
final case class Box[A](value: A)
case object Origin

sealed trait Shape
final case class Circle(r: Double) extends Shape
final case class Rect(w: Double, h: Double) extends Shape

enum Color:
  case Red, Green
  case Rgb(r: Int, g: Int, b: Int)

object Shapes:
  final case class Square(side: Double)
