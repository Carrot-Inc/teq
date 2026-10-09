package siglib
import scala.quoted.*

object Sigs:
  inline def of[T](inline name: String): String = ${ ofImpl[T]('name) }

  def ofImpl[T: Type](name: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val cls = TypeRepr.of[T].typeSymbol
    val shown = (cls.methodMember(name.valueOrAbort) ++ List(cls.fieldMember(name.valueOrAbort)).filterNot(_.isNoSymbol))
      .map(s => s.signature.paramSigs.mkString("(", ", ", ")") + " " + s.signature.resultSig)
    Expr(shown.mkString(" | "))
