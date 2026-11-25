// A default of a generic class's constructor left out of a parent's call is its getter applied to the call's type
// arguments (`C.<init>$default$1[Int]`): a written parent, a generic one at its own type parameter, a local class's, a
// trait-first class's; a call of the same class at other type arguments inside the parent's arguments, a creation and
// an anonymous class; a secondary constructor's default, a second parameter list's, a trait parameter's, an enum
// case's and an object's parent call, a class of two type parameters, and a class whose type arguments the call infers
// from its value arguments; and a secondary constructor's default in a `new`, its type arguments written and inferred.
class C[X](val x: Option[X] = None, val y: Int = 0) { def n: Int = x.size + y + 31 }
class W extends C[Int]
class V[Y] extends C[Y]
trait T extends C[Int]
class K extends T
class D[X](val y: Int = 0, val x: Option[X] = None) { def n: Int = x.size + y + 31 }
class N1 extends D[Int](new D[String]().x.size + 1)
class N2 extends D[Int]((new D[String] {}).x.size + 2)
class S[X](val x: Option[X], val k: Int) { def this(o: Option[X] = None) = this(o, 7); def n: Int = x.size + k }
class SK extends S[Int]()
class P[X](val a: Int)(val x: Option[X] = None) { def n: Int = a + x.size }
class PK extends P[Int](3)()
trait TP[X](val x: Option[X] = None) { def n: Int = x.size + 31 }
class TK extends TP[Int]
enum E[X](val x: Option[X] = None) { case A extends E[Int]; case B extends E[String]() }
object OB extends C[Int]
class C2[X, Y](val x: Option[X] = None, val y: Option[Y] = None) { def n: Int = x.size + y.size + 31 }
class K2 extends C2[Int, String]
class G[X](val a: X, val b: Option[X] = None) { def n: Int = b.size + 31 }
class GK extends G(5)
class SI extends S()
object News:
  def written: Int = new S[Int]().n + new C[Int]().n
  def inferred: Int = new G("g").n + { val s: S[Int] = new S(); s.n }

object Main:
  def local: Int =
    class L extends C[Int]
    new L().n

  def main(args: Array[String]): Unit =
    println(List(new W().n, new V[String]().n, new K().n, local))
    println(List(new N1().n, new N2().n, new SK().n, new PK().n, new TK().n, E.A.x.size, E.B.x.size, OB.n, new K2().n))
    class Sub extends W
    println(new Sub().n)
    println(List(new GK().n, new SI().n, News.written, News.inferred))
    println((new TK {}).n)
