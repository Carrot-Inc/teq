package app

import lib.Over

@main def run(): Unit =
  val a = new Over(1); val b = new Over(2)
  println(a.self(b))
  println(a.chr)
