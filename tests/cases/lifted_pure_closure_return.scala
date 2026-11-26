// teq: --wconf msg=A.pure.expression.does.nothing:s,msg=Non.local.returns.are.no.longer.supported:s
// A lambda with a `return` as a pure statement leaves the method's result as it is (its class files:
// tests/support/abi/lifted_pure_closure_return.scala; the member set and the run pinned, the body unpinned).
object Main {
  def run(): Int = {
    (() => { return 3 })
    7
  }
  def main(args: Array[String]): Unit = println(run())
}
