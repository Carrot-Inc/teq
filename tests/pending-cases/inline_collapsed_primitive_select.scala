// A primitive operation ConstFold collapses into its operand (`+x`, `x.toInt` on an Int) inside an inline body:
// the copy keeps the operation as its typed record, the walk retypes it as InlineTyper does, and FirstTransform
// folds it to the constant with the qualifier's effects. Pending: the walk's retype_member aborts on the collapsed
// node (substitution.rs) and FirstTransform's fold of collapsed nodes is not on integration yet. scalac 3.8.4's output.
class Holder:
  def h: Holder = { println("h"); this }
  val a: Int = Holder.plusSel(this)
  val b: Int = Holder.convSel(this)
  val c: Int = Holder.plusInline(h.x)
  val x: 42 = 42
object Holder:
  inline def plusSel(k: Holder): Int = +k.h.x
  inline def convSel(k: Holder): Int = k.h.x.toInt
  inline def plusInline(inline v: Int): Int = +v
@main def main(): Unit =
  val k = new Holder()
  println(List(k.a, k.b, k.c).mkString(","))
