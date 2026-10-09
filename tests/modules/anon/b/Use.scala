package anb

import ana.*

object Use:
  def main(args: Array[String]): Unit =
    val c = Counters.from(3)
    c.next()
    println(c.next())
    println(Counters.squares(List(1, 2, 3)))
    println(new Counters.Local(4).plus(_ * 10))
