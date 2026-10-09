// After Scala 3's tests/run/enum-Tree.scala-like enums with a class parent: an enum whose
// cases are exceptions, and one that passes its parameters to a class.
import scala.compiletime.{constValue, constValueTuple}
import scala.deriving.Mirror

trait Coded:
  def code: Int
  def tag: String = s"#$code"

class Ranked(val rank: Int):
  def stars: String = "*" * rank
  override def toString: String = s"Ranked($rank)"

trait Label[A]:
  def label(a: A): String

object Label:
  inline def derived[A](using m: Mirror.SumOf[A]): Label[A] =
    val name = constValue[m.MirroredLabel]
    val cases = constValueTuple[m.MirroredElemLabels].toList
    new Label[A]:
      def label(a: A): String = s"$name.${cases(m.ordinal(a))}"

enum Status(val status: Int) extends Ranked(status / 100) with Coded derives Label:
  case Ok extends Status(200)
  case NotFound extends Status(404)
  case Custom(c: Int, name: String) extends Status(c)
  def code: Int = status
  def describe: String = s"$tag $productPrefix $stars"

enum PublishError extends Exception derives Label:
  case Timeout(after: Int)
  case Rejected(reason: String)
  case Unknown
  override def getMessage: String = this match
    case Timeout(a) => s"timed out after $a"
    case Rejected(r) => s"rejected: $r"
    case Unknown => "unknown"

enum Priority extends Ranked(1):
  case Low, High

def fail(e: PublishError): Int = throw e

@main def m(): Unit =
  println(Status.Ok.describe)
  println(Status.NotFound.describe)
  println(Status.Custom(418, "teapot").describe)
  println(Status.Ok.rank + Status.Ok.status + Status.Ok.code)
  println(Status.Custom(1, "x").name)
  println(Status.Ok == Status.Ok)
  println(Status.Custom(1, "x") == Status.Custom(1, "x"))
  println(Status.Custom(1, "x").ordinal)
  println(Status.Ok.ordinal)
  println(summon[Label[Status]].label(Status.NotFound))
  println(summon[Label[Status]].label(Status.Custom(1, "x")))
  val r: Ranked = Status.Ok
  println(r)
  println(Status.Custom(500, "e"))
  val e: Exception = PublishError.Timeout(3)
  println(e.getMessage)
  try fail(PublishError.Rejected("no"))
  catch
    case PublishError.Rejected(r) => println(s"caught $r")
    case t: Throwable => println("other")
  try throw PublishError.Unknown
  catch case t: Exception => println(t.getMessage)
  println(PublishError.Unknown.ordinal)
  println(PublishError.Timeout(1))
  println(summon[Label[PublishError]].label(PublishError.Unknown))
  println(PublishError.Timeout(2).isInstanceOf[Exception])
  println(Priority.High.stars)
  println(Priority.valueOf("Low"))
  println(Priority.values.map(_.rank).sum)
