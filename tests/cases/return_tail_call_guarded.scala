// expected: by hand
// A call of a def by itself under a `return` that a finalizer or a guarded handler stands around is a
// call: the finalizer runs on each way out, the handler sees each frame's exception. scalac 3.8.4 makes
// both a loop and prints `0 1` (the finalizer run once for 1001 returns) and throws
// `java.lang.IllegalStateException: e` (the handler of the frame the guard holds in lost). The `mixed`
// defs loop by a call nothing guards as well: their guarded call stays a call.
object Guarded {
  var count: Int = 0
  final def throughFinally(n: Int): Int = try { if n > 0 then return throughFinally(n - 1); 0 } finally count += 1
  final def throughCatch(n: Int): Int =
    try { if n > 0 then return throughCatch(n - 1); throw new IllegalStateException("e") }
    catch { case _: IllegalStateException if n == 5 => 42 }
  var mixedCount: Int = 0
  final def mixedFinally(n: Int): Int =
    if n > 2000 then return mixedFinally(n - 1)
    else try { if n > 0 then return mixedFinally(n - 1); 0 } finally mixedCount += 1
  final def mixedCatch(n: Int): Int =
    if n > 20 then return mixedCatch(n - 1)
    else
      try { if n > 0 then return mixedCatch(n - 1); throw new IllegalStateException("e") }
      catch { case _: IllegalStateException if n == 5 => 42 }
}

object Main {
  def main(args: Array[String]): Unit = {
    println(Guarded.throughFinally(1000).toString + " " + Guarded.count)
    println(try Guarded.throughCatch(10).toString catch { case e: Throwable => e.toString })
    println(Guarded.mixedFinally(1000000).toString + " " + Guarded.mixedCount)
    println(try Guarded.mixedCatch(1000000).toString catch { case e: Throwable => e.toString })
  }
}
