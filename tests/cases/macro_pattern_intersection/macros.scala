// A quote pattern whose type variable stands in an intersection with the macro's type
// parameter, `$layer: Layer[in, e & E, out]` as zio's layer macro writes it: `e` is the
// scrutinee's type argument.
import scala.quoted.*

final class Layer[-In, +E, +Out](val name: String)

object Macros:
  inline def describe[E](inline l: Layer[?, E, ?]): String = ${ describeImpl[E]('l) }

  def describeImpl[E: Type](l: Expr[Layer[?, E, ?]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    l match
      case '{ $layer: Layer[in, e & E, out] } =>
        Expr(s"${Type.show[in]} ${Type.show[e]} ${Type.show[out]}")
