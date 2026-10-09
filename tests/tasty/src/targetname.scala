package fix.tname

import scala.annotation.targetName

// Overloads that erase alike, told apart on the JVM by `@targetName` (izumi-reflect's
// `Inspector.extractVariance`), an operator given a plain name, and a call from the jar's own
// body to each.
object Variance:
  def extract(xs: List[Int]): String = "ints " + xs.sum
  @targetName("extractStrings")
  def extract(xs: List[String]): String = "strings " + xs.mkString
  def both: String = extract(List(1, 2)) + " / " + extract(List("a", "b"))

final class Counter(val n: Int):
  @targetName("add")
  def +(o: Counter): Counter = new Counter(n + o.n)
  @targetName("addAll")
  def +(os: List[Counter]): Counter = new Counter(n + os.map(_.n).sum)
  override def toString: String = s"Counter($n)"
