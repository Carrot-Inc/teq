package xb

import xa.*

object Use:
  def main(args: Array[String]): Unit =
    println(Up.last((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)) + 1)
    println(Up.mixed((1, "s2", 3, "s4", 5, "s6", 7, "s8", 9, "s10", 11, "s12", 13, "s14", 15, "s16", 17, "s18", 19, "s20", 21, "s22", 23000000000000L)))
