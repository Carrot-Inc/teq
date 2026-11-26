// teq: --wconf msg=A.pure.expression.does.nothing:s
// A lambda as a pure statement over a var leaves the var's updates as they are (its class files:
// tests/support/abi/lifted_pure_closure_var_box.scala; the member set and the run pinned, the body unpinned).
object Main {
  def run(n: Int): Int = {
    var x = n
    (() => x)
    x += 1
    x
  }
  def main(args: Array[String]): Unit = println(run(args.length))
}
