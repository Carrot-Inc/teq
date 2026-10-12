package ncb

// scalac's client reaches the companion through `o.K()` for the pattern (its `unapply`).
@main def run(): Unit =
  val o = new nca.O(1)
  val k = new o.K(2, "y")
  println(k.sum)
  println(new o.K(2, "y") == new o.K(2, "y"))
  println(k match { case o.K(x, y) => s"$x $y" })
  println(o.make(5).sum)
