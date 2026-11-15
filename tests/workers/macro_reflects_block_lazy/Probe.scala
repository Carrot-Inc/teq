package hearths

import scala.quoted.*

object Probe:
  inline def probe(inline n: Int): String = ${ probeCode('n) }
  def probeCode(n: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val k = n.valueOrAbort
    val total = Symbol.requiredModule("hearths.Hearth").methodMember("total").head
    var found: List[Symbol] = Nil
    object news extends TreeAccumulator[Unit]:
      def foldTree(u: Unit, tree: Tree)(owner: Symbol): Unit = tree match
        case New(tpt) if tpt.tpe.typeSymbol.maybeOwner.isDefDef && !found.contains(tpt.tpe.typeSymbol) => found = tpt.tpe.typeSymbol :: found
        case _ => foldOverTree(u, tree)(owner)
    news.foldTree((), total.tree)(total)
    def shape(rhs: Option[Term]): String = rhs match
      case Some(Literal(c)) => "=" + c.value
      case Some(Block(_, _)) => "=block"
      case Some(Apply(_, _)) => "=apply"
      case Some(Ident(_)) => "=ident"
      case Some(Select(_, _)) => "=select"
      case Some(_) => "=other"
      case None => "=none"
    def fields(c: Symbol): String = c.declaredFields.map(f => f.tree match
      case v: ValDef => v.name + shape(v.rhs) + (if f.flags.is(Flags.Lazy) then "L" else "")
      case _ => f.name + "?").mkString(" ")
    val all = found.flatMap(c => c :: c.typeRef.baseClasses.filter(b => b.maybeOwner.isDefDef)).distinct
    Expr(all.map(c => c.name + "[" + fields(c) + "]").mkString(" ") + "|" + Hearth.total(k))
