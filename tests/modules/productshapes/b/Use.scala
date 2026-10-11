package use

import productshapes.*

object Use:
  def main(args: Array[String]): Unit =
    println(copy(C(1)(2), 3)(4).b)
    var n = 0
    val f = twice[Int]
    println(f({ n += 1; n }))
    val l = lazyPair("yes")
    println("before")
    println(l())
    println(l())
    println(lazySum())
    println(cases(1, 2))
    println(poly(List(7)))
    println(wrap.x.head)
    val o = new Outer(10)
    val m = new o.N(1)(2)
    println(m.copy(a = 3)(4).total)
    println(m.bump.total)
    println(again(o, m))
