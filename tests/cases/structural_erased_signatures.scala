// A reflective call names its method by the classes of its parameters' erasures, as dotty's
// `Dynamic.addClassOfs` passes them and scala-library's `Selectable.applyDynamic` looks it up
// (`getMethod(name, paramTypes*)`), and as Scala.js names a reflective proxy: an array by its
// element and dimensions, a value class by its own class at the call and by its underlying type in
// the method, which takes and gives the underlying value; a method of another signature is no
// match, and a missing one is a `NoSuchMethodException` naming the receiver's class and the classes.
import scala.reflect.Selectable.reflectiveSelectable

class Meter(val v: Int) extends AnyVal:
  override def toString = s"Meter($v)"
class Show extends Selectable:
  def applyDynamic(name: String, tags: Class[?]*)(args: Any*): Any = { println(s"$name ${tags.map(_.getName).mkString(",")} ${args.mkString(",")}"); 7 }
class Box:
  def f(m: Meter): Meter = Meter(m.v + 1)
  def m: Meter = Meter(5)
  def g(a: Array[Int]): Int = a.length
  def g(a: Array[String]): Int = -a.length
  def h(a: Array[Array[Int]]): Int = a.length
  def k(a: Any, b: AnyRef, c: Matchable, d: Object): String = s"k $a $b $c $d"
  def n(x: Int): String = s"n int $x"
  def n(x: Long): String = s"n long $x"
def attempt(label: String)(body: => Any): Unit =
  try println(s"$label ${body}") catch case e: NoSuchMethodException => println(s"$label no such method ${e.getMessage}")

@main def run(): Unit =
  val s = (new Show).asInstanceOf[Show { def f(m: Meter): Meter }]
  attempt("show")(s.f(Meter(1)))
  val b: { def f(m: Meter): Meter } = new Box
  attempt("value class param")(b.f(Meter(2)))
  val bm: { def m: Meter } = new Box
  attempt("value class result")(bm.m)
  val r = reflectiveSelectable(new Box)
  attempt("array int")(r.applyDynamic("g", classOf[Array[Int]])(Array(1, 2)))
  attempt("array string")(r.applyDynamic("g", classOf[Array[String]])(Array("x", "y")))
  attempt("array array")(r.applyDynamic("h", classOf[Array[Array[Int]]])(Array(Array(1))))
  attempt("array long")(r.applyDynamic("g", classOf[Array[Long]])(Array(1L)))
  attempt("wrong signature")(r.applyDynamic("f", classOf[String])("x"))
  attempt("int for meter")(r.applyDynamic("f", classOf[Int])(3))
  val k: { def k(a: Any, b: AnyRef, c: Matchable, d: Object): String } = new Box
  attempt("objects")(k.k(1, "b", 'c', List(2)))
  attempt("long")(r.applyDynamic("n", classOf[Long])(4L))
  attempt("short")(r.applyDynamic("n", classOf[Short])(4.toShort))
  attempt("no parameters")(r.applyDynamic("n")())
