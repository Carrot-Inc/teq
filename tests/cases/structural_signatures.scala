// A reflective structural call names its method by the name and the erased classes of the
// parameters (dotty's `addClassOfs`, scala-library's `Selectable.applyDynamic` by signature,
// Scala.js's reflective proxies): two alternatives of one arity are told apart, through the
// conversion and explicitly, an inherited or overridden one reached on the receiver's class, a
// parameter of `Any` or `AnyRef` by `Object`'s class.
import scala.reflect.Selectable.reflectiveSelectable

class Point(val x: Int)
class Box:
  def f(x: Int): String = s"int $x"
  def f(x: String): String = s"string $x"
  def f(p: Point): String = s"point ${p.x}"
  def f(x: Int, y: Int): String = s"pair ${x + y}"
  def g(): String = "none"
  def g(x: Long): String = s"long $x"
class Wide:
  def f(x: Any): String = "any"
  def f(x: Int): String = "int"
  def g(xs: List[Int]): String = "list"
  def g(x: Long): String = "long"
  def h(x: AnyRef): String = "anyref"
  def h(x: Long): String = "long"
class Sub extends Box:
  override def f(x: String): String = s"sub ${super.f(x)}"
trait Named:
  def name(prefix: String): String = prefix + "named"
  def name(times: Int): String = "named" * times
class Thing extends Named

@main def run(): Unit =
  val b = reflectiveSelectable(new Box)
  println(b.applyDynamic("f", classOf[Int])(1))
  println(b.applyDynamic("f", classOf[String])("a"))
  println(b.applyDynamic("f", classOf[Point])(new Point(3)))
  println(b.applyDynamic("f", classOf[Int], classOf[Int])(1, 2))
  println(b.applyDynamic("g")())
  println(b.applyDynamic("g", classOf[Long])(5L))
  val s: { def f(x: String): String } = new Sub
  val i: { def f(x: Int): String } = new Sub
  println(s.f("b"))
  println(i.f(2))
  val t: { def name(prefix: String): String } = new Thing
  val n: { def name(times: Int): String } = new Thing
  println(t.name("the "))
  println(n.name(2))
  val w = reflectiveSelectable(new Wide)
  println(w.applyDynamic("f", classOf[Object])("s"))
  println(w.applyDynamic("f", classOf[Int])(1))
  println(w.applyDynamic("g", classOf[List[?]])(List(1)))
  println(w.applyDynamic("h", classOf[Object])("x"))
  val wa: { def f(x: Any): String } = new Wide
  println(wa.f(2))
