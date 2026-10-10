object Effects:
  def effect(n: Int): Int =
    println(s"effect $n")
    n

@main def run(): Unit =
  val a = Effects.effect(1)
  println(U.shown(a))
  println(U.twice(a))
  println(U.proxied(a + 1))
  println(U.nestedExpr(a + 2))
  println(U.shown(Effects.effect(2)))
