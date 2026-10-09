// Adapted from scala3 tests/run/infix.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
import annotation.targetName
object Test {
  def main(args: Array[String]): Unit = ()

  case class Rational(n: Int, d: Int) {
    infix def + (that: Rational) =
      Rational(this.n * that.d + that.n * this.d, this.d * that.d)
    @targetName("multiply") infix def * (that: Rational) =
      Rational(this.n * that.n, this.d * that.d)
  }

  val r1 = Rational(1,2)
  val r2 = Rational(2,3)
  println(r1 * r2)
  println(r1 + r2)
}