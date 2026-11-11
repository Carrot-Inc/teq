package tsidu

import tsid.*

trait T3 extends T:
  def h: Int = super.tm()

class D3 extends T3:
  override def tk: Int = 100

class D2 extends T2

@main def run(): Unit =
  println(C2().f)
  println(D2().f)
  println(D3().h)
