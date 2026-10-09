case class P(x: Int)
def f(o: Any) = o.toString
@main def run(): Unit =
  val c = P
  println(c(1))
  println(f(P))
  println(List(1, 2).map(P))
