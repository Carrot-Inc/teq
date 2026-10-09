package aib

import ain.*

object Use:
  def main(args: Array[String]): Unit =
    val l = new Lib()
    println(l.f(41))
    println(l.bump() + l.bump())
    println(new Fin().g(1))
