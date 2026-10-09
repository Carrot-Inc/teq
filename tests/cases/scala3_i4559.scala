// Adapted from scala3 tests/run/i4559.scala (Apache-2.0, see tests/scala3/README.md).
trait A {
  println(s"super[A] init")
  lazy val x = { println("super[A].x()"); 123 }
}

class B extends A {
  override lazy val x = { println("x()"); 456 }
}

object Test { def main(args: Array[String]): Unit = { new B().x } }
