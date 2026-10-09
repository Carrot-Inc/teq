package xf

object Use:
  def pick(x: Any): String = "Any"
  def pick(x: Int): String = "Int"
  def main(args: Array[String]): Unit =
    println(pick(xe.Up.last((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))))
    println(pick(xe.Up.lastBound((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))))
    println(pick(xe.Up.firstTyped((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))))
