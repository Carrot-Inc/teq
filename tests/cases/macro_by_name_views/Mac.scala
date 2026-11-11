import scala.quoted.*
object Mac:
  inline def probe(inline x: Any): String = ${ probeImpl('x) }
  def probeImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def strip(t: Term): Term = t match
      case Inlined(_, Nil, e) => strip(e)
      case Block(Nil, e) => strip(e)
      case _ => t
    def desc(t: Term): String = t match
      case NamedArg(n, inner) => s"NamedArg($n, ${desc(inner)})"
      case Apply(f, as) => s"Apply(${desc(f)}, ${as.map(desc).mkString(", ")})"
      case Select(q, n) => s"Select(${desc(q)}, $n)"
      case New(_) => "New"
      case Ident(n) => n
      case Literal(c) => c.value.toString
      case Block(_, _) => "Block"
      case other => other.getClass.getSimpleName
    val t = strip(x.asTerm)
    val same = t match
      case Apply(_, a1) => t match
        case Apply(_, a2) => (a1.last == a2.last).toString
        case _ => "-"
      case _ => "-"
    Expr(desc(t) + " | " + same)
