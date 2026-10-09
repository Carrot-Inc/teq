// Adapted from scala3 tests/run/t8888.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
class C {
  final def resume: Unit = (this: Any) match {
    case x : C => (x: Any) match {
      case y : C =>
        () => (x, y) // used to trigger a ClassFormatError under -Ydelambdafy:method
    }
  }
}

object Test {
  def main(args: Array[String]): Unit = ()
  new C().resume
}
