package nclb

import ncla.*

object Loud extends Deriv:
  println("loud")
  val name: String = "L"
def mk(n: String): Impl = { println("mk " + n); Impl(n) }

@main def run(): Unit =
  val flag = mk("flag").name.nonEmpty
  println((if flag then Loud else Loud).derived.show())
  println(mk("a").derived.show())
  println(mk("c").passed.show())
  val v = Impl("v")
  println(v.derived.show())
