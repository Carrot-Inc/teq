package ltb

@main def run(): Unit =
  println(lta.Lib.make(7).get + lta.Lib.make(8).get)
  val c = lta.Lib.counter(10)
  val d = lta.Lib.counter(20)
  println(s"${c.next()} ${c.next()} ${d.next()}")
  println(lta.Lib.twice(3) + lta.Lib.twice(4))
