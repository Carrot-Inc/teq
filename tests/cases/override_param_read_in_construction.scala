// An overriding val parameter read while its superclass (or enum) is constructed (dotty's `Constructors.intoConstr`):
// an identifier naming the class's immutable parameter accessor in its constructor's code reads the constructor's own
// parameter (`saved`, `t0` are what the superclass got), a closure's or a local function's there too, a lambda's of a
// SAM type whatever the type (`ExpandSAMs` keeps its body a local function of the constructor), where `this.s`, a
// member's read, a lazy val's and an anonymous class's stay the accessor's, virtual (the subclass's field).
abstract class Fn:
  def apply(): Int
trait TFn:
  def apply(): Int
class A(val s: Int):
  def f = s
  val saved = s
  val viaThis = this.s
  val viaDef = f
  val viaLambda = (() => s)()
  val stored = () => s
  val viaLocalDef = { def g = s; g }
  val viaLocalFun = { def h(k: Int) = s + k; h(10) }
  val viaAnon = new Object { override def toString = s.toString }.toString
  lazy val lz = s
  val viaClassSam: Fn = () => s
  val viaTraitSam: TFn = () => s
  var count = 0
  count = s
class B(override val s: Int) extends A(s + 1)

class P(val s: Int):
  val saved = s
  val viaThis = this.s
class Q(s0: Int) extends P(s0)

class R(x: Int):
  val y = { def g = x; g }
class S extends R(5)

enum H(val s: Int):
  val t0 = s
  val up = (() => s)()
  case K(override val s: Int) extends H(s + 1)
  case L extends H(7)

@main def run(): Unit =
  val b = B(2)
  println(s"${b.s} ${b.saved} ${b.viaThis} ${b.viaDef}")
  println(s"${b.viaLambda} ${b.stored()} ${b.viaLocalDef} ${b.viaLocalFun} ${b.viaAnon} ${b.lz} ${b.count}")
  println(s"${b.viaClassSam()} ${b.viaTraitSam()}")
  val a = A(5)
  println(s"${a.s} ${a.saved} ${a.viaThis} ${a.viaDef} ${a.stored()}")
  val q = Q(7)
  println(s"${q.s} ${q.saved} ${q.viaThis}")
  println(S().y)
  val k = H.K(1)
  println(s"${k.s} ${k.t0} ${k.up} ${H.L.t0}")
