package a2
class C[X](val x: Option[X] = None, val y: Int = 0) { def n: Int = x.size + y + 31 }
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
object News { def written: Int = new S[Int]().n + new C[Int]().n; def inferred: Int = new G("g").n + { val s: S[Int] = new S(); s.n } }
