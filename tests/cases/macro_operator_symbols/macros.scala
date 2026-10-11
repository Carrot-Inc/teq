import scala.quoted.*

// A primitive operator's symbol as the reflection API gives it: the class's method, one symbol
// whatever selection it was read from (`x + y` and `y + x` select `Int.+(Int)`).
object Ops:
  inline def same(inline a: Any, inline b: Any): String = ${ sameImpl('a, 'b) }
  def sameImpl(a: Expr[Any], b: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def op(t: Term): Symbol = t.underlyingArgument match
      case Apply(sel: Select, _) => sel.symbol
      case other => report.errorAndAbort(s"no operator: ${other.show}")
    val (x, y) = (op(a.asTerm), op(b.asTerm))
    Expr(s"${x.name} ${x.owner.name} ${x == y}")
