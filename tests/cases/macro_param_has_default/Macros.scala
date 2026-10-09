package mlib
import scala.quoted.*

object Defaults:
  inline def withDefault[T]: List[String] = ${ impl[T] }
  def impl[T: Type](using q: Quotes): Expr[List[String]] =
    import q.reflect.*
    val ctor = TypeRepr.of[T].typeSymbol.primaryConstructor
    Expr(ctor.paramSymss.flatten.filter(_.flags.is(Flags.HasDefault)).map(_.name))
