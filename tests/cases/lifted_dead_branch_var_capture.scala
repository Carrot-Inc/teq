// A var that a lambda reads only in a dropped branch keeps its updates outside the lambda (its class files:
// tests/support/abi/lifted_dead_branch_var_capture.scala).
object Main {
  def run(n: Int): Int = {
    var x = n
    val f = () => { if (false) x += 1; 7 }
    x += 2
    f() + x
  }
  def main(args: Array[String]): Unit = println(run(args.length))
}
