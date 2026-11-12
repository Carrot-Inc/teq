// `Symbol.newMethod` with a `MethodType`: the type is the symbol's info (`QuotesImpl`'s `newMethod`), and
// `DefDef.apply` hands its body one reference per name of it (`tpd.DefDef(sym, rhsFn)`), here `List(List(t))`.
import scala.quoted.*

inline def addOneTo(inline n: Int): Int = ${ addOneToImpl('n) }

def addOneToImpl(n: Expr[Int])(using q: Quotes): Expr[Int] =
  import q.reflect.*
  val tpe = MethodType(List("t"))(_ => List(TypeRepr.of[Int]), _ => TypeRepr.of[Int])
  val sym = Symbol.newMethod(Symbol.spliceOwner, "added", tpe)
  val defined = DefDef(sym, {
    case List(List(t: Term)) => Some('{ ${ t.asExprOf[Int] } + 1 }.asTerm.changeOwner(sym))
    case other => report.errorAndAbort("the parameters the body was given: " + other)
  })
  Block(List(defined), Apply(Ref(sym), List(n.asTerm))).asExprOf[Int]
