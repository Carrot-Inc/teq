// Adapted from scala3 tests/run/i2266.scala (Apache-2.0, see tests/scala3/README.md): extends App replaced.
object Test {
  def main(args: Array[String]): Unit = ()
  lazy val x: true = { println("X"); true }
  println(x)

  object Inner {
    println("Y")  // not printed
    inline val y = 1
  }
  println(Inner.y)

  inline val MV = Int.MaxValue
}

