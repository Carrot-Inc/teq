// Adapted from scala3 tests/run/i6341.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  class Config(val t1: Int)

  inline def m(t2:Int) = t2

  m(new Config(3).t1)
}
