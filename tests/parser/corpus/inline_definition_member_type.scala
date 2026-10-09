// teq: --inline-substitution
// A member an inline body selects on its parameter is the one the definition resolved, its type
// seen from the argument at the expansion, as scalac 3.8.4's `InlineTyper` computes it: `member(O)`
// is an `O.type`, since `O` overrides `value` with that result type. The expansion by
// substitution gives scalac's; the retype path reports `found B, required O`.
trait B:
  def value: B
object O extends B:
  def value: O.type = this
transparent inline def member(x: B) = x.value
@main def run(): Unit =
  val r: O.type = member(O)
  println(r == O)
