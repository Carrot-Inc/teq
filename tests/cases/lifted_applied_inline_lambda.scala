// An inline def's lambda applied at the call runs its body (its class files:
// tests/support/abi/lifted_applied_inline_lambda.scala).
object Main {
  inline def make = (x: Int) => x + 1
  def run(n: Int): Int = make(n)
  def main(args: Array[String]): Unit = println(run(args.length))
}
