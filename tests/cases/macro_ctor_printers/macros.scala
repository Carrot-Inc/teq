import scala.quoted.*

object Re:
  private def strip(using Quotes)(t: quotes.reflect.Term): quotes.reflect.Term =
    import quotes.reflect.*
    t match
      case Inlined(_, Nil, b) => strip(b)
      case _ => t

  // A constructor call as the three printers show it: the typed tree, a TreeMap's copy, a copy by the
  // trees' copy methods, one over a fresh New of the type, one whose TypeApply takes other arguments, and
  // one rebuilt by the constructors over the New's own type tree.
  inline def pr(inline e: Any): String = ${ prImpl('e) }
  def prImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = strip(e.asTerm)
    val owner = Symbol.spliceOwner
    val sb = new StringBuilder
    def line(label: String, x: Tree): Unit =
      sb.append(s"\n  $label: ${x.show} | ${x.show(using Printer.TreeShortCode)} | ${x.show(using Printer.TreeStructure)}")
    def copy(t: Term): Term = t match
      case a @ Apply(f, as) => Apply.copy(a)(copy(f), as)
      case ta @ TypeApply(f, ts) => TypeApply.copy(ta)(copy(f), ts)
      case s @ Select(q @ New(tpt), n) => Select.copy(s)(New.copy(q)(tpt), n)
      case other => other
    def fresh(t: Term): Term = t match
      case a @ Apply(f, as) => Apply.copy(a)(fresh(f), as)
      case ta @ TypeApply(f, ts) => TypeApply.copy(ta)(fresh(f), ts)
      case s @ Select(New(tpt), n) => Select.copy(s)(New(Inferred(tpt.tpe)), n)
      case other => other
    def anyArgs(t: Term): Term = t match
      case a @ Apply(f, as) => Apply.copy(a)(anyArgs(f), as)
      case TypeApply(f, ts) => TypeApply(f, ts.map(_ => TypeTree.of[Any]))
      case other => other
    def rebuilt(t: Term): Term = t match
      case Apply(f, as) => Apply(rebuilt(f), as)
      case TypeApply(f, ts) => TypeApply(rebuilt(f), ts)
      case s @ Select(New(tpt), _) => Select(New(tpt), s.symbol)
      case other => other
    line("source", t)
    line("treemap", (new TreeMap {}).transformTerm(t)(owner))
    line("copy", copy(t))
    line("freshNew", fresh(t))
    line("anyArgs", anyArgs(t))
    line("rebuilt", rebuilt(t))
    Expr(sb.toString)

  // The New's type tree: one tree however often it is asked for, an alias's type, its type's arguments.
  inline def tpt(inline e: Any): String = ${ tptImpl('e) }
  def tptImpl(e: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def theNew(t: Term): Option[New] = t match
      case n: New => Some(n)
      case Apply(f, _) => theNew(f)
      case TypeApply(f, _) => theNew(f)
      case Select(q, _) => theNew(q)
      case Inlined(_, _, b) => theNew(b)
      case Block(_, b) => theNew(b)
      case Typed(b, _) => theNew(b)
      case _ => None
    val n = theNew(e.asTerm).get
    val alias = n.tpt.tpe.typeSymbol.isAliasType
    val shown = if alias then s" tpe=${n.tpt.tpe.show}" else ""
    val args = n.tpt.tpe.typeArgs.map(_.typeSymbol.name).mkString(",")
    Expr(s"same=${n.tpt == n.tpt}$shown args=$args")
