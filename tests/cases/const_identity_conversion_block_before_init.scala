// An Int's toInt over a constant val read before its initialiser has run, its
// receiver a block with an effect: dotty's Select({ println("pre"); h.x }, toInt)
// has the constant type ConstFold gives it (ConstFold.scala:24-29, 49-54, 85-99),
// and FirstTransform's constToLiteral takes it to { { println("pre"); this.h.x }; 42 }
// (TreeInfo.scala:763-783), so the block runs and `before` is 42.
class Holder:
  def h: Holder = this
  val before: Int = { println("pre"); h.x }.toInt
  val x: 42 = 42
@main def main(): Unit = println(new Holder().before)
