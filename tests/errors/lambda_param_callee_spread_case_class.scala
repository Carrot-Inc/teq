// A lambda under an open formal whose parameter is spread into a case class's repeated parameter. scalac
// 3.8.4 types the function as `Int* => Many` and refuses the call below it, `f(Seq(1, 2))`, with `Found:
// Seq[Int] Required: Int`. Here a repeated parameter gives an open formal no type, and the lambda is
// refused where it is written.
// expect: 10:25: error: type mismatch: found Any, required Seq[Int] | Array[? <: Int]
case class Many(items: Int*)
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => Many(xs*))
  println(f(Seq(1, 2)))
