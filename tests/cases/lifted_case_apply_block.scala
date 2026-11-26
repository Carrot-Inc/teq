// A block of a case class's apply and a local def that ends in a literal runs to the literal (its class files:
// tests/support/abi/lifted_case_apply_block.scala).
case class Value(n: Int)
object Main {
  def run(n: Int): Int = { val v = Value(n); def gone(): Int = 9; 2 }
  def main(args: Array[String]): Unit = println(run(args.length))
}
