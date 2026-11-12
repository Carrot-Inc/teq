// A lambda under an open formal whose parameter is spread into a method's repeated parameter. scalac
// 3.8.4 gives the variable the repeated type (`inferredFromTarget`'s `formal <:< target`): its function
// is `Int* => String`, and it refuses the call below it, `f(Seq(1, 2))`, with `Found: Seq[Int] Required:
// Int`. Here a repeated parameter gives an open formal no type, and the lambda is refused where it is
// written.
// expect: 11:25: error: type mismatch: found Any, required Seq[Int] | Array[? <: Int]
def join(xs: Int*): String = xs.mkString("/")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => join(xs*))
  println(f(Seq(1, 2)))
