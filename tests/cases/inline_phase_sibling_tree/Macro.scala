// A macro's `Symbol.tree` of a member of a class has no right-hand side where trees are not retained
// (dotty's `FromSymbol.valDefFromSym`, `Symbols.retainsDefTree`): the siblings are read as their
// declarations, none of them typed or expanded for the read, as the inlining phase reaches each call
// after the program is typed (inline_phase_sibling_tree_retained reads them under -Yretain-trees).
import scala.quoted.*
object M:
  inline def inspect: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    var cls = Symbol.spliceOwner
    while !cls.isClassDef do cls = cls.owner
    val fields = cls.declarations.filter(_.name.startsWith("d"))
    Expr(fields.map(s => s.tree match
      case ValDef(_, tpt, Some(rhs)) => s.name + ":" + (tpt.tpe =:= TypeRepr.of[String]).toString + ":" + rhs.show
      case DefDef(_, _, tpt, Some(rhs)) => s.name + ":" + (tpt.tpe =:= TypeRepr.of[String]).toString + ":" + rhs.show
      case _ => s.name + ":other:" + s.tree.show).mkString("\n"))
