// An inline method of the class expanded in its constructor code (dotty's `Inlined`, which `Constructors.intoConstr`
// descends into): an identifier of the body naming the class's own parameter accessor reads the constructor's parameter
// where the call stands on the constructor's own `this` (`saved`, `v`, `v2`, `v3` and the local inline def's `loc` are
// what A got), and so does an argument naming it (`twice(s)`); `this.s` in the body (`vt`) and a member read stay the
// accessor's, virtual. A transparent expansion of the parameter keeps the accessor's path (`in.type`).
class A(val s: Int):
  inline def im = s
  inline def im2(k: Int) = s + k
  inline def imt = this.s
  val saved = im
  val v = im
  val v2 = im2(10)
  val v3 = this.im
  val vt = imt
  val loc = { inline def l = s; l }
class B(override val s: Int) extends A(s + 1)

inline def twice(x: Int): Int = x + x
class C(val s: Int):
  val t = twice(s)
class D(override val s: Int) extends C(s + 1)

class Inner:
  val k = 4
transparent inline def tsame(x: Inner) = x
class E(val in: Inner):
  val c: in.type = tsame(in)

@main def run(): Unit =
  val b = B(2)
  println(s"${b.s} ${b.saved} ${b.v} ${b.v2} ${b.v3} ${b.vt} ${b.loc}")
  println(D(2).t)
  val e = E(Inner())
  println(s"${e.c eq e.in} ${e.c.k}")
