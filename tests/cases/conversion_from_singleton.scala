// An implicit conversion from a singleton type (http4s' syntax for `Status.Ok.type`, applied as
// `Ok("body")`): a stable receiver of that singleton converts, though its widened type does not
// show the singleton.
import scala.language.implicitConversions
object St:
  val Ok: St = St(200)
case class St(code: Int)
class OkOps(s: St):
  def apply(body: String): String = s"${s.code} $body"
object Conv:
  implicit def okSyntax(status: St.Ok.type): OkOps = new OkOps(status)
object Main:
  import Conv.*
  def main(args: Array[String]): Unit =
    val o: St.Ok.type = St.Ok
    println(o("fine"))
    println(St.Ok("fine"))
