// A trait nested in a class takes its enclosing instance from the prefix of the class's base type
// (dotty's `ExplicitOuter.outerPrefix`): written as the parent, behind an alias, or inherited
// through another parent, a class's or a trait's.
class O(val n: Int):
  trait I:
    def get = n
  type Alias = I
object H:
  val o = new O(7)
  val p = new O(8)
type Hidden = H.p.I
abstract class A extends H.o.I
class C extends A
trait J extends H.p.I
class D extends J
class E extends Hidden
class F extends H.o.Alias

@main def run(): Unit =
  println((new C).get)
  println((new D).get)
  println((new E).get)
  println((new F).get)
