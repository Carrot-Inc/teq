// A case class nested in a class compares its enclosing instances by identity in its synthesized
// `equals`, after the class test and before the fields and `canEqual` (dotty's `equalsBody` tests `that`
// against `C`'s `this`-prefixed type, to which `PatternMatcher.addOuterTest` adds `x$0.$outer eq
// this.$outer`), the enclosing instance's own `equals` never called; a class extending it from another
// enclosing class compares the case class's outer. Master compared the fields alone.
class O:
  override def equals(x: Any): Boolean = { println("outer equals"); true }
  case class I(n: Int)
  case class K(n: Int):
    override def canEqual(x: Any): Boolean = { println("canEqual"); true }
val a = new O
val b = new O
class Holder:
  class J extends a.I(1)
@main def run(): Unit =
  println(a.I(1) == b.I(1))
  println(a.I(1) == a.I(1))
  println(a.K(1) == b.K(1))
  println(a.K(1) == a.K(1))
  val x: O#I = a.I(1)
  println(x match { case b.I(n) => "wrong"; case _ => "right" })
  val h1 = new Holder
  val h2 = new Holder
  println(new h1.J == new h2.J)
  println(new h1.J == a.I(1))
  println(new h1.J == b.I(1))
  println(Set(a.I(1), b.I(1), a.I(1)).size)
