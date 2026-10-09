// Adapted from scala3 tests/run/absoverride.scala (Apache-2.0, see tests/scala3/README.md); replaced: the abstract type member `T` is a type parameter, `synchronized` is dropped, the local class is a top-level one.
abstract class AbsIterator[T] {
  def hasNext: Boolean
  def next: T
}

trait RichIterator[T] extends AbsIterator[T] {
  def foreach(f: T => Unit): Unit = {
    while (hasNext) do f(next)
  }
}

class StringIterator(s: String) extends AbsIterator[Char] {
  private var i = 0
  def hasNext = i < s.length()
  def next = { val x = s.charAt(i); i += 1; println("next: " + x); x }
}

trait SyncIterator[T] extends AbsIterator[T] {
  abstract override def hasNext: Boolean =
    super.hasNext
  abstract override def next: T = {
    println("<sync>"); val x = super.next; println("</sync>"); x
  }
}
trait LoggedIterator[T] extends AbsIterator[T] {
  abstract override def next: T = {
    val x = super.next; println("log: " + x); x
  }
}
class Iter2(s: String) extends StringIterator(s)
               with SyncIterator[Char] with LoggedIterator[Char]
class Iter extends StringIterator("jvm") with RichIterator[Char] with SyncIterator[Char] with LoggedIterator[Char]
object Test {
  def main(args: Array[String]): Unit = {
    val iter = new Iter
    iter.foreach(println)
  }
}
