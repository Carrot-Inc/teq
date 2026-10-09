package abu

import abn.*

object Use:
  def main(args: Array[String]): Unit =
    var n = 0
    val l = new Lib({ n += 1; n })
    println(l.f)
    println(n)
