import scala.quoted.*
object M:
  inline def typeOf[T](inline x: T): String = ${ typeImpl[T]('x) }
  def typeImpl[T: Type](x: Expr[T])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(Type.show[T] + " / " + x.asTerm.tpe.widen.show)
  inline def shape(inline x: Any): String = ${ shapeImpl('x) }
  def shapeImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def kind(t: Term): String = t match
      case Inlined(_, _, e) => kind(e)
      case Apply(f, args) => "apply(" + kind(f) + "; " + args.map(kind).mkString(", ") + ")"
      case Select(q, n) => "select(" + kind(q) + "." + n + ")"
      case Literal(c) => "lit(" + c.value + ")"
      case Ident(n) => "ident(" + n + ")"
      case Typed(e, _) => "typed(" + kind(e) + ")"
      case other => "other"
    Expr(kind(x.asTerm))
  inline def where(inline x: Any): String = ${ whereImpl('x) }
  def whereImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val e = Position.ofMacroExpansion
    Expr(e.sourceFile.name + ":" + e.startLine + ":" + e.startColumn)
  inline def symbolOf(inline x: Any): String = ${ symbolImpl('x) }
  def symbolImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def sym(t: Term): Symbol = t match
      case Inlined(_, _, e) => sym(e)
      case _ => t.symbol
    val s = sym(x.asTerm)
    Expr(s.name + " in " + s.owner.name)
