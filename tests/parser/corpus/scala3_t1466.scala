// Adapted from scala3 tests/run/t1466.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object IOvervalueMyPrivacy {
  private[this] var i = 0
  def go = {
    List(1,2,3).foreach(i += _)
    i
  }
}

object Test {
  def main(args: Array[String]): Unit = ()
  assert(IOvervalueMyPrivacy.go == 6)
}
