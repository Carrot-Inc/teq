package upb

import upa.*

object Other:
  def other(x: (U1 | U2) & Z): String = "other"

@main def run(): Unit =
  println(L.applied(new ZU1))
  println(L.aliased(new ZU1))
  println(Other.other(new ZU1))
