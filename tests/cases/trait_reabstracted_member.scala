// A trait's abstract redeclaration of a method overrides the bodies of the traits it extends, so the class's
// member is the one dotty's `membersNamed` and `mergeSingleDenot` choose (`MixinOps.isInImplementingClass`):
// `C` mixing `B`, which redeclares `A`'s `toString`, has no forwarder to `A`'s and prints as `Object` does
// (master: `A`), a body in an unrelated trait still wins, and a superclass's method shows through. Per alternative
// of an overloaded name (`nonPrivateMember`'s alternatives): `toString(n: Int)` beside the reabstracted `toString`
// stays `O1`'s, the nullary one `Object`'s.
trait A { override def toString: String = "A" }
trait B extends A { override def toString: String }
class C extends B
trait X { override def toString: String = "X" }
trait Y { override def toString: String }
class D extends X with Y
class E extends Y with X
trait H { override def hashCode: Int = 7 }
trait H2 extends H { override def hashCode: Int }
class F extends H2
trait Q extends B { override def toString: String = "Q" }
class G extends Q
abstract class P { override def toString: String }
class I extends P with A
trait EqA { override def equals(o: Any): Boolean = true }
trait EqB extends EqA { override def equals(o: Any): Boolean }
class J extends EqB
class S { override def toString: String = "S" }
class K extends S with B
class L extends A
class M extends L with B
trait O1:
  override def toString: String = "wrong"
  def toString(n: Int): String = "number"
trait O2 extends O1 { override def toString: String }
class O3 extends O2

def name(x: Any): String = x.toString.takeWhile(_ != '@')

@main def run() =
  println(name(new C))
  val a: A = new C
  println(name(a))
  println(name(s"${new C}"))
  println(name(new D))
  println(name(new E))
  println(new F().hashCode == 7)
  val h: H = new F
  println(h.hashCode == 7)
  println(new G().toString)
  println(name(new I))
  val j = new J
  println(j.equals(new J))
  val e: EqA = j
  println(e.equals(new J))
  println(new K().toString)
  println(new M().toString)
  val o = new O3
  println(name(o))
  println(o.toString(1))
  val o1: O1 = o
  println(name(o1))
