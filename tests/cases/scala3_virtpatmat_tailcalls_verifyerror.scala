// Adapted from scala3 tests/run/virtpatmat_tailcalls_verifyerror.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App, Scala 2 syntax of if.
// shouldn't result in a verify error when run...
object Test {
  def main(args: Array[String]): Unit = ()
  @annotation.tailrec
  final def test(meh: Boolean): Boolean = {
    Some("a") match {
      case x =>
        x match {
          case Some(_) => if meh then test(false) else false
          case _ => test(false)
        }
    }
  }
  println(test(true))
}
