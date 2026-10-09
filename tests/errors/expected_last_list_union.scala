// expect: 13:14: error: type mismatch: found String | W, required W
// expect: 14:14: error: type mismatch: found String | W, required W
// expect: 2 errors found
// An earlier argument list settles the type argument from its own arguments, without the
// expected type, which the whole result then fails to meet, as scalac reports it.
import scala.language.implicitConversions
final case class W(s: String)
object W:
  given Conversion[String, W] = W(_)
object Main:
  def two[B](ifEmpty: B)(f: Double => B): B = f(1.0)
  def two2[B](ifEmpty: B)(f: B): B = f
  val c: W = two("-")(x => W("a"))
  val d: W = two2("-")(W("a"))
