// A lambda under an open formal whose parameter is spread into an enum case's repeated parameter. scalac
// 3.8.4 types the function as `Int* => Node` and refuses the call below it, `f(Seq(1, 2))`, with `Found:
// Seq[Int] Required: Int`. Here a repeated parameter gives an open formal no type, and the lambda is
// refused where it is written.
// expect: 12:30: error: type mismatch: found Any, required Seq[Int] | Array[? <: Int]
enum Node:
  case Many(items: Int*)
  case Plain
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => Node.Many(xs*))
  println(f(Seq(1, 2)))
