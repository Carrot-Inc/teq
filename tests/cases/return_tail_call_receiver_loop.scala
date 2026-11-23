// A def's loop through another instance where it calls itself in return position: its receiver evaluated before its
// arguments, a left-out default argument read on the called instance, and a `return` of a call on another instance
// in a def that loops on `this`.
object Log { var s = ""; def r[T](tag: String, v: T): T = { s += tag; v } }
final class Cell(val id: Int) {
  var next: Cell = this
  final def plain(n: Int, m: Int): Int = if n > 0 then Log.r("R", this).plain(Log.r("A", n - 1), Log.r("B", m + 1)) else m
  final def default(n: Int, x: Int = id): Int = if n > 0 then next.default(n - 1) else x
  final def mixed(n: Int): Int = { if n > 5 then return next.mixed(n - 1); if n > 0 then mixed(n - 1) else id }
}
object Main {
  def main(args: Array[String]): Unit = {
    val a = new Cell(1)
    val b = new Cell(2)
    a.next = b
    b.next = new Cell(3)
    Log.s = ""
    val p = a.plain(2, 0)
    println("plain " + p + " " + Log.s)
    println("default " + a.default(1) + " " + a.default(2))
    println("mixed " + a.mixed(7))
  }
}
