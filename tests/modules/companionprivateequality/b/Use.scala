package app

import lib.Priv

@main def run(): Unit =
  val a = new Priv(1); val b = new Priv(2)
  println(s"${a == b} ${Priv.both(a, b)}")
