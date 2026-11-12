// A lambda whose parameter is spread into a repeated parameter under an open formal that has a declared
// bound the sequence conforms to; the function is not applied. scalac 3.8.4 compiles it: the bound is `Seq[Any]` and the callee never applies the function.
def join(xs: Int*): String = xs.mkString("/")
def count[A <: Seq[Any], B](f: A => B): Int = 1

@main def run(): Unit =
  println(count(xs => join(xs*)))
