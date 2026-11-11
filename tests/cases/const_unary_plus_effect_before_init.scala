// A numeric unary_+ over a constant val read before its initialiser has run, selected on a method with an effect:
// FirstTransform's constToLiteral takes Select(h.x, unary_+) of constant type to { h.x; 42 } (TreeInfo.scala:763-783),
// so h runs and `before` is 42.
// From the Codex review of tr/cut-head-tail-c2 e1b01a08fe (P1-1, the rest of transformSelect).
class Holder:
  def h: Holder = { println("h"); this }
  val before = +h.x
  val x: 42 = 42
@main def main(): Unit = println(new Holder().before)
