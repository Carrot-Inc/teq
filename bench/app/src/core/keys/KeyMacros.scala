package meridian.core.keys

import scala.quoted.*

object KeyMacros:
  def longKey[K: Type](using Quotes): Expr[LongKey[K]] =
    import quotes.reflect.*
    val tpe = TypeRepr.of[K]
    val symbol = tpe.typeSymbol
    val name = Expr(symbol.fullName)
    if symbol.flags.is(Flags.Case) then
      val field = symbol.caseFields match
        case single :: Nil => single
        case _ => report.errorAndAbort(s"${symbol.name} needs exactly one field to be a LongKey")
      val companion = Ref(symbol.companionModule)
      '{ new LongKey[K] {
           def keyName = $name
           def wrap(raw: Long): K = ${ Apply(Select.unique(companion, "apply"), List('{ raw }.asTerm)).asExprOf[K] }
           extension (k: K) def raw: Long = ${ '{ k }.asTerm.select(field).asExprOf[Long] }
         } }
    else
      '{ new LongKey[K] {
           def keyName = $name
           def wrap(raw: Long): K = raw.asInstanceOf[K]
           extension (k: K) def raw: Long = k.asInstanceOf[Long]
         } }

  def textKey[K: Type](using Quotes): Expr[TextKey[K]] =
    import quotes.reflect.*
    val tpe = TypeRepr.of[K]
    val symbol = tpe.typeSymbol
    val name = Expr(symbol.fullName)
    if symbol.flags.is(Flags.Case) then
      val field = symbol.caseFields match
        case single :: Nil => single
        case _ => report.errorAndAbort(s"${symbol.name} needs exactly one field to be a TextKey")
      val companion = Ref(symbol.companionModule)
      '{ new TextKey[K] {
           def keyName = $name
           def wrap(raw: String): K = ${ Apply(Select.unique(companion, "apply"), List('{ raw }.asTerm)).asExprOf[K] }
           extension (k: K) def raw: String = ${ '{ k }.asTerm.select(field).asExprOf[String] }
         } }
    else
      '{ new TextKey[K] {
           def keyName = $name
           def wrap(raw: String): K = raw.asInstanceOf[K]
           extension (k: K) def raw: String = k.asInstanceOf[String]
         } }
