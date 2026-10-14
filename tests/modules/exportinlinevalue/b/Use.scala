package eivb

import eiv.*

@main def run(): Unit =
  println(f(1))
  val s: String = pick(true)
  println(s.length)
  val p = new Proxy(new Lib)
  println(p.f(1) + p.twice(5))
