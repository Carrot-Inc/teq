package meridian.core.enums

import scala.quoted.*
import scala.deriving.Mirror

object EnumMacros:
  def enumerated[E: Type](using Quotes): Expr[Enumerated[E]] =
    import quotes.reflect.*
    val symbol = TypeRepr.of[E].typeSymbol
    if !symbol.flags.is(Flags.Enum) then report.errorAndAbort(s"${symbol.name} is not an enum")
    Expr.summon[Mirror.SumOf[E]] match
      case None => report.errorAndAbort(s"${symbol.name} has no sum mirror")
      case Some(_) =>
        val name = Expr(symbol.fullName)
        val valuesExpr = Select.unique(Ref(symbol.companionModule), "values").asExprOf[Array[E]]
        labelMember(symbol) match
          case Some(label) =>
            '{ new Enumerated[E] {
                 def enumName = $name
                 val values: IArray[E] = IArray.unsafeFromArray($valuesExpr)
                 extension (e: E) def entryName: String = ${ '{ e }.asTerm.select(label).asExprOf[String] }
               } }
          case None =>
            '{ new Enumerated[E] {
                 def enumName = $name
                 val values: IArray[E] = IArray.unsafeFromArray($valuesExpr)
                 extension (e: E) def entryName: String = e.asInstanceOf[Product].productPrefix
               } }

  def labelled[E: Type](using Quotes): Expr[Labelled[E]] =
    import quotes.reflect.*
    val symbol = TypeRepr.of[E].typeSymbol
    if !symbol.flags.is(Flags.Enum) then report.errorAndAbort(s"${symbol.name} is not an enum")
    val name = Expr(symbol.fullName)
    labelMember(symbol) match
      case Some(label) =>
        '{ new Labelled[E] {
             def enumName = $name
             extension (e: E) def entryName: String = ${ '{ e }.asTerm.select(label).asExprOf[String] }
           } }
      case None =>
        '{ new Labelled[E] {
             def enumName = $name
             extension (e: E) def entryName: String = e.asInstanceOf[Product].productPrefix
           } }

  private def labelMember(using Quotes)(symbol: quotes.reflect.Symbol): Option[quotes.reflect.Symbol] =
    import quotes.reflect.*
    val field = symbol.fieldMember("label")
    if field.exists then Some(field)
    else symbol.methodMember("label").headOption
