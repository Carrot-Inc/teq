// A case class's selectors `_1` .. `_N` are members (dotty's `Desugar.productElemMeths`), before the class is
// checked: one implements a parent's abstract `_1` (master: "class C needs to be abstract") and answers a call
// through the parent, `c._1` types (master: "value _1 is not a member"), the parameter of its position or a body val
// or def of no parameter lists of the name keeps its own, a body def of parameters of the name is another
// alternative beside the selector, a repeated parameter's is its sequence, an enum case and a value class have them.
// The selector overrides a parent's concrete `_1` unmarked (dotty's `RefChecks.autoOverride`; master: the parent's).
trait P { def _1: Int }
case class C(x: Int) extends P
case class D(a: String, b: List[Int])
case class R(xs: Int*)
case class N(_1: Int, y: String)
case class B(x: Int) { def _2: String = "own" }
case class V(u: Int) extends AnyVal
enum E { case K(k: Int, l: String) }
trait Q[A] { def _2: A }
case class G[A](n: Int, a: A) extends Q[A]
trait Z { def _1: Int = 0 }
case class Y(y: Int) extends Z
trait H[A] { def _1: A }
case class O(value: Int) extends H[Int]:
  def _1(extra: Int): Int = value + extra
@main def run() =
  val c = C(1)
  println(c._1)
  val p: P = c
  println(p._1)
  println(c.productIterator.mkString)
  println(c.productElementNames.mkString)
  val d = D("a", List(1))
  println(d._1 + d._2)
  println(R(1, 2)._1)
  println(N(3, "y")._1 + N(3, "y")._2)
  println(B(4)._1 + B(4)._2)
  println(V(5)._1)
  println(E.K(6, "l")._1 + E.K(6, "l")._2)
  val q: Q[String] = G(7, "g")
  println(q._2)
  val o = O(3)
  println(o._1)
  println(o._1(4))
  val h: H[Int] = o
  println(h._1)
  val z: Z = Y(5)
  println(z._1)
