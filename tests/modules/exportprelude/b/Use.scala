package epb

import epa.*

object Impl extends Thing:
  def n: Int = 1

@main def run(): Unit =
  println(Impl.n + twice(base))
