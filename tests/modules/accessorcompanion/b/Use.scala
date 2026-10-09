package acu2

import acm.*

object Use:
  def main(args: Array[String]): Unit =
    val c = new C()
    println(c.peek)
    println(c.add(2) + c.add(3))
