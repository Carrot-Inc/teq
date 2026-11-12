// A parameter spread into a repeated parameter where the expected type gives it (a formal open in its
// result alone, a typed val), and a parameter beside a repeated one under an open formal: each is typed,
// the first from the expected type, the last from the callee's parameter that is not repeated.
def join(xs: Int*): String = xs.mkString("/")
def sum(n: Int, xs: Int*): Int = n + xs.sum
def onSeq[B](f: Seq[Int] => B): Seq[Int] => B = f
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = onSeq(xs => join(xs*))
  val g: Seq[Int] => String = xs => join(xs*)
  val h = on(n => sum(n, 2, 3))
  println(f(Seq(1, 2)) + " " + g(Seq(3)) + " " + h(1))
