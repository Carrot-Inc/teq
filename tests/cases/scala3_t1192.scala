// Adapted from scala3 tests/run/t1192.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App, Scala 2 syntax of for.
object Test {
  def main(args: Array[String]): Unit = ()
  val v1: Array[Array[Int]] = Array(Array(1, 2), Array(3, 4))
  def f[T](w: Array[Array[T]]): Unit = {
    for r <- w do println(r.toList.toString)
  }
  f(v1)
}
