// `Symbol.tree` of the lambda around a macro's call, where the call expands after the lambda's
// typing: its definition as typed, its result type the expected function's and its body present
// (dotty's `FromSymbol.definitionFromSym` takes the stored `DefDef` before the symbol's own).
import scala.quoted.*

inline def inspect: String = ${ impl }
def impl(using Quotes): Expr[String] =
  import quotes.reflect.*
  val s = Symbol.spliceOwner.owner
  val d = s.tree.asInstanceOf[DefDef]
  Expr(d.name + ":" + d.paramss.flatMap(_.params).map(_.name).mkString(",") + ":" + d.returnTpt.tpe.show + ":rhs=" + d.rhs.isDefined)

inline def inspectCount: Int = ${ countImpl }
def countImpl(using Quotes): Expr[Int] =
  import quotes.reflect.*
  val d = Symbol.spliceOwner.owner.tree.asInstanceOf[DefDef]
  Expr(d.paramss.flatMap(_.params).size)
