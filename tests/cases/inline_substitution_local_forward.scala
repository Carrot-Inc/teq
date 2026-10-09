// A local inline method called before its block reaches its definition, checked on the call in
// its definition's scope: `g`'s `k` is `Base.k`, which the block imports, not the parameter `k`
// of the lambda the first call stands in (scalac prints 101, 102; the retype path types the body
// at the call and prints 2). scalac prints the lines of the .expected file.
object Base:
  val k = 100
@main def run(): Unit =
  import Base.k
  val base = 10
  val first = h(1)
  val f = (k: Int) => g(k)
  inline def g(n: Int): Int = n + k
  inline def h(n: Int): Int = n + base
  println(first)
  println(f(1))
  println(g(2))
