package use

import nestedcompanion.*

object Use:
  def main(args: Array[String]): Unit =
    val o = new O(10)
    val c = o.C(1)(2)
    println(c.copy(a = 3)(4).total)
    println(o.E(5).total)
    println(o.E.first.total)
    val p = new O(20)
    val d = p.D(1)(2)(3)
    def recv = { println("receiver"); d }
    val f = recv.copy(a = 4)(5)
    println(f(6).total)
    println(f(7).total)
