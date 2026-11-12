// A lambda under an open formal whose parameter is a plain argument of an `Any*` parameter, applied to
// a sequence: scalac 3.8.4 types the function as `Any* => String`; the sequence is the one element of its
// argument, which is in turn the one element `show` is given.
def show(xs: Any*): String = xs.mkString("[", ",", "]")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(x => show(x))
  println(f(Seq(1, 2)))
