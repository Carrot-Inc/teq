package uvb

import uva.syntax.*

object Main:
  def main(args: Array[String]): Unit =
    val v: vec2 = new vec2(1, 2) + new vec2(3, 4)
    val w: uva.syntax.vec4 = new uva.syntax.vec4(1, 2, 3, 4)
    println(v)
    println(uva.syntax.dot(v, new vec2(1, 1)) + w.w)
