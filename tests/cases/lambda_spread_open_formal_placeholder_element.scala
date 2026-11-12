// A placeholder spread into a repeated parameter under an open formal (`join(_*)`), applied to an
// element: scalac 3.8.4 types the function as `Int* => String`.
def join(xs: Int*): String = xs.mkString("/")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(join(_*))
  println(f(1))
