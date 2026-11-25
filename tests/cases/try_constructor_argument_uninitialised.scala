// A `try` with a handler as an argument of a constructor: the object `new` made stands uninitialised on the operand
// stack under the try, so the JVM writer stashes it with the other operands at its own type and loads it again after
// the handler (dotty's genLoadTry stashes the whole stack); the first, a later, a nested and both arguments, and a
// case class's apply, which teq writes as its constructor.
object Out {
  def work(n: Int): Int = if n == 1 then throw new IllegalStateException("i") else if n == 2 then throw new RuntimeException("r") else n * 10
}
class Box(val v: Int)
class Pair(val a: Int, val b: Int) { def sum: Int = a + b }
class Text(val s: String)
case class Pt(x: Int, y: Int)
object Main {
  def row(name: String)(f: Int => Any): Unit =
    println(name + ": " + (0 to 4).map(n => try f(n).toString catch { case e: RuntimeException => "E:" + e.getMessage }).mkString(" "))
  def main(args: Array[String]): Unit = {
    row("first")(n => new Box(try Out.work(n) catch { case _: IllegalStateException => -1 }).v)
    row("later")(n => new Pair(n, try Out.work(n) catch { case _: IllegalStateException => -1 }).sum)
    row("nested")(n => new Box(new Pair(n, try Out.work(n) catch { case _: IllegalStateException => -1 }).sum).v)
    row("both")(n => new Pair(try Out.work(n) catch { case _: IllegalStateException => -1 }, try Out.work(n + 1) catch { case _: IllegalStateException => -2 }).sum)
    row("reference")(n => new Text(try "v" + Out.work(n) catch { case _: IllegalStateException => "c" }).s)
    row("case apply")(n => Pt(n, try Out.work(n) catch { case _: IllegalStateException => -1 }).y)
  }
}
