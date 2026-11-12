// A lambda with no expected type whose parameter is spread into the callee's repeated parameter: its type
// is the sequence of the repeated parameter's elements.
def join(xs: String*): String = xs.mkString("/")
def sum(n: Int, xs: Int*): Int = n + xs.sum

@main def run(): Unit =
  val spread = xs => join(xs*)
  val placeholder = join(_*)
  val two = (n, xs) => sum(n, xs*)
  println(spread(Seq("a", "b")) + " " + placeholder(List("c")) + " " + two(1, Seq(2, 3)))
