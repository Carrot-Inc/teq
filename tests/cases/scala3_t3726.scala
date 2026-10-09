// Adapted from scala3 tests/run/t3726.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  def test(f: () => Int) = {
    val x = f()
    5
  }

  println(test(() => { println("hi there"); 0 }))
}
