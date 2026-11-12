// A lambda whose parameter alone is an open formal (the result is given) and is spread into a repeated
// parameter; the function made from it is never applied.
def join(xs: Int*): String = xs.mkString("/")
def len[A](f: A => String): A => Int = a => f(a).length

@main def run(): Unit =
  val g = len(xs => join(xs*))
  println(g.toString.nonEmpty)
