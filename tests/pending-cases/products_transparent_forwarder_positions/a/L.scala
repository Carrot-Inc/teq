package n2

class Box(val k: Int):
  extension (s: Int) inline def ex(inline y: Int): Int = s * 10 + y

object Fw:
  val bx = Box(4)
  export bx.ex

transparent inline def tfex: Int = Fw.ex(8)(9) + 6
