// jars: scala-library
// teq: --deprecation --werror --wconf msg=Use.LazyListIterable:s
// The std's `Stream` class carries scala-library's message: a build that silences its warning by that text
// passes under `--werror`,
// with scala-library's jar on the class path beside the std too.
object Main {
  val kept: scala.collection.immutable.Stream[Int] = null
  def main(a: Array[String]): Unit = println(kept == null)
}
