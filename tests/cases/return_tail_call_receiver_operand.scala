// A call of a def by itself on another instance under a `return` that an operand holds is the def's loop going round
// again, a hundred thousand calls deep (on JavaScript it stays a call, which overflows the stack: a pending row).
class Cell(val k: Int) {
  final def down(n: Int): Int = k + (if n > 0 then return new Cell(k + 1).down(n - 1) else 0)
}

object Main {
  def main(args: Array[String]): Unit = {
    println(new Cell(0).down(100000))
  }
}
