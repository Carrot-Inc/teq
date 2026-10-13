package cvb

@main def run(): Unit =
  val f = cva.F
  println(f(3))
  println(cva.F)
  val o = new cva.O
  val c: Any = o.K
  println(c)
  println(o.K(4))
