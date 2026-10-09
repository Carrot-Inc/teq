package d2a

sealed trait Shape
final case class Circle(r: Int) extends Shape
final case class Rect(w: Int, h: Int) extends Shape
case object Empty extends Shape

enum Tone:
  case Light, Dark
