package ib

import ia.*

class IntHandler extends Handler[Int]("int"), Logging:
  def handle(a: Int): String = log(a.toString)

object Use:
  def main(args: Array[String]): Unit =
    println(new IntHandler().run(3))
    val c = new Counter
    c.inc()
    c.count = c.count + 1
    println(c.count + c.start)
