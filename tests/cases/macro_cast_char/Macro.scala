// A macro's `asInstanceOf[Char]` unboxes as the JVM does, where dotty runs a macro's code
// (`quoted.Interpreter`), whatever target the program is built for: a `Char` is itself, `null`
// the zero char.
import scala.quoted.*
object M:
  inline def chars: String = ${impl}
  def impl(using Quotes): Expr[String] =
    val v: Any = 'a'
    val n: Any = null
    val c: Char = v.asInstanceOf[Char]
    val z: Char = n.asInstanceOf[Char]
    Expr(s"$c ${z.toInt} ${(c + 1).toChar}")
