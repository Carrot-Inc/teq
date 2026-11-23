// A call of a def by itself under a `return` that an operand holds is the def's loop going round
// again too: a hundred thousand calls deep, no stack overflows.
object Operand {
  final def concat(n: Int): String = "x" + (if n > 0 then return concat(n - 1) else "e")
  final def defaulted(n: Int, acc: Int = 7): Int = 1 + (if n > 0 then return defaulted(n - 1) else acc)
  final def branches(n: Int): String = "x" + (return (if n > 0 then branches(n - 1) else "e"))
}

object Main {
  def main(args: Array[String]): Unit = {
    val n = 100000
    println(Operand.concat(n))
    println(Operand.defaulted(n, 3))
    println(Operand.branches(n))
  }
}
