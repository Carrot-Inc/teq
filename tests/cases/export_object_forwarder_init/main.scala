package p

@main def run(): Unit =
  println("start")
  println(O.x)
  val o: Lib.O.type = O
  println(o.x)
  println(Wrap.N.y)
  val n: Lib.N.type = Wrap.N
  println(n.y)
