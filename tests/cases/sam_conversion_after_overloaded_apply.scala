// A `Conversion` written as a lambda takes its parameter type from `Conversion`'s single abstract method, the
// `apply` that overrides `Function1`'s (dotty's `abstractTermMembers`, member denotations merged by signature).
// A trait that extends a function type and overloads `apply` (an `apply()` beside the
// function's) made `Function1`'s `apply` an alternative of its name; the SAM of every later `Conversion` was then
// two methods and its lambdas missed their parameter types. The trait and an anonymous instance of it come first.
import scala.concurrent.Future
import scala.language.implicitConversions

trait Acc[A](val value: A)
trait Header(val path: String)
trait Handler extends (Header => Acc[String]):
  def apply(): Handler = this

def handler(prefix: String): Handler = new:
  def apply(h: Header): Acc[String] = new Acc(prefix + h.path) {}

trait Writer[V]:
  def write(v: V): String
given Writer[Int] = _.toString

final case class Expr[V](field: String, value: V)
def doc(elements: (String, String)*): String = elements.map((k, v) => s"$k=$v").mkString("{", ",", "}")
def doc(elements: Iterable[(String, String)]): String = elements.mkString("[", ",", "]")

def ready[A](a: A): Future[A] = Future.successful(a)

object Givens:
  final class Name(val value: String)
  given Conversion[Name, String] = _.value
  given named: Conversion[String, Name] = s => Name(s.toUpperCase)
  given toDoc[V](using w: Writer[V]): Conversion[Expr[V], String] = e => doc(e.field -> w.write(e.value))
  given options[V](using w: Writer[V]): Conversion[Option[V], Option[String]] = _.map(w.write)
  given Conversion[Int, Future[Int]] = ready
  given Conversion[String, Future[String]] = ready(_)

@main def main(): Unit =
  import Givens.given
  println(handler("/root")(new Header("/x") {}).value)
  println(handler("/root")() eq handler("/root")())
  val n: Givens.Name = "named"
  val s: String = n
  println(s)
  val d: String = Expr("n", 3)
  println(d)
  val four: Option[Int] = Some(4)
  val o: Option[String] = four
  println(o)
  val f: Future[Int] = 5
  println(f.value)
  val g: Future[String] = "six"
  println(g.value)
