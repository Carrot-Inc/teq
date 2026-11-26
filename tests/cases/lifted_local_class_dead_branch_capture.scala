// A local class whose method reads a parameter only in a dropped branch runs to its live branch (its class files:
// tests/support/abi/lifted_local_class_dead_branch_capture.scala).
object Main {
  def run(n: Int): Int = {
    class Local { def value(): Int = if (false) n else 7 }
    new Local().value()
  }
  def main(args: Array[String]): Unit = println(run(args.length))
}
