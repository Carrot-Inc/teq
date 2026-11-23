// A call of a def by itself that goes round its loop evaluates its receiver, then its arguments, in the
// source's order, as dotty's TailRec assigns them: under a `return` on another instance (in return position:
// return_tail_call_receiver_loop).
object Log { var s = ""; def r[T](tag: String, v: T): T = { s += tag; v } }
class Base { def f(n: Int, m: Int): Int = n + m }
final class Cell(val next: Cell) extends Base {
  final def call(n: Int, m: Int): Int = { if n > 0 then return Log.r("R", this).call(Log.r("A", n - 1), Log.r("B", m + 1)); m }
  final def field(n: Int, m: Int): Int = { if n > 0 then return next.field(Log.r("A", n - 1), Log.r("B", m + 1)); m }
  final def other(n: Int, m: Int): Int = { if n > 0 then return new Cell(Log.r("N", this)).other(Log.r("A", n - 1), Log.r("B", m + 1)); m }
  override def f(n: Int, m: Int): Int = { if n > 0 then return super.f(Log.r("A", n - 1), Log.r("B", m + 1)); m }
}
object Main {
  def run(tag: String, v: => Int): Unit = { Log.s = ""; val r = v; println(tag + " " + r + " " + Log.s) }
  def main(args: Array[String]): Unit = {
    val c = new Cell(null)
    val d = new Cell(c)
    run("call", c.call(2, 0)); run("field", { val e = new Cell(null); val ring = new Cell(e); ring.field(1, 0) })
    run("other", c.other(2, 0)); run("super", c.f(2, 0))
  }
}
