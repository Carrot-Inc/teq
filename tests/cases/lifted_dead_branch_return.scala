// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
// A lambda whose `return` stands only in a dropped branch returns its own result (its class files:
// tests/support/abi/lifted_dead_branch_return.scala).
object Main {
  def run(): Int = {
    val f = () => { if (false) return 3; 7 }
    f()
  }
  def main(args: Array[String]): Unit = println(run())
}
