package gdb

import gda.*

object Use:
  def main(args: Array[String]): Unit =
    println(Defs.a())
    println(Defs.c())
    println(Defs.d(5)())
    println(Defs.e())
    println(new Box[Int].n())
