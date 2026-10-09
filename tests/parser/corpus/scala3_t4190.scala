// Adapted from scala3 tests/run/t4190.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
import collection.mutable.*

object Test {
  def main(args: Array[String]): Unit = ()
  val x: ArrayBuffer[String] = ArrayBuffer("a", "b", "c")
  x.view map (_ + "0") foreach println
}
