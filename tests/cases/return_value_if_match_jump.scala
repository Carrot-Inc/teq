// A `return`'s value that is a conditional whose `then` is a match one of whose cases is the def's own call that
// loops: the case's jump stands as a statement of its block (a Unit match), and the `then` still goes on to the
// conditional's end, never into its `else`. Beside it, a conditional whose branches both jump under a type test,
// returns in both branches of a block's conditional, and a branch whose block returns before dead code.
object R {
  def f(n: Int, c: Boolean): Unit = {
    if n < 0 then return
    return (if c then (n match { case 1 => f(n - 1, c); case _ => () }) else println("else"))
  }

  def g(n: Int, x: Any): Int = {
    if n <= 0 then return 7
    return (if x.isInstanceOf[String] then (if n % 2 == 0 then g(n - 1, x) else g(n - 2, x)) else 0)
  }

  def h(n: Int, c: Boolean): Int = {
    val y: Int = { val z = n + 1; if c then return z else return z * 2 }
    y + 100
  }

  def k(n: Int): Int = {
    val y = if n > 0 then { return n; 5 } else 6
    y + 1
  }
}

object Main {
  def main(args: Array[String]): Unit = {
    R.f(3, true); println("f1")
    R.f(1, true); println("f2")
    R.f(2, false); println("f3")
    println(R.g(6, "s")); println(R.g(5, 1))
    println(R.h(1, true)); println(R.h(1, false))
    println(R.k(3)); println(R.k(-3))
  }
}
