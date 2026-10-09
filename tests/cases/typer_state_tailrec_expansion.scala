import scala.annotation.tailrec

// A `@tailrec` method whose recursive call is the expansion of a plain inline call: the check
// reads the body once its plain calls are expanded, as scalac's runs after its `Inlining` phase.
object M:
  inline def next(n: Int): Int = A.loop(n)
object A:
  @tailrec def loop(n: Int): Int =
    if n == 0 then 42
    else M.next(n - 1)
@main def main(): Unit = println(A.loop(10))
