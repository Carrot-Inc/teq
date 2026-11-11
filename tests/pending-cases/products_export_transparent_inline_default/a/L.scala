package etr

class Box(val k: Int):
  transparent inline def tr(inline x: Int = k + 3): Int = x * 10

object Fw:
  val bx = Box(4)
  export bx.tr
