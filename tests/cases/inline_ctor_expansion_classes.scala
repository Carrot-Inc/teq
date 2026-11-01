// An inline call in constructor code whose expansion makes a class (dotty's `Constructors.intoConstr` stops at a class
// definition): a receiver placed in the class reads the accessor (`in.wrap`), and a class the expansion makes reads its
// own accessor through its own `this` (`mk`). A `this` an ascription or a cast widens is bound to a proxy
// (`Inliner.canElideThis`), whose selection is the accessor's.
abstract class Fn:
  def apply(): Int

class Inner(val k: Int):
  inline def wrap: Fn = new Fn { def apply() = k }
class E(val in: Inner):
  val w = in.wrap
class F(override val in: Inner) extends E(Inner(1))

class G(val s: Int):
  inline def im = s
  transparent inline def mk: Any = new G(s + 1000) { def q = this.im; override def toString = q.toString }
  val o: Any = if s >= 1000 then "none" else mk
  val r1 = (this: G).im
  val r2 = this.asInstanceOf[G].im
  val r3 = (this: this.type).im
class H(override val s: Int) extends G(s + 1)

@main def run(): Unit =
  println(s"${F(Inner(2)).w()} ${E(Inner(5)).w()}")
  println(G(1).o)
  val h = H(2)
  println(s"${h.r1} ${h.r2} ${h.r3}")
