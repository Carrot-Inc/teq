// A lambda under an open formal whose parameter is spread into a repeated parameter, given to a method
// that never applies it: scalac 3.8.4 compiles it, whatever the function's parameter type is.
def join(xs: Int*): String = xs.mkString("/")
def count[A, B](f: A => B): Int = 1

@main def run(): Unit =
  println(count(xs => join(xs*)))
