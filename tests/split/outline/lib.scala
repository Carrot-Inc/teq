package lib

import scala.compiletime.constValue

trait Render:
  def render(n: Int): String

object Fields:
  var calls = 0

  inline def field[L <: String](value: Int, inline scale: Int): String =
    val label = constValue[L]
    calls += 1
    val shown = new Render:
      def render(n: Int): String = label + ": " + n
    val scaled = if value > 0 then value * scale else -value
    val suffix = if scaled > 100 then " (large)" else if scaled > 10 then " (medium)" else ""
    shown.render(scaled) + suffix
