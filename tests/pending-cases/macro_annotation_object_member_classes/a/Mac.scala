package mil

import scala.annotation.{experimental, MacroAnnotation}
import scala.quoted.*

object M:
  inline def k: Int = 3
  inline def m(inline x: Int): Int = ${ twice('x) }
  inline def d(inline x: Int = k + 1): Int = ${ twice('x) }
  def twice(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x * 2 }

@experimental
class plus extends MacroAnnotation:
  def transform(using Quotes)(definition: quotes.reflect.Definition, companion: Option[quotes.reflect.Definition]): List[quotes.reflect.Definition] =
    import quotes.reflect.*
    definition match
      case DefDef(name, params, tpt, Some(rhs)) =>
        val body = '{ ${ rhs.asExprOf[Int] } + 14 }
        List(DefDef.copy(definition)(name, params, tpt, Some(body.asTerm.changeOwner(definition.symbol))))
      case _ => List(definition)
