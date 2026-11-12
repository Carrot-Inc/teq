// A recursive `@threadUnsafe` lazy member recomputes under scalac (dotty's transformMemberDefThreadUnsafe, the flag
// set after the right-hand side), and over teq's unsynchronised holder of a marked member; a local one recomputes
// under its holder's monitor. Copied from scala3 tests/run/i1856.scala (Apache-2.0, see tests/scala3/README.md).
import scala.annotation.threadUnsafe

object Test {
  var count = 0
  @threadUnsafe lazy val lzy: Int = {
    if (count < 10) {
      println(s"Iteration $count")
      count += 1
      lzy
    } else 42
  }

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
    println(lzy)
    println(lzy2)
  }
}
