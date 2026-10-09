package aub

import aua.*

class C extends T("x")(using 42)

class D extends Two(1)(using "s", 7)

class W extends Wide[Int](3, 9)

@main def run(): Unit =
  val c = C()
  println(c.tag + " " + c.answer)
  println(D().both)
  println(Mixed(1)(using true)(2)(using 'c', 5L).flags)
  println(W().larger)
  println(twice(4)(using 3))
  println("ab".times(3)(using '!'))
  println(aua.label(using 5))
