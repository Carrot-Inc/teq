// A lambda whose parameter is spread into a repeated parameter under an open formal that has a declared
// bound the sequence conforms to; the function is not applied. scalac 3.8.4 compiles it: the variable is a class's type parameter with the bound `AnyRef`.
def join(xs: Int*): String = xs.mkString("/")
class Holder[A <: AnyRef, B](val f: A => B)

@main def run(): Unit =
  val h = Holder(xs => join(xs*))
  println(h.f == null)
