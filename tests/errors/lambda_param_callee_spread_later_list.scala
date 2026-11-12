// A lambda under an open formal whose parameter is spread into a repeated parameter, applied by the callee
// to an argument of a later list. scalac 3.8.4 types the variable as `Int*` and refuses the later list's
// sequence, `Seq(1, 2)`, with `Found: Seq[Int] Required: Int`. Here a repeated parameter gives an open
// formal no type, and the lambda is refused where it is written.
// expect: 10:26: error: type mismatch: found Any, required Seq[Int] | Array[? <: Int]
def join(xs: Int*): String = xs.mkString("/")
def app[A, B](f: A => B)(a: A): B = f(a)

@main def run(): Unit =
  println(app(xs => join(xs*))(Seq(1, 2)))
