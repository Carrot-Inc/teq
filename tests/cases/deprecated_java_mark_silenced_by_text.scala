// A Java method marked `@Deprecated` is warned of with the text scalac gives every such mark: a `@nowarn` that
// matches that text silences it, and the other name of the program, which the text does not match, is still
// warned of: the build's summary counts one.
import scala.annotation.nowarn
object Main {
  @nowarn("msg=see corresponding Javadoc")
  def plus(s: String): String = java.net.URLEncoder.encode(s)
  def count(t: Traversable[Int]): Int = if t == null then 0 else 2
  def main(a: Array[String]): Unit = println(plus("a b") + count(null))
}
