// A concrete var's setter is a member, as dotty's `Desugar.isSetterNeeded` makes it: selected by
// its name (an alternative of a written method of that name, which takes `Short` and loses to it
// for an `Int`), through a trait, a constructor parameter, a qualified private var, a package
// object's var and a generic class, as a function value, and called unqualified in its object. A
// concrete var still implements an abstract one of a trait, its own or inherited; an export of a
// var forwards its setter, which an assignment through the export calls.
trait T:
  var t = 1

class C(var c: Int) extends T

object O:
  var n = 0
  def n_=(x: Short): Unit = n = 99
  private[O] var q = 0
  def setq(): Unit = O.q_=(4)
  def getq = q
  def reset(): Unit = n_=(0)

package object po:
  var pv = 0

class Box[A](init: A):
  var value: A = init

class Base { var x = 0 }
trait AbsX { var x: Int }
class Mixed extends Base with AbsX
trait AbsY { var y: Int }
trait ConcY { var y = 0 }
class Both extends ConcY with AbsY

object V { var w = 0 }
object EV { export V.* }

@main def run(): Unit =
  O.n_=(7)
  println(O.n)
  O.n_=(3.toShort)
  println(O.n)
  O.reset()
  println(O.n)
  val x = new C(2)
  val tt: T = x
  tt.t_=(10)
  x.c_=(20)
  O.setq()
  po.pv_=(6)
  val f: Int => Unit = x.c_=
  f(30)
  val b = new Box(List(1))
  b.value_=(List(2, 3))
  println(s"${x.t} ${x.c} ${O.getq} ${po.pv} ${b.value}")
  val m: AbsX = new Mixed
  m.x = 1
  m.x_=(m.x + 1)
  val both: AbsY = new Both
  both.y = 3
  println(s"${m.x} ${both.y}")
  EV.w_=(5)
  val viaSetter = V.w
  EV.w = 6
  println(s"$viaSetter ${V.w} ${EV.w}")
