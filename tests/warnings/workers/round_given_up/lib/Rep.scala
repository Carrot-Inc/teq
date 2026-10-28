import scala.quoted.*

// Reports `msg` at the definition of `T`, a place of the caller's program whichever call reports it.
object Rep:
  inline def warnAt[T](inline msg: String): Unit = ${ warnAtImpl[T]('msg) }
  def warnAtImpl[T: Type](msg: Expr[String])(using Quotes): Expr[Unit] =
    import quotes.reflect.*
    report.warning(msg.valueOrAbort, TypeRepr.of[T].typeSymbol.pos.get)
    '{ () }
