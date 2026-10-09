// A deferred given (`given x: T = deferred`, SIP-64) is implemented in the first class below the
// trait, by a search where that class is defined: the context outside it, with the class's own
// using parameters (dotty's `Typer.implementDeferredGivens`), never the class's members nor the
// givens it inherits; a subclass of an implementing class inherits its implementation. Objects,
// anonymous and abstract classes, enums, case classes and given instances implement it too.
// teq left it to `???` on the JVM and could not resolve `deferred` on JavaScript and the
// interpreter.
import scala.compiletime.deferred

trait T:
  given x: Int = deferred
  def twice = summon[Int] * 2

object Definition:
  given Int = 11
  class C extends T
  object O extends T
  abstract class A extends T
  enum E extends T:
    case One, Two
  case class K(s: String) extends T

class Provided:
  given provided: Int = 99
object Shadow:
  given Int = 22
  class Inherits extends Provided with T

class Using(using i: Int) extends T
class Down(using i: Int) extends Using(using 11)
class Sub extends Definition.A

class Outer(n: Int):
  given Int = n
  class Inner extends T

trait Ordered[A]:
  given ord: Ordering[A] = deferred
  def sorted(l: List[A]): List[A] = l.sorted
class Ints extends Ordered[Int]
class Bound[B: Ordering] extends Ordered[B]
class Passed[B](using o: Ordering[B]) extends Ordered[B]

trait Anonymous:
  given Int = deferred
object Anon:
  given Int = 4
  class C extends Anonymous

trait Redeclared extends T:
  override given x: Int = 100
object Implemented:
  given Int = 6
  class C extends T with Redeclared

object Objects:
  given String = "s"
  trait Named:
    given name: String = deferred
  given named: Named with
    def extra = 1

@main def run(): Unit =
  given Int = 22
  println((new Definition.C).x)
  println(s"${Definition.O.x} ${Definition.O.twice}")
  println(s"${Definition.E.One.x} ${Definition.K("k").twice}")
  println((new Shadow.Inherits).x)
  println(s"${Using(using 33).x} ${Down(using 44).x} ${(new Sub).x}")
  val outer = Outer(8)
  println((new outer.Inner).x)
  val anon = new T {}
  println(anon.x)
  def make(using i: Int): T = new T {}
  println(make(using 55).x)
  println(Ints().sorted(List(3, 1, 2)))
  println(Bound[String](using Ordering.String.reverse).sorted(List("a", "c", "b")))
  println(Passed[Int](using Ordering.Int.reverse).sorted(List(3, 1, 2)))
  println((new Anon.C).given_Int)
  println((new Implemented.C).twice)
  println(Objects.named.name)
