import scala.quoted.*

object Macros:
  inline def result[A](inline value: A): String = ${ resultImpl('value) }

  def resultImpl[A: Type](value: Expr[A])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def expansion(tree: Term): Term = tree match
      case Inlined(_, Nil, body) => expansion(body)
      case _ => tree
    val copied = (new TreeMap {}).transformTerm(expansion(value.asTerm))(Symbol.spliceOwner)
    val isValue = copied.isExpr
    val hasResultType = copied.tpe.widen =:= TypeRepr.of[A]
    val expression = copied.asExprOf[A]
    '{ ${Expr(s"$isValue $hasResultType") } + " " + $expression.toString }
