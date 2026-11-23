// expected: by hand
// A call of a def by itself under a `return` in a `try`'s body is a call whatever the handlers: here a
// handler throws what the case of an outer frame takes, so each frame's handler matters. scalac 3.8.4 makes
// it a loop and prints `java.lang.IllegalArgumentException: x 1` (no outer frame is left to take it); the
// call gives `7 1`.
object R {
  var count = 0
  final def rethrow(n: Int): Int =
    try { if n > 0 then return rethrow(n - 1); throw new IllegalStateException("s") }
    catch {
      case _: IllegalStateException => count += 1; if n == 0 then throw new IllegalArgumentException("x") else 5
      case _: IllegalArgumentException => 7
    }
}
object Main {
  def main(args: Array[String]): Unit =
    println((try R.rethrow(3).toString catch { case e: Throwable => e.toString }) + " " + R.count)
}
