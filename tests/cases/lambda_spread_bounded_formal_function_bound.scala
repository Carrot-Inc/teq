// A lambda whose parameter is spread into a repeated parameter under an open formal that has a declared
// bound the sequence conforms to; the function is not applied. scalac 3.8.4 compiles it: the bound is the function type `Int => Int`, which a sequence of Int is.
def join(xs: Int*): String = xs.mkString("/")
def on[A <: Int => Int, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => join(xs*))
  println(f == null)
