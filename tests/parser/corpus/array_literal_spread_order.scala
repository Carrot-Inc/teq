// `Array(x, xs*)` with a spread evaluates the head, then the sequence, each once, before the
// array is made of them.
@main def run(): Unit =
  val b = Array({ println("head"); 1 }, { println("tail"); Seq(2, 3) }*)
  println(b.mkString(","))
  var n = 0
  val c = Array({ n += 1; n }, { n += 10; Seq(n, n) }*)
  println(c.mkString(",") + " " + n)
  val d = Array({ println("h"); 'a' }, { println("t"); List('b') }*)
  println(d.mkString(","))
