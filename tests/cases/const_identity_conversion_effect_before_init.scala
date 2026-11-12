// An Int's toInt over a constant val read before its initialiser has run,
// selected on a method with an effect: FirstTransform's constToLiteral takes
// Select(h.x, toInt) of constant type to { h.x; 42 } (TreeInfo.scala:763-783),
// so h runs and `before` is 42.
class Holder:
  def h: Holder = { println("h"); this }
  val before = h.x.toInt
  val x: 42 = 42
@main def main(): Unit = println(new Holder().before)
