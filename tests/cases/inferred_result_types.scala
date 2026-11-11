// A member without a written result type takes the conjunction of the result types of the members it overrides,
// each its base class's own declaration, instantiated with the member's own type and value parameters and seen from
// the member's class (dotty's Namer.inferredResultType, `inherited`); one that overrides nothing and a local
// definition take their right-hand side's. A final or inline val takes its right-hand side's type, typed against
// the inherited one: a constant stays one, any other type is widened within it. `kind` shows the type by the
// overload it selects.
object Show:
  def kind(x: Any): String = "Any"
  def kind(x: Int): String = "Int"
  def kind(x: Long): String = "Long"
  def kind(x: String): String = "String"
  def kind(x: List[Int]): String = "List"
  def kind(x: Seq[Int]): String = "Seq"
import Show.kind

trait A:
  def x: Any
trait B:
  def x: Int
class C1 extends B with A:
  def x = 1
class C2 extends A with B:
  def x = 1

trait F:
  def id[T](t: T): T
class G extends F:
  def id[U](u: U) = u

trait Box:
  type T
  def value: T
class IntBox extends Box:
  type T = Int
  def value = 3
trait Dep:
  def f(b: Box): b.T
class DepI extends Dep:
  def f(b: Box) = b.value

trait H:
  def m(x: Int): Any
class K extends H:
  def m(x: Int) = 1
  def m(x: Int, y: Int) = 2

class L extends A:
  inline def x = 1
trait FI:
  def f: Int => Int
class GI extends FI:
  inline def f = _ + 1

trait V:
  val c: Int
  val l: Long
  val s: Seq[Int]
  val f: Int => Int
  val xs: List[Int]
class W extends V:
  final val c = 6
  final val l = 7
  final val s = List(1)
  final val f = x => x + 1
  final val xs = List()
class W2 extends V:
  val c = 6
  val l = 7
  val s = List(1)
  val f = x => x + 2
  val xs = List()

trait IV:
  val c: Int
class IW extends IV:
  final inline val c = 6

trait P[T]:
  def get: T
class Q extends P[String]:
  def get = "q"

@main def run(): Unit =
  println(kind((new C1).x) + " " + kind((new C2).x))
  println(kind((new G).id(1)) + " " + kind((new G).id("s")))
  val ib = new IntBox
  val i: Int = (new DepI).f(ib)
  println(i)
  println(kind((new K).m(1)) + " " + kind((new K).m(1, 2)))
  println(kind((new L).x))
  val fi: FI = new GI
  println((new GI).f(1).toString + " " + fi.f(10))
  val w = new W
  val six: 6 = w.c
  val seven: 7L = w.l
  println(six.toString + " " + seven + " " + kind(w.s) + " " + w.f(1) + " " + kind(w.xs))
  val w2 = new W2
  println(kind(w2.c) + " " + kind(w2.l) + " " + kind(w2.s) + " " + w2.f(1) + " " + kind(w2.xs))
  val inlined: 6 = (new IW).c
  println(inlined)
  println(kind((new Q).get))
  def local = 1
  println(kind(local))
