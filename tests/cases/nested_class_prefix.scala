// `new p.Inner` takes `p` as the enclosing instance wherever it is written (a qualified `this`
// among the prefixes): another instance
// inside the outer class, or a value outside it.
class O(val n: Int):
  class I:
    def value = O.this.n
  def make(o: O) = new o.I
@main def run(): Unit =
  println(new O(1).make(new O(2)).value)
  val o = new O(3)
  println(new o.I().value)
  qualified()

class P(val n: Int, val p: P):
  class C:
    def get = n
  def make = new P.this.p.C
  def own = new P.this.C

def qualified(): Unit =
  val q = new P(1, new P(2, null))
  println(q.make.get)
  println(q.own.get)
