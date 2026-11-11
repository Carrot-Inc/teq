package rbb

import rba.*

object Text extends Bounded with Free:
  type T = String
object Vs extends AbsU with Free:
  type T = V

@main def run(): Unit =
  println(Api.looser(Text)("looser"))
  println(Api.wider(Text)("wider"))
  println(Api.both(Text)("both"))
  println(Api.depU(Vs)(new V(1)).x)
  println(Api.interU(Vs)(new V(2)).x)
