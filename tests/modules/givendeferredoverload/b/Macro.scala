package gdob

import scala.quoted.*
object M:
  inline def fields[C]: String = ${impl[C]}
  def impl[C: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(TypeRepr.of[C].typeSymbol.declaredFields.map(s => s.name + ":" + s.owner.name).sorted.mkString(","))
