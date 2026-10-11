package pb

import pa.*

object Use:
  def main(args: Array[String]): Unit =
    given String = "kg"
    val i = Item("a")(1)(using "g")
    val j = i.copy(name = "b")(2)
    println((j.name, j.weight, j.unit))
    val k = i.copy(name = "c")
    println(k(3).weight)
    println(Code(4) match { case Code(n) => n; case _ => 0 })
    println(Code(0) match { case Code(n) => n; case _ => -1 })
    println(7 match { case Pick(n) => n; case _ => 0 })
    println("abc" match { case Pick(n) => n; case _ => 0 })
    var count = 0
    val f = twice
    println(f({ count += 1; count }))
    lazy val (x, y) = { println("init"); (1, 2) }
    println("before")
    println(x + y)
