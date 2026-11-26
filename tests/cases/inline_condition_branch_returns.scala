// An `if` whose branch is an inline method's expansion that returns: `true && pass({ return true })` reduces to the
// expansion, whose inline argument is the `return`. The method returns `true` from the branch; scalac writes nothing
// after the return, and neither does teq.
object Main {
  inline def pass(inline p: Boolean): Boolean = { println("pass"); p }
  def work(x: Int): Boolean = {
    val y = if (x > 0) true && pass({ return true }) else false
    y
  }
  def main(args: Array[String]): Unit = { println(work(1)); println(work(-1)) }
}
