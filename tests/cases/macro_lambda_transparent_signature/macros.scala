// `Symbol.tree` of the lambda around a transparent inline macro, which expands while the lambda is
// typed: the `$anonfun` keeps the signature the expected function gives it, its body absent
// (dotty's `FromSymbol.definitionFromSym` from the symbol's info).
import scala.quoted.*

transparent inline def inspect: String = ${ impl }
def impl(using Quotes): Expr[String] =
  import quotes.reflect.*
  val s = Symbol.spliceOwner.owner
  val d = s.tree.asInstanceOf[DefDef]
  Expr(d.name + ":" + d.paramss.flatMap(_.params).map(_.name).mkString(",") + ":" + d.returnTpt.tpe.show + ":rhs=" + d.rhs.isDefined)

transparent inline def inspectCount: Int = ${ countImpl }
def countImpl(using Quotes): Expr[Int] =
  import quotes.reflect.*
  val d = Symbol.spliceOwner.owner.tree.asInstanceOf[DefDef]
  Expr(d.paramss.flatMap(_.params).size)
