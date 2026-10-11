// The companion of a case class nested in a class, written or synthesized, constructs inside
// the class (`Desugar.classDef`'s `apply`): its enclosing instance is the one the companion is
// selected on, `o` of `o.C(1)`, the prefix `Typer.newExpr` keeps; inside the companion its own
// enclosing instance; through another value holding the companion, that value's.
class O(val base: Int):
  case class C(a: Int):
    def total = base + a
  object C:
    def helper = 5
    def make: C = C(1)
  case class D(a: Int)(val b: Int):
    def total = base + a + b

@main def run(): Unit =
  val o = new O(10)
  println(o.C(1).total)
  println(o.C.helper)
  println(o.C(2) match { case o.C(a) => a })
  val p = new O(20)
  println(p.D(1)(2).copy(a = 3)(4).total)
  println(o.C.make.total)
  val m = o.C
  println(m(2).total)
  def pick(x: O) = x.C
  println(pick(p)(3).total)
