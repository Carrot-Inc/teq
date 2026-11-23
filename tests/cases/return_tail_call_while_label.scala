// A call of a def by itself under a `return` inside a `while` of its body goes round the def's loop, not the `while`'s.
object Main {
  def f(n: Int): Int = {
    var k = 0
    while k < 1 do {
      k += 1
      if n > 0 then return f(n - 1)
    }
    n
  }
  def main(args: Array[String]): Unit = println(f(3))
}
