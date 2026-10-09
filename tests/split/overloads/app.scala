package app

import shapes.*

class Ascii extends Painter:
  def paint(c: Circle): String = "(" + c.r + ")"
  def paint(r: Rect): String = "[" + r.w + "x" + r.h + "]"
  def paint(size: Int): String = "#" * size

@main def run(): Unit =
  val p = Ascii()
  val painter: Painter = p
  println(painter.paint(Circle(2)) + " " + painter.paint(Rect(2, 3)) + " " + painter.paint("hi"))
  println(p.paint(3))
  println(Measure.describe(Circle(1)) + ", " + Measure.describe(Rect(2, 2)))
