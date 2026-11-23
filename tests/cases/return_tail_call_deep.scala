// A call of a def by itself under a `return` is the def's loop going round again wherever the `return`
// stands, as a call in return position is: a hundred thousand calls deep, no stack overflows.
object Deep {
  final def statement(n: Int): Int = { if n > 0 then return statement(n - 1); 0 }
  final def inVal(n: Int): Long = { val r: Long = if n > 0 then return inVal(n - 1) else 7L; r }
  final def inCaseValue(n: Int): Int = { val r: Int = n match { case 0 => 0; case _ => return inCaseValue(n - 1) }; r }
  final def inWhile(n: Int): Int = { var i = 0; while i < 3 do { if n > 0 then return inWhile(n - 1); i += 1 }; i }
  final def inCatch(n: Int): Int = try { if n > 0 then throw new RuntimeException(); 0 } catch { case _: RuntimeException => return inCatch(n - 1) }
  final def wraps(n: Int): Int = return (if n == 0 then 0 else wraps(n - 1))
}

class Counter(val start: Int) {
  final def down(n: Int, other: Counter): Int = { if n > 0 then return down(n - 1, other); other.start }
}

object Main {
  def main(args: Array[String]): Unit = {
    val n = 100000
    println(Deep.statement(n))
    println(Deep.inVal(n))
    println(Deep.inCaseValue(n))
    println(Deep.inWhile(n))
    println(Deep.inCatch(n))
    println(Deep.wraps(n))
    println(new Counter(1).down(n, new Counter(9)))
  }
}
