// A partial function literal handed to a macro (`inline pf: PartialFunction[A, B]`, Airstream's `handleCase`): seen by
// `quotes.reflect` as the closure over its match dotty's typed tree is, a `Block` that `Lambda(params, Match(_, cases))`
// takes apart into the written cases, where teq showed the application its literal is made of.
import scala.quoted.*
object M:
  inline def shape[A, B](inline pf: PartialFunction[A, B]): String = ${ shapeImpl('pf) }
  def shapeImpl[A: Type, B: Type](pf: Expr[PartialFunction[A, B]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def go(t: Term): String = t match
      case Inlined(_, _, inner) => go(inner)
      case Lambda(params, Match(_, cases)) => s"lambda of ${params.size} with ${cases.size} cases"
      case other => "other"
    Expr(go(pf.asTerm))
