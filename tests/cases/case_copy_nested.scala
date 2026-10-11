// The copy of a case class nested in a class constructs as dotty's synthesized member does
// (`copyMeths`' `creatorExpr`, inside the class): with the receiver's own enclosing instance,
// wherever the copy is written, a partial application's too.
class O(val base: Int):
  case class C(a: Int)(val b: Int):
    def total = base + a + b
    def bump = copy(a = a + 1)(b)
  case class D(a: Int):
    def total = base + a
def again(o: O, c: o.C): Int = c.copy(a = 7)(8).total

@main def run(): Unit =
  val o = new O(10)
  val c = o.C(1)(2)
  println(c.copy(a = 3)(4).total)
  val f = c.copy(a = 5)
  println(f(6).total)
  println(c.bump.total)
  println(again(o, c))
  val p = new O(100)
  val d = p.C(0)(0)
  println(d.copy(a = 1)(1).total)
  println(p.D(1).copy(a = 2).total)
