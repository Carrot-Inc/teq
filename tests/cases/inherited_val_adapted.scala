// An overriding final val of no written type takes its right-hand side's type after the adaptation to the inherited
// type: a numeric widening, a given conversion, the discarding into Unit, an eta-expansion, a function literal
// implementing a single abstract method; the value read through the object and through the parent.
import scala.language.implicitConversions
trait Base {
  def wide: Long
  def conv: String
  def unit: Unit
  def eta: Int => Int
  def sam: java.util.function.IntUnaryOperator
}
object O extends Base {
  def small: Byte = 3
  def three: Int = { println("three"); 3 }
  given Conversion[Int, String] with { def apply(i: Int): String = "given=" + i }
  def inc(i: Int): Int = i + 1
  override final val wide = small
  override final val conv = 42
  override final val unit = three
  override final val eta = inc
  override final val sam = (i: Int) => i * 2
}
@main def main(): Unit =
  val b: Base = O
  println(O.wide + 1L)
  println(b.wide)
  println(O.conv.length)
  println(b.conv)
  println(O.unit == b.unit)
  println(O.eta(1) + b.eta(2))
  println(O.sam.applyAsInt(4) + b.sam.applyAsInt(5))
