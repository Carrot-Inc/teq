// A macro's `Symbol.tree` of a member under -Yretain-trees (teq's --retain-trees) is its definition
// as typed (`Symbols.retainsDefTree`), each right-hand side the plain inline call the typer kept,
// unexpanded: the inlining phase expands the first, whose macro reads the three, after the whole
// program is typed, where scalac shows the calls `PostTyper` keeps (`sym.defTree`).
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
