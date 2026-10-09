// Adapted from scala3 tests/run/t7300.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  // single line comment in multi line comment
  /*//*/ val x = 1 */*/
  val x = 2
  println(x)

  // single line comment in nested multi line comment
  /*/*//*/ val y = 1 */*/*/
  val y = 2
  println(y)
}
