// An assignment operator's application is typed as it is written first (`Applications.typedApply`'s `realApply`
// before `typedOpAssign`): its left side the expression it is, the operator found as any member is (a member, an
// extension of the scope, of the type's companion, of a given, a conversion, an implicit class), on an applied
// left side, a var and a field alike. Only where that fails is the left side an assignment's target.
import scala.language.implicitConversions

class Attr(val name: String):
  def :=(v: String): String = name + "=" + v
trait Named:
  object data
  def data(s: String): Attr = new Attr("data-" + s)
trait Keys:
  object key
trait MoreKeys:
  def key(k: String): Attr = new Attr("key-" + k)
object Html extends Named with Keys with MoreKeys

final case class V(x: Int):
  def rot(a: Int): V = V(x + a)
  def rot(a: Int, o: V): V = V(x + a + o.x)
  def ~==(o: V): Boolean = x == o.x

object time:
  opaque type S = Double
  object S:
    def apply(d: Double): S = d
    extension (s: S) def ~==(o: S): Boolean = math.abs(s - o) < 0.01

class E(val n: Int):
  def +(i: Int): E = { println("plus"); new E(n + i) }
object E:
  extension (e: E) def +=(i: Int): Unit = println("companion extension " + (e.n + i))

class G(val n: Int):
  def +(i: Int): G = { println("plus"); new G(n + i) }
trait Ops:
  extension (g: G) def +=(i: Int): Unit
given Ops with
  extension (g: G) def +=(i: Int): Unit = println("given extension " + (g.n + i))

class W(val n: Int):
  def +(i: Int): W = { println("plus"); new W(n + i) }
class Wide(val w: W):
  def +=(i: Int): Unit = println("conversion " + (w.n + i))
given Conversion[W, Wide] = w => new Wide(w)

class I(val n: Int):
  def +(i: Int): I = { println("plus"); new I(n + i) }
implicit class IOps(val i: I):
  def +=(k: Int): Unit = println("implicit class " + (i.n + k))

class Cells[A](make: Int => A):
  def apply(i: Int): A = { println("apply " + i); make(i) }
  def update(i: Int, a: A): Unit = println("update " + i)

class Holder:
  var e = new E(1)
  var g = new G(2)
  var w = new W(3)
  var i = new I(4)

object Test:
  def main(args: Array[String]): Unit =
    import Html.*
    println(data("id") := "a")
    println(key("pressed") := "false")
    println(V(1).rot(0) ~== V(1))
    println(V(1).rot(0, V(1)) ~== V(2))
    println(time.S(0.033) ~== time.S(0.0333))
    // an applied left side whose element has the operator from elsewhere: no `update`
    val es = new Cells(k => new E(k))
    es(1) += 1
    val gs = new Cells(k => new G(k))
    gs(2) += 1
    val ws = new Cells(k => new W(k))
    ws(3) += 1
    val is = new Cells(k => new I(k))
    is(4) += 1
    // a var and a field of such a type: the operator, no assignment
    var e = new E(10)
    e += 1
    var g = new G(20)
    g += 1
    var w = new W(30)
    w += 1
    var i = new I(40)
    i += 1
    val h = new Holder
    h.e += 5
    h.g += 5
    h.w += 5
    h.i += 5
    println(e.n + g.n + w.n + i.n + h.e.n + h.g.n + h.w.n + h.i.n)
