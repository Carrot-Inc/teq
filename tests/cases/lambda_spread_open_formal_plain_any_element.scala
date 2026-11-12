// A lambda under an open formal whose parameter is a plain argument of an `Any*` parameter, applied to
// an element: scalac 3.8.4 types the function as `Any* => String`, and `show` is given the sequence of
// the function's elements as its one element.
def show(xs: Any*): String = xs.mkString("[", ",", "]")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(x => show(x))
  println(f(1))
