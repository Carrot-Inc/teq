// A `try` with a handler as an element of a List, Seq, Vector or Array literal: teq writes the literal over a
// `new ArraySeq$ofRef` (or the array) that stands uninitialised under the elements, so the try's stash holds it at
// its own type, as dotty's genLoadTry stashes the whole stack.
object Out {
  def work(n: Int): Int = if n == 1 then throw new IllegalStateException("i") else if n == 2 then throw new RuntimeException("r") else n * 10
}
object Main {
  def row(name: String)(f: Int => Any): Unit =
    println(name + ": " + (0 to 4).map(n => try f(n).toString catch { case e: RuntimeException => "E:" + e.getMessage }).mkString(" "))
  def main(args: Array[String]): Unit = {
    row("list")(n => List(1, try Out.work(n) catch { case _: IllegalStateException => -1 }, 3).sum)
    row("list, two")(n => List(try Out.work(n) catch { case _: IllegalStateException => -1 }, try Out.work(n + 2) catch { case _: IllegalStateException => -2 }))
    row("list of strings")(n => List("a", try "v" + Out.work(n) catch { case _: IllegalStateException => "c" }))
    row("seq")(n => Seq(1, try Out.work(n) catch { case _: IllegalStateException => -1 }))
    row("vector")(n => Vector(try Out.work(n) catch { case _: IllegalStateException => -1 }, 2))
    row("array")(n => Array(1, try Out.work(n) catch { case _: IllegalStateException => -1 }).sum)
  }
}
