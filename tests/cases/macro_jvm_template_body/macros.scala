import scala.quoted.*

// The macro runs std definitions that have a `@jvm` template and a Scala body; the interpreter
// runs the body on every target where it has no builtin.
object Macros:
  inline def utf8Length(inline s: String): Int = ${ utf8LengthImpl('s) }
  def utf8LengthImpl(s: Expr[String])(using Quotes): Expr[Int] =
    Expr(s.valueOrAbort.getBytes().length)
