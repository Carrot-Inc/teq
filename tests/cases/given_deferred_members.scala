// The implementation of a deferred given among the class's other members: a by-name using
// parameter is forced once, by the lazy implementation (an ordinary `summon` of it on each read),
// and a deferred given overriding an old-style abstract one implements both, read as a method
// through the abstract one and as a value through the deferred one.
import scala.compiletime.deferred

trait T:
  given x: Int = deferred
class ByName(using n: => Int) extends T
class Summoned(using n: => Int):
  def x: Int = summon[Int]

trait A:
  given a: Int
  def read: Int = a
trait B extends A:
  given a: Int = deferred
class C(using Int) extends B

trait GA[X]:
  given g: X
trait GB[X] extends GA[X]:
  given g: X = deferred
class GC(using String) extends GB[String]

@main def run(): Unit =
  var count = 0
  val b = new ByName(using { count += 1; count })
  println(s"$count ${b.x} ${b.x} $count")
  val s = new Summoned(using { count += 1; count })
  println(s"${s.x} ${s.x} $count")
  val c = new C(using 12)
  val asA: A = c
  println(s"${c.a} ${asA.a} ${asA.read}")
  val gc = new GC(using "g")
  println(s"${gc.g} ${(gc: GA[String]).g}")
