package tpb

import tpa.*

def tag(s: String): String = { println("tag " + s); s }

class C(n: Int) extends Counted("c", n) with Sorted[Int]
class P extends Named(last = tag("L"), first = tag("F"))
object O extends Counted("o", 5, 10)

@main def run(): Unit =
  val c = C(3)
  println(c.bump() + c.bump())
  println(c.label + " " + c.doubled + " " + c.count)
  println(c.sort(List(3, 1, 2)))
  println(P().full)
  println(O.bump())
  val a = new Named("A") {}
  println(a.full)
