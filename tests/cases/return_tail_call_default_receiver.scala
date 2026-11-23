// A def's call of itself under a `return` on another instance: a left-out default argument is evaluated on the
// called instance, and a by-name argument and a closure keep what they read in their own iteration.
final class Cell(val id: Int) {
  var next: Cell = this
  final def f(n: Int, x: Int = id): Int = {
    if n > 0 then return next.f(n - 1)
    x
  }
  final def byName(n: Int, x: => Int): Int = {
    if n > 0 then return next.byName(n - 1, x * 10 + id)
    x
  }
  final def captured(n: Int, acc: List[() => Int]): Int = {
    if n > 0 then return next.captured(n - 1, (() => id * 100 + n) :: acc)
    acc.map(g => g()).sum
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    val a = new Cell(1)
    val b = new Cell(2)
    a.next = b
    b.next = new Cell(3)
    println(a.f(1))
    println(a.f(2))
    println(a.byName(2, 0))
    println(a.captured(2, Nil))
  }
}
