package nppb

import npp.*

object Use:
  def main(args: Array[String]): Unit =
    val o = new O(5)
    println(new o.C().get)
    println(new D().value)
