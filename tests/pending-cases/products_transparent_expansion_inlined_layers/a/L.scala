package ett

class Box(val k: Int):
  inline def p(inline x: Int = k + 5): Int = x * 100

object Fw:
  val bx = Box(4)

transparent inline def tt: Int = Fw.bx.p() + 1
