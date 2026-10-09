package d2c

import d2a.*
import d2b.*

class Canvas extends Painter(Tone.Dark):
  def shapes: List[Shape] = Palette.default
  def paint(s: Shape): String = s match
    case Circle(r) => s"circle $r"
    case Rect(w, h) => s"rect ${w * h}"
    case Empty => "empty"

object Canvas:
  def main(args: Array[String]): Unit =
    val c = new Canvas
    println(s"${c.all} ${c.tone}")
