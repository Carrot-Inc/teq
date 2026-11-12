// Adapted from scala3 tests/run/i1856.scala (Apache-2.0, see tests/scala3/README.md); replaced: the recursive
// `@threadUnsafe` member, whose half is scala3_i1856. A local lazy val's
// initialiser runs under its holder's monitor, which the thread re-enters, so its recursion recomputes.
import scala.annotation.threadUnsafe

object Test {
  def lzy2 = {
    var countLocal = 0
    @threadUnsafe lazy val lzyLocal: Int = {
      if (countLocal < 10) {
        println(s"Iteration $countLocal")
        countLocal += 1
        lzyLocal
      } else 42
    }
    lzyLocal
  }

  def main(args: Array[String]): Unit = {
    println(lzy2)
  }
}
