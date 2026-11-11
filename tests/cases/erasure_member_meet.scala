trait A { type T <: CharSequence }
trait B { type T <: java.io.Serializable }
trait C { type T <: String }
object Api:
  def two(c: B & A)(x: c.T): Int = 1
  def three(c: C & A & B)(x: c.T): Int = 2
object Both extends A with B with C { type T = String }
object Main:
  def main(args: Array[String]): Unit =
    println(Api.two(Both)("ok"))
    println(Api.three(Both)("ok"))
