// A lambda whose parameter is spread into a repeated parameter under an open formal that has a declared
// bound the sequence conforms to; the function is not applied. scalac 3.8.4 compiles it: the lambda has two parameters and the second, the spread one, has the bound `AnyRef`.
def sum(n: Int, xs: Int*): Int = n + xs.sum
def keep2[A, B <: AnyRef, C](f: (A, B) => C): (A, B) => C = f

@main def run(): Unit =
  val f = keep2((n, xs) => sum(n, xs*))
  println(f == null)
