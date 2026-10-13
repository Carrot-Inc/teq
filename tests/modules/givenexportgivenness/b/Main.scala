package gegb

import gega.O

object E:
  export O.given
object F:
  export O.*
object G:
  export O.{given, *}

@main def run(): Unit =
  println(E.x)
  println(F.x("f"))
  println(s"${G.x}${G.x("g")}")
  import E.given
  println(summon[Int])
