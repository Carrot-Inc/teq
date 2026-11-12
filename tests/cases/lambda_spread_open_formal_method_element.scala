// A lambda under an open formal whose parameter is spread into a method's repeated parameter, applied
// to an element: scalac 3.8.4 types the function as `Int* => String`, a function of the elements.
def join(xs: Int*): String = xs.mkString("/")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => join(xs*))
  println(f(1))
