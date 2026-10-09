// Adapted from scala3 tests/run/isInstanceOf-eval.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  lazy val any = {
    println(1)
    1: Any
  }

  any.isInstanceOf[Int]

  lazy val int = {
    println(2)
    2
  }

  int.isInstanceOf[Int]
}
