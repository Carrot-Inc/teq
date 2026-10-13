// A deferred given's implementation beside a method of its name with other parameters: the
// class's entry holds the method and the inherited declaration as alternatives (dotty's
// `membersNamed`), `c.x` reading the implementation `Typer.implementDeferredGivens` gave the class,
// through the class, the trait and a subclass; and a given value beside a method in one class.
import scala.compiletime.deferred
trait T { given x: Int = deferred; val v: String = "v" }
class C(using i: Int) extends T:
  def x(s: String): String = s + i
  def v(n: Int): String = v * n
class D extends C(using 7)
class E(using Int) extends T:
  given y: Int = 9
  def y(s: String): String = s + y
@main def run(): Unit =
  given Int = 4
  val c = new C
  println(c.x)
  println(c.x("s"))
  println((c: T).x)
  println(c.v(2) + c.v)
  val d = new D
  println(d.x + " " + d.x("d") + " " + (d: T).x)
  val e = new E
  println(e.y + e.y("e") + (e: T).x)
