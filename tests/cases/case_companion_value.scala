// A case class without a written companion has one (dotty's `Desugar.classDef` `companionDefs`): an
// object of the class's place, a value like a written one on every target, printed by its name (its
// `toString`), applied as its `apply`; a case class nested in a class has one per
// enclosing instance, its constructor defaults read through it. Master took the name as a call of the
// constructor, which lacked its arguments.
case class F(n: Int)
class O(val m: Int):
  case class K(n: Int)
  case class D(x: Int = m, y: String = "d")
@main def run(): Unit =
  println(F.apply(1))
  println(F.toString)
  println((F: Any) != null)
  println(F == F)
  val f = F
  println(f(3))
  val o = new O(7)
  val c: Any = o.K
  println(c)
  println(c == o.K)
  println(o.K(5))
  println(o.D())
  val d: Any = o.D
  println(d)
  println(o.D(1))
