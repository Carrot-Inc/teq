// jars: scala-library scala2-lib
// teq: --target jvm --std=scala-library
// expect: 10:11: error: the constructor of Version is private to Version
// expect: 11:11: error: the constructor of Guarded is private to Guarded
// expect: 2 errors found
// A private constructor of a jar's class is out of reach as a source class's is (these from a
// Scala 2.13 library); a case class keeps Scala 2's public `apply` beside it.
import scala2lib.*
@main def main(): Unit =
  val a = new Version(1, 2)
  val b = new Guarded(3)
  println(Version(1, 2))
  println(Guarded.make(4).n)
