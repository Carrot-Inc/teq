package xb

import xa.*

object Use:
  def main(args: Array[String]): Unit =
    val c = new Car
    println(c.start(3) + " " + c.engineName)
    println(Kit.hammer + Kit.size + new Kit.Nail(2).length)
