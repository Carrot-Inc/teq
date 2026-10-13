package oeb

@main def run(): Unit =
  val a = new oea.O
  val b = new oea.O
  println(a.I(1) == b.I(1))
  println(a.I(1) == a.I(1))
  val x: oea.O#I = a.I(1)
  println(x match { case b.I(n) => "bad"; case _ => "good" })
  val p: oea.O#Plain = new a.Plain(2)
  println(p match { case _: b.Plain => "bad"; case q: a.Plain => q.n })
