package isb

import isa.Eqv

case class P(a: Int, b: String) derives Eqv
case class W(a: Int, b: Int, c: Int) derives Eqv
enum Color derives Eqv:
  case Red, Green

@main def run(): Unit =
  println(summon[Eqv[P]].eqv(P(1, "a"), P(1, "b")))
  println(summon[Eqv[W]].eqv(W(1, 2, 3), W(4, 5, 6)))
  println(summon[Eqv[Color]].eqv(Color.Red, Color.Red))
  println(Eqv.by[String].eqv("x", "y"))
  println(Eqv.arity[(Int, String, Boolean)])
