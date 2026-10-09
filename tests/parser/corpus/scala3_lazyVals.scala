// Copied from scala3 tests/run/lazyVals.scala (Apache-2.0, see tests/scala3/README.md).
object Test {
 def foo = {
   lazy val s = 42 // needs LazyIntHolder
   s
  }

  def main(args: Array[String]): Unit = println(foo)
}
