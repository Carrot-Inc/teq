// A numeric unary_+ over a constant val read before its initialiser has run, its operand a block with an effect: dotty's
// Select({ println("pre"); h.x }, unary_+) has the constant type ConstFold gives it (ConstFold.scala:24-29, 49-54), and
// FirstTransform's constToLiteral takes it to { { println("pre"); this.h.x }; 42 } (TreeInfo.scala:763-783), so the
// block runs and `before` is 42, not the field's default. teq's lowering of an Int's unary_+ hands on the operand's node.
// From the Codex review of tr/cut-head-tail-c2 e1b01a08fe (P1-1, the rest of transformSelect).
class Holder:
  def h: Holder = this
  val before: Int = +{ println("pre"); h.x }
  val x: 42 = 42
@main def main(): Unit = println(new Holder().before)
