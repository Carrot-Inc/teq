// A new method's definition, the block of it and its closure, and its parameter symbols as scalac prints
// them: the parameters have the method type's names, and the block is the lambda `Lambda.unapply` reads.
import scala.quoted.*

inline def shown: String = ${ shownImpl }

def shownImpl(using q: Quotes): Expr[String] =
  import q.reflect.*
  val tpe = MethodType(List("t"))(_ => List(TypeRepr.of[Int]), _ => TypeRepr.of[Int])
  val sym = Symbol.newMethod(Symbol.spliceOwner, "added", tpe)
  val defined = DefDef(sym, {
    case List(List(t: Term)) => Some('{ ${ t.asExprOf[Int] } + 1 }.asTerm.changeOwner(sym))
    case other => report.errorAndAbort("the parameters the body was given: " + other)
  })
  Expr(defined.show + " / " + Block(List(defined), Closure(Ref(sym), None)).show + " / " + sym.paramSymss.map(_.map(_.name)))
