// Inline bodies that scalac 3.8.4 accepts at their definitions, each kept there for its
// expansion: the definition check reports nothing, and every expansion is
// scalac's.
import scala.compiletime.{constValue, summonInline}
import scala.compiletime.ops.int.+
trait B:
  inline def apply(): String
class C extends B:
  inline def apply(): String = "C.apply"
object O extends C
trait TC[A]:
  def name: String
given TC[Int] with
  def name = "Int"
given TC[B] with
  def name = "TC[B]"
given TC[O.type] with
  def name = "TC[O.type]"
def show[A](a: A)(using tc: TC[A]): String = tc.name
// A deferred member dispatched on the argument's class.
inline def call(x: B): String = x()
// Local definitions copied apart at each expansion.
inline def make(x: Int) = { def local() = x; () => local() }
// Searches and constants deferred to the expansion.
inline def nameOf[A]: String = summonInline[TC[A]].name
inline def plusOne[N <: Int]: Int = constValue[N + 1]
// A result type widened at the definition and precise at the expansion.
transparent inline def pick(inline b: Boolean) = inline if b then 1 else "one"
// A dependent result type recomputed at the expansion.
inline def wrap(x: B): List[x.type] = List(x)
transparent inline def first(x: B) = wrap(x).head
// An implicit argument resolved at the definition.
inline def shown(x: B): String = show(x)
// An inline given with an anonymous class, expanded at each use.
inline given tcList[A](using a: TC[A]): TC[List[A]] = new TC[List[A]]:
  def name = "List[" + a.name + "]"
@main def run(): Unit =
  println(call(new C))
  val a = make(1)
  val b = make(2)
  println(a() * 10 + b())
  println(nameOf[Int])
  println(plusOne[41])
  val i: Int = pick(true)
  val s: String = pick(false)
  println(s"$i $s")
  val o: O.type = first(O)
  println(o == O)
  println(shown(O))
  println(summon[TC[List[Int]]].name)
