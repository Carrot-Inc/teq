//> using option -language:postfixOps
// teq: --language postfixOps
// Postfix operators where scalac's -language:postfixOps enables them, teq's --language postfixOps.
@main def run(): Unit =
  val xs = List(5, 2, 8)
  println(xs sorted)
  println(xs.count(4 <))
  println("x".map(_.toUpper) reverse)
