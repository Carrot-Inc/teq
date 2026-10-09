package eib

import eia.Facade

@main def run(): Unit =
  println(Facade.twice(2))
  println(Facade.k + 1)
  val n: Int = Facade.pick(true)
  val s: String = Facade.pick(false)
  println(s"$n$s")
