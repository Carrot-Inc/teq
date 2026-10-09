// Adapted from scala3 tests/run/t4935.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App, Scala 2 syntax of for.
object Test {
  def main(args: Array[String]): Unit = ()
  for i <- 0 to 1 do {
    val a = Foo
  }
}

object Foo {
  println("hello")
}
