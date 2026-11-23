// A `return` of a call on `this` inside a `while` of a def that also loops through another receiver. On JavaScript
// the loop the def has by its tail call in return position continues the inner `while` (teq's row: `12312` where
// scalac prints `123`).
final class M(val id: Int, var next: M) {
  final def thisInWhileWithOther(n: Int, acc: Int): Int = {
    var i = 0
    while i < 2 do { i += 1; if n > 0 && n % 2 == 1 then return thisInWhileWithOther(n - 1, acc + i) }
    if n == 0 then acc else next.thisInWhileWithOther(n - 1, acc * 10 + id)
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    val a = new M(1, null); val b = new M(2, null); val c = new M(3, a)
    a.next = b; b.next = c
    println(a.thisInWhileWithOther(5, 0))
  }
}
