// Copied from scala3 tests/run/tuple-deconstruct-wildcard-sideeffect.scala (Apache-2.0, see tests/scala3/README.md).
object Test {
  def foo(): Int = { println("Side effect"); 2 }

  def main(args: Array[String]) =
    val (x, _) = (1, foo()) // the result of foo is ignored but it must be executed anyway!
    println(x)
}
