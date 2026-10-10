package nplb

import npl.*

class E extends H.o.I()

object Use:
  def main(args: Array[String]): Unit =
    val o = new O(1)
    val a = new O(2)
    val x: Any = new o.V { val v = 2 }
    println(x match
      case _: a.V => "matched"
      case _ => "missed")
    println(new E().value)
    println(a.test(new o.J(1)) + " " + a.test(new a.J(1)))
