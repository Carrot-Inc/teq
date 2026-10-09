package pcub

import pcua.P

object Use:
  def main(args: Array[String]): Unit =
    given Int = 7
    given String = "curried"
    println(P.both([A <: AnyVal, B] => (a: A) ?=> (b: B) ?=> s"$b $a"))
