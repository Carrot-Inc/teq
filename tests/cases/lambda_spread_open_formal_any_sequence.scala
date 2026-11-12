// A lambda under an open formal whose parameter is spread into an `Any*` parameter, applied to a
// sequence: scalac 3.8.4 types the function as `Any* => String`, and the sequence is its one element.
def show(xs: Any*): String = xs.mkString("[", ",", "]")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => show(xs*))
  println(f(Seq(1, 2)))
