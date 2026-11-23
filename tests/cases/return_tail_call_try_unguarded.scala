// A call of a def by itself under a `return` in a `try`'s body with an unguarded `catch` whose handler
// completes: scalac 3.8.4 makes it a loop, which gives the call's value; teq keeps the call under every
// `try` body, so a hundred thousand deep it overflows the stack (a row of teq's).
object U {
  var handled = 0
  final def plain(n: Int): Int = try { if n > 0 then return plain(n - 1); 8 } catch { case _: ArithmeticException => -1 }
  final def handles(n: Int): Int =
    try { if n > 0 then return handles(n - 1); throw new ArithmeticException("a") } catch { case _: ArithmeticException => handled += 1; n + 100 }
}
object Main {
  def main(args: Array[String]): Unit = {
    println(U.plain(3))
    println(U.handles(3).toString + " " + U.handled)
    println(U.plain(100000))
    U.handled = 0
    println(U.handles(100000).toString + " " + U.handled)
  }
}
