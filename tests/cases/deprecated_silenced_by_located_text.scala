// A warning names a top-level definition with its package's full name: a `@nowarn` that matches the text
// scalac gives a class of a nested package silences that warning, and the other name of the program, which the
// text does not match, is still warned of: the build's summary counts one.
import scala.annotation.nowarn
object Main {
  @nowarn("msg=class Stream in package scala.collection.immutable is deprecated")
  def size(s: scala.collection.immutable.Stream[Int]): Int = if s == null then 0 else 1
  def count(t: Traversable[Int]): Int = if t == null then 0 else 2
  def main(a: Array[String]): Unit = println(size(null) + count(null))
}
