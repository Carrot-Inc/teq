package app

import lib.*

@main def run(): Unit =
  val v: B.u.type = chosen
  println(v eq B.u)
  println(chosen.name)
