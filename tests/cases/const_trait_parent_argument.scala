// A trait parent's argument, `h.x` of a constant val read before Holder has set it: FirstTransform transforms a
// template's parents as its body (MegaPhase.scala:371-377), so the argument is its literal and `seen` is 42, as for a
// class parent's argument. From Codex's review of tr/cut-head-tail-c2 6689715658 (P1-A).
trait Parent(v: Int):
  val seen: Int = v
class Child(h: Holder) extends Parent(h.x)
class Holder:
  val before: Int = new Child(this).seen
  val x: 42 = 42
@main def main(): Unit = println(new Holder().before)
