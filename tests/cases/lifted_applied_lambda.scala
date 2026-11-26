// A lambda applied where it is written evaluates its argument once (its class files:
// tests/support/abi/lifted_applied_lambda.scala).
object Main {
  def arg(): Int = { println("arg"); 3 }
  def run(): Int = ((x: Int) => x + 1)(arg())
  def main(args: Array[String]): Unit = println(run())
}
