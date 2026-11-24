package use
final class Holder(val b: Sub)
class Sub extends base.Base:
  // An inner class does not derive from Base, so a protected read from it needs an accessor unless it is folded.
  class In(h: Holder):
    def read: Int = h.b.k + h.b.m
  def read(h: Holder): Int = new In(h).read
@main def run(): Unit =
  val s = new Sub
  println(s.read(new Holder(s)))
