// A local def and a lambda that read a field only in dropped branches run to their live branches (its class files:
// tests/support/abi/lifted_dead_branch_this_capture.scala).
class Box(val n: Int) {
  def run(): Int = {
    def deadThis(): Int = if (false) n else 7
    val f = () => if (false) n else 8
    deadThis() + f()
  }
}
object Main { def main(args: Array[String]): Unit = println(new Box(3).run()) }
