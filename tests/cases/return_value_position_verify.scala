// A `return` where a `String` is wanted writes nothing after it, so the dead code that follows is
// verified against a `String`, as after a throw. Nor does anything follow a conditional's branch that
// leaves the method or goes round its loop, alone, as a block's result or in both branches of a
// conditional.
object Main {
  def f(x: Int): String = { val y: String = if x > 0 then return "p" else "n"; y + x }
  def g(x: Int): String = "x" + (if x > 0 then return "r" else "e")
  final def bj(n: Int): Int = { val r: Int = if n > 0 then { val m = n - 1; return bj(m) } else 7; r }
  final def bj2(n: Int): Int = return (if n > 0 then { val m = n - 1; bj2(m) } else 7)
  final def nj(n: Int): Int = return (if n > 0 then (if n % 2 == 0 then nj(n - 1) else nj(n - 2)) else 7)
  def br(x: Int): Int = { val y: Int = if x > 0 then { val z = x * 2; return z } else 1; y + 1 }
  def nr(x: Int): Int = { val y: Int = if x > 0 then (if x > 5 then return 1 else return 2) else x; y }
  def sr(x: Int): Int = { val y: Int = if x > 0 then { if x > 5 then return 1; 5 } else 1; y }
  def main(args: Array[String]): Unit = {
    println(f(1) + f(-1) + " " + g(1) + g(-1))
    println(List(bj(3), bj2(3), nj(5), br(3), br(-1), nr(9), nr(3), nr(-1), sr(9), sr(3)).mkString(" "))
  }
}
