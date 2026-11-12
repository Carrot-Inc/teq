// A lambda under an open formal whose parameter is spread into a repeated parameter, kept in a val and
// never applied.
def join(xs: Int*): String = xs.mkString("/")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => join(xs*))
  println(f == null)
