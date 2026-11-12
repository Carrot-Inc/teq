// A primitive operation ConstFold collapses into its operand (`+x`, `x.toInt` on an
// Int) inside an inline body: InlineTyper retypes it, and FirstTransform folds it to the
// constant with the qualifier's effects. Pending on the retyping of the collapsed node and
// on FirstTransform's fold of collapsed nodes. The expectation is scalac 3.8.4's output.
class Holder:
  def h: Holder = { println("h"); this }
  val plus: Int = Holder.plus(h.x)
  val neg: Int = Holder.neg(h.x)
  val conv: Int = Holder.conv(h.x)
  val plainPlus: Int = Holder.plainPlus(h.x)
  val x: 42 = 42
object Holder:
  inline def plus(inline v: Int): Int = +v
  inline def neg(inline v: Int): Int = -v
  inline def conv(inline v: Int): Int = v.toInt
  inline def plainPlus(v: Int): Int = +v
@main def main(): Unit =
  val k = new Holder()
  println(List(k.plus, k.neg, k.conv, k.plainPlus).mkString(","))
