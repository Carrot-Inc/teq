// A lambda of two parameters under open formals, the second spread into the callee's repeated parameter.
// scalac 3.8.4 types the function as `(Int, Int*) => Int` and refuses the call below it, `f(1, Seq(2, 3))`,
// with `Found: Seq[Int] Required: Int`. Here the first parameter is typed from the callee's parameter in its
// place, as scalac types it, and the repeated parameter gives the second no type: the lambda is refused where
// it is written, at the sequence argument.
// expect: 11:33: error: type mismatch: found Any, required Seq[Int] | Array[? <: Int]
def sum(n: Int, xs: Int*): Int = n + xs.sum
def on2[A, B, C](f: (A, B) => C): (A, B) => C = f

@main def run(): Unit =
  val f = on2((n, xs) => sum(n, xs*))
  println(f(1, Seq(2, 3)))
