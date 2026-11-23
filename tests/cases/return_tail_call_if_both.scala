// A `return` of a conditional both of whose branches call the def by itself: each call goes round the loop.
object Main {
  def f(n: Int): Int = {
    if n > 0 then return (if n % 2 == 0 then f(n - 1) else f(n - 1))
    7
  }
  def main(args: Array[String]): Unit = println(f(4))
}
