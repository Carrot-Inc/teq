// Adapted from scala3 tests/run/t3835.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  // work around optimizer bug SI-5672  -- generates wrong bytecode for switches in arguments
  // virtpatmat happily emits a switch for a one-case switch
  // this is not the focus of this test, hence the temporary workaround
  def a = (1, 2, 3) match { case (r, θ, φ) => r + θ + φ }
  println(a)
  def b = (1 match { case é => é })
  println(b)
}
