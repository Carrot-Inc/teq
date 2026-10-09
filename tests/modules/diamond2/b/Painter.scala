package d2b

import d2a.*

abstract class Painter(val tone: Tone):
  def shapes: List[Shape]
  def paint(s: Shape): String
  def all: String = shapes.map(paint).mkString(";")

object Palette:
  def default: List[Shape] = List(Circle(1), Rect(2, 3), Empty)
