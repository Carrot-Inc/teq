package rb

import ra.*

object Use:
  def main(args: Array[String]): Unit =
    val s = Stores.ints
    val i: Int = s.get
    println(i + 1)
