// A lambda whose parameter is spread into a repeated parameter under an open formal that has a declared
// bound the sequence conforms to; the function is not applied. scalac 3.8.4 compiles it: the bound is `Iterable[Int]` and the function is put into a list.
def join(xs: Int*): String = xs.mkString("/")
def keepIt[A <: Iterable[Int], B](f: A => B): A => B = f

@main def run(): Unit =
  val f = keepIt(xs => join(xs*))
  println(List(f).size)
