// A reflective call names its method by the erasure of every parameter list, as dotty's
// `TypeErasure` erases a method type (the lists joined, contextual ones run-time parameters):
// `String | Null` erases to `String`, a type parameter to its bound; a structural call passes the
// contextual arguments it resolves or is given, as `Dynamic.handleStructural` passes every list.
import scala.reflect.Selectable.reflectiveSelectable

class Box:
  def f(x: String | Null): String = "string"
  def f(x: Int): String = "int"
  def g(x: Int)(using s: String): String = s + x
  def h[T <: CharSequence](x: T): String = "chars " + x.length
class Show extends Selectable:
  def applyDynamic(name: String, tags: Class[?]*)(args: Any*): Any =
    s"$name ${tags.map(_.getName).mkString(",")} ${args.mkString(",")}"
def attempt(label: String)(body: => Any): Unit =
  try println(s"$label ${body}") catch case _: NoSuchMethodException => println(s"$label missing")

@main def run(): Unit =
  val r = reflectiveSelectable(new Box)
  attempt("union string")(r.applyDynamic("f", classOf[String])("s"))
  attempt("union object")(r.applyDynamic("f", classOf[Object])("s"))
  attempt("context")(r.applyDynamic("g", classOf[Int], classOf[String])(7, "ok"))
  attempt("context first list")(r.applyDynamic("g", classOf[Int])(7))
  attempt("bound")(r.applyDynamic("h", classOf[CharSequence])("abc"))
  attempt("bound erased away")(r.applyDynamic("h", classOf[Object])("abc"))
  given String = "given"
  val b: { def g(x: Int)(using s: String): String } = new Box
  attempt("structural given")(b.g(8))
  attempt("structural explicit")(b.g(9)(using "explicit"))
  val s = (new Show).asInstanceOf[Show { def g(x: Int)(using s: String): String }]
  attempt("custom")(s.g(10))
