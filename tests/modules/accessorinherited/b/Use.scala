package aib

import ain.*

class Sub extends Holder
class GenSub extends Gen[String]("h")

object Use:
  def main(args: Array[String]): Unit =
    val l = new Lib()
    println(l.f(41))
    println(l.bump() + l.bump())
    println(new Fin().g(1))
    println(new Sub().n)
    val g = new GenSub
    println(g.get + g.get + g.seen)
