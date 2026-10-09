package bb

import ba.*

object Use:
  def main(args: Array[String]): Unit =
    println(Lazy.when(true)(1 + 1))
    var n = 0
    Lazy.twice { n += 1 }
    println(n)
