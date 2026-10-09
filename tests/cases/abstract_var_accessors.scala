// An abstract var is its getter and setter pair: implemented by a var or by a def pair, assigned
// and read through the trait or class that declares it, unqualified inside it, and compound
// assignment through a def pair with the receiver evaluated once.
import scala.collection.mutable.ListBuffer

trait AbsVar:
  var x: Int
  def set(v: Int): Unit = x = v
  def bump(): Unit = x += 1
  def twice: Int = x * 2

trait DefPair:
  def y: Int
  def y_=(v: Int): Unit

class Store:
  var cell = 0

class Meters(val v: Int) extends AnyVal:
  override def toString = s"${v}m"

trait Handle[A]:
  var current: A

class Ref[A](init: A):
  var value: A = init

// The Hooks shape: a wrapper over a mutable cell, assigned through the trait.
trait SetRefHandle[A]:
  var current: A

object Hooks:
  def handle[T](ref: Ref[T]): SetRefHandle[T] = new SetRefHandle[T]:
    def current: T = ref.value
    def current_=(v: T): Unit = ref.value = v

abstract class Cell:
  var v: Long
  lazy val label: String = "cell " + v

abstract class Pair:
  def y: Int
  def y_=(v: Int): Unit

class Param(var x: Int) extends AbsVar

trait Concrete extends AbsVar:
  var x = 5

class FromTrait extends Concrete

trait Again extends AbsVar:
  var x: Int

class Twice extends Again:
  var x = 3

trait Lengths:
  var m: Meters

trait Items:
  var items: ListBuffer[Int]

class Doubler:
  private var cell = 0
  def y: Int = cell
  def y_=(v: Int): Unit = cell = v * 2
  def set(): Unit = { y = 5; y += 1 }

// A var inherited from a class that knows nothing of the trait implementing an abstract var or
// a setter pair beside it.
class BaseX:
  var x: Int = 0
class ChildX extends BaseX with AbsVar
abstract class MidX extends BaseX with AbsVar
class LeafX extends MidX
class BaseY:
  var y: Int = 1
class ChildY extends BaseY with DefPair

// A getter overloaded beside its setter, and an abstract var beside a method of its name.
trait Overloaded:
  def w: Int
  def w(n: Int): Int = w + n
  def w_=(v: Int): Unit
class OverloadedCell extends Overloaded:
  var w: Int = 0
trait Beside:
  var z: Int
  def z(n: Int): Int = z + n

// A setter overloaded beside the one a var implements, a generic var whose implementation is
// overloaded, a setter taking a wider type than its getter's, and an override of a separately
// declared setter overload.
trait TwoSetters:
  def v: Int
  def v_=(n: Int): Unit
  def v_=(s: String): Unit = v = s.length
class TwoSettersCell extends TwoSetters:
  var v: Int = 0
class GenCell extends Handle[Int]:
  var current: Int = 0
  def current(n: Int): Int = current + n
class Wide:
  var seen = 0L
  def u: Int = 1
  def u_=(n: Long): Unit = seen = n
trait Spelled:
  var s: Int
  def s_=(text: String): Unit
class SpelledCell extends Spelled:
  var s: Int = 0
  override def s_=(text: String): Unit = s = text.length

// A def pair inherited from a class that knows nothing of the trait, and accessors inherited so
// under another erasure than the generic trait's, which the JVM bridges in the class mixing it in.
trait VarT:
  var x: Int
class DefBase:
  private var n = 1
  def x: Int = n
  def x_=(v: Int): Unit = n = v
class DefChild extends DefBase with VarT
trait GenVar[A]:
  var gx: A
class IntBase:
  var gx = 0
class IntChild extends IntBase with GenVar[Int]
trait GenGet[A]:
  def g: A
class ValBase:
  val g: Int = 2
class ValChild extends ValBase with GenGet[Int]

class Ops[T](h: Handle[T]):
  def set(v: T): Unit = h.current = v
  def get: T = h.current

class HookOps[T](h: SetRefHandle[T]):
  def set(v: T): Unit = h.current = v
  def get: T = h.current

object Main:
  def main(args: Array[String]): Unit =
    val st = Store()
    val a: AbsVar = new AbsVar { var x = 1 }
    a.x = 2; a.x += 1; println("1: " + a.x)
    val b: DefPair = new DefPair { def y = st.cell; def y_=(v: Int) = st.cell = v }
    b.y = 5; b.y += 1; println("2: " + b.y + " " + st.cell)
    val c: DefPair = new DefPair { var y = 7 }
    c.y = 8; c.y *= 2; println("3: " + c.y)
    val d: AbsVar = new AbsVar { def x = st.cell; def x_=(v: Int) = st.cell = v }
    d.x = 11; println("4: " + d.x + " " + st.cell)
    d.set(12); d.bump(); println("5: " + d.x + " " + st.cell + " " + d.twice)
    a.set(20); a.bump(); println("6: " + a.x + " " + a.twice)

    var picked = 0
    def pick(): DefPair = { picked += 1; b }
    pick().y += 10; println("7: " + b.y + " " + picked)
    def pickVar(): AbsVar = { picked += 1; a }
    pickVar().x -= 1; println("8: " + a.x + " " + picked)

    val box = Ref((0, 0))
    val h = Hooks.handle(box)
    h.current = (3, 4)
    println("9: " + box.value + " " + h.current)
    val (p, q) = h.current
    println("10: " + (p + q))
    HookOps(h).set((5, 6)); println("11: " + box.value + " " + HookOps(h).get)

    val hi: Handle[Int] = new Handle[Int] { var current: Int = 1 }
    hi.current = 9; hi.current += 1; println("12: " + hi.current)
    val counter = Ref(0)
    val hd: Handle[Int] = new Handle[Int] { def current = counter.value; def current_=(v: Int) = counter.value = v * 10 }
    hd.current = 4; println("13: " + hd.current + " " + counter.value)
    Ops(hi).set(30); println("14: " + Ops(hi).get)

    val cell: Cell = new Cell { var v = 7L }
    cell.v = 40L; cell.v += 2L; println("15: " + cell.v + " " + cell.label)
    val pair: Pair = new Pair { var y = 1 }
    pair.y = 2; pair.y += 3; println("16: " + pair.y)

    val pr = Param(1)
    val pa: AbsVar = pr
    pa.x = 4; pr.x += 1; println("17: " + pa.x + " " + pr.x)
    val ft: AbsVar = FromTrait()
    ft.x += 2; println("18: " + ft.x)
    val tw = Twice()
    (tw: AbsVar).x = 9; println("19: " + tw.x + " " + (tw: Again).x)

    val len: Lengths = new Lengths { var m = Meters(2) }
    len.m = Meters(5); println("20: " + len.m)
    val it: Items = new Items { var items = ListBuffer(1) }
    it.items += 2; println("21: " + it.items.toList)
    it.items = ListBuffer(7); println("22: " + it.items.toList)
    val db = Doubler()
    db.set(); println("23: " + db.y)

    val cx: AbsVar = new ChildX
    cx.x = 7; cx.x += 1; cx.bump(); println("24: " + cx.x + " " + cx.twice)
    val lx: AbsVar = new LeafX
    lx.set(3); println("25: " + lx.x)
    val cy: DefPair = new ChildY
    cy.y = 4; cy.y += 1; println("26: " + cy.y)
    val ov: Overloaded = new OverloadedCell
    ov.w = 7; ov.w += 1; println("27: " + ov.w + " " + ov.w(1))
    val bz: Beside = new Beside { var z = 1 }
    bz.z = 4; bz.z += 1; println("28: " + bz.z + " " + bz.z(2))
    val ts: TwoSetters = new TwoSettersCell
    ts.v = 7; ts.v += 1; println("29: " + ts.v); ts.v = "abc"; println("30: " + ts.v)
    val gc: Handle[Int] = new GenCell
    gc.current = 8; println("31: " + gc.current + " " + GenCell().current(1))
    val wide = Wide()
    wide.u += 2147483648L; println("32: " + wide.seen)
    val sp: Spelled = new SpelledCell
    sp.s_=("abcd"); println("33: " + sp.s); sp.s = 2; println("34: " + sp.s)
    val vt: VarT = new DefChild
    vt.x = 7; vt.x += 1; println("35: " + vt.x)
    val ic = new IntChild
    val gv: GenVar[Int] = ic
    gv.gx = 7; gv.gx += 1; println("36: " + ic.gx + " " + gv.gx)
    val gg: GenGet[Int] = new ValChild
    println("37: " + gg.g)
