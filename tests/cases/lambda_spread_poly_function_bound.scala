// A lambda whose parameter is spread into a repeated parameter, given to a polymorphic function value whose
// type parameter has the bound `Array[Int]`; the function is not applied. scalac 3.8.4 compiles it.
def join(xs: Int*): String = xs.mkString("/")
val poly = [A <: Array[Int]] => (f: A => String) => f

@main def run(): Unit =
  val f = poly(xs => join(xs*))
  println(f == null)
