// Adapted from scala3 tests/run/i1463.scala (Apache-2.0, see tests/scala3/README.md); replaced: the asserts print their results.
object Test {
  case object Bob { override def equals(other: Any) = true }

  class Bob2 {
    override def equals(other: Any) = true
  }
  val Bob2 = new Bob2

  def f0(x: Any) = x match { case Bob2 => Bob2 }
  def f1(x: Any) = x match { case Bob => Bob }

  def main(args: Array[String]): Unit = {
    println(f0(Bob2) eq Bob2)
    println(f0(0) eq Bob2)
    println(f0(Nil) eq Bob2)

    println(f1(Bob) eq Bob)
    println(f1(0) eq Bob)
    println(f1(Nil) eq Bob)
  }
}
