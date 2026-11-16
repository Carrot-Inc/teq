// teq: --main Main
// `@nowarn` on a definition and on an object silences the deprecation warnings of the names written in them,
// and a name written outside them is still warned of: the build's summary counts one.
import scala.annotation.nowarn
object Main {
  @nowarn("cat=deprecation") def size(s: Stream[Int]): Int = if s == null then 0 else 1
  def count(t: Traversable[Int]): Int = if t == null then 0 else 2
  def main(a: Array[String]): Unit = { println(size(null) + count(null)); Quiet.main(a) }
}
@nowarn("cat=deprecation") object Quiet extends App { println("quiet") }
