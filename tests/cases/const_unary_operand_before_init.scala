// A primitive unary operator over a constant val read before its initialiser has run: dotty's
// `Select(h.x, unary_-)` has the constant type ConstFold gives it, and FirstTransform's constToLiteral takes it to
// `{ h.x; -42 }` (dropOp, its operand's effect kept), so `before` is -42, not the negated field's default; with a pure
// operand the literal alone. From the interim review of tr/cut-head-tail-c2 633a6bca3d (P1-1).
class Holder:
  def h: Holder = this
  val before: Int = -h.x
  val pure: Int = -this.x
  val x: 42 = 42
@main def main(): Unit =
  val k = new Holder()
  println(k.before)
  println(k.pure)
