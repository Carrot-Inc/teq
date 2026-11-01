// Identifiers of a class's parameter accessor that reach its constructor's code through copies (`Constructors.intoConstr`
// over the code as the backends take it): placed by an inline expansion, as an inline argument or the body of an
// inline method, in a class's body (a partial function literal's cases, `ExpandSAMs.toPartialFunction`'s class; an
// anonymous class's parent call, its body empty or not) they read the accessor (B's 2), in a lambda, a SAM lambda or
// bound to a by-value proxy the parameter (3). A `this` an ascription or a cast widens in a lambda's class for a SAM type
// is bound to a proxy (`Inliner.canElideThis`). A val of an inferred type a partial function's cases complete on demand
// keeps its right side constructor code.
abstract class Fn:
  def apply(): Int
class Box(val v: Int)

class A(val s: Int):
  inline def get = s
  inline def nested = get
  inline def mk: Box = new Box(s) {}
  val pf: PartialFunction[Int, Int] = { case x => x + get }
  val pfGuard: PartialFunction[Int, Int] = { case x if x == get => 100; case x => x }
  val pfMatch: PartialFunction[Int, Int] = x => x match { case y => y + nested }
  val lam = (x: Int) => x + get
  val pfPlain: PartialFunction[Int, Int] = x => x + s
  val box = mk.v
  val boxHere = (new Box(s) {}).v
  val sam: Fn = () => get
  val samAscribed: Fn = () => (this: A).get
  val samCast: Fn = () => this.asInstanceOf[A].get
  val lamAscribed = () => (this: A).get
class B(override val s: Int) extends A(s + 1)

object Lib:
  inline def pfOf(inline x: Int): PartialFunction[Int, Int] = { case _ => x }
  inline def pfOfVal(x: Int): PartialFunction[Int, Int] = { case _ => x }
  inline def boxOf(inline x: Int): Box = new Box(x) {}
  inline def lamOf(inline x: Int): () => Int = () => x
class P(val s: Int):
  val a = Lib.pfOf(s)
  val b = Lib.pfOfVal(s)
  val c = Lib.boxOf(s)
  val d = Lib.lamOf(s)
class Q(override val s: Int) extends P(s + 1)

class F(val s: Int):
  val pf: PartialFunction[Int, Int] = { case _ => later }
  val pfD: PartialFunction[Int, Int] = { case _ => H(3).y }
  val later = s
class G(override val s: Int) extends F(s + 1)
class D(val k: Int):
  val y = k
class H(override val k: Int) extends D(k + 10)

@main def run(): Unit =
  val b = B(2)
  println(s"${b.pf(0)} ${b.pfGuard(2)} ${b.pfMatch(0)} ${b.lam(0)} ${b.pfPlain(0)} ${b.box} ${b.boxHere}")
  println(s"${b.sam()} ${b.samAscribed()} ${b.samCast()} ${b.lamAscribed()}")
  val q = Q(2)
  println(s"${q.a(0)} ${q.b(0)} ${q.c.v} ${q.d()}")
  val g = G(2)
  println(s"${g.later} ${g.pf(0)} ${H(3).y} ${g.pfD(0)}")
  val a = A(5)
  println(s"${a.pf(0)} ${a.pfGuard(5)} ${a.box} ${a.sam()} ${a.samAscribed()}")
