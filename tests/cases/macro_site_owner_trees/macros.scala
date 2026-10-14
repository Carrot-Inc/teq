// `Symbol.tree` of the owners around a splice: the splice's own `val macro` (dotty's `Splicer`,
// `nme.MACROkw`), a lambda's `$anonfun` as a `DefDef` with its parameters, a generator's too
// (dotty's `FromSymbol.definitionFromSym`), and the method around them.
import scala.quoted.*

inline def inspect: String = ${inspectImpl}
def inspectImpl(using Quotes): Expr[String] =
  import quotes.reflect.*
  val s = Symbol.spliceOwner
  val result = List(s, s.owner, s.owner.owner).map: s =>
    val kind = s.tree match
      case _: DefDef => "DefDef"
      case _: ValDef => "ValDef"
      case _: ClassDef => "ClassDef"
      case _ => "other:" + s.tree.show
    s.name + ":" + s.isDefDef + ":" + kind
  Expr(result.mkString(";"))

inline def params: String = ${paramsImpl}
def paramsImpl(using Quotes): Expr[String] =
  import quotes.reflect.*
  def walk(s: Symbol, acc: List[String]): List[String] =
    if s.isNoSymbol || s == defn.RootClass || s == defn.RootPackage then acc
    else if s.isDefDef then
      val found = s.tree match
        case DefDef(name, clauses, _, _) =>
          name + clauses.flatMap(_.params).map {
            case v: ValDef => v.name + ": " + v.tpt.tpe.show
            case _ => "?"
          }.mkString("(", ", ", ")")
        case _ => "?"
      walk(s.maybeOwner, found :: acc)
    else walk(s.maybeOwner, acc)
  Expr(walk(Symbol.spliceOwner, Nil).reverse.mkString(" < "))
