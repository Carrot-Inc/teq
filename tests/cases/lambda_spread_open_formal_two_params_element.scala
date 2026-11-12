// A lambda of two parameters under open formals, the second spread into the callee's repeated
// parameter, applied to elements: scalac 3.8.4 types the function as `(Int, Int*) => Int`.
def sum(n: Int, xs: Int*): Int = n + xs.sum
def on2[A, B, C](f: (A, B) => C): (A, B) => C = f

@main def run(): Unit =
  val f = on2((n, xs) => sum(n, xs*))
  println(f(1, 2))
