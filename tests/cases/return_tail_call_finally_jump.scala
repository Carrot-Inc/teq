// A call of a def by itself under a `return` in a `finally` block is the loop's jump (dotty's TailRec enters a
// finalizer as no tail position, and the `return` in it leaves the method: a tail position); the `return`
// discards the exception the `try` threw, so only the innermost frame's exception escapes. A hundred thousand deep.
object F {
  var finals = 0
  final def finExc(n: Int): Int = { try { if n >= 0 then throw new RuntimeException("e") } finally { finals += 1; if n > 0 then return finExc(n - 1) }; 0 }
}
object Main {
  def r(v: => Any): String = try v.toString catch { case e: RuntimeException => e.getClass.getSimpleName + ":" + e.getMessage }
  def main(args: Array[String]): Unit = {
    println(r(F.finExc(3)) + " " + F.finals)
    F.finals = 0
    println(r(F.finExc(100000)) + " " + F.finals)
  }
}
