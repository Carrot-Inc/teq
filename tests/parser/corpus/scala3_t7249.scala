// Adapted from scala3 tests/run/t7249.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  def bnToLambda(s: => String): () => String = () => s
  var x: () => String = () => sys.error("Nope")
  val y = bnToLambda { x() }
  x = () => "Yup!"
  println(y())
}
