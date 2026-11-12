// A `return` in a new method's body returns from that method: the block of the definition and its closure
// keeps the definition (the lambda `Lambda.unapply` reads has no method of its own to return from here).
import scala.quoted.*

inline def early: Int => Int = ${ earlyImpl }

def earlyImpl(using q: Quotes): Expr[Int => Int] =
  import q.reflect.*
  val tpe = MethodType(List("t"))(_ => List(TypeRepr.of[Int]), _ => TypeRepr.of[Int])
  val sym = Symbol.newMethod(Symbol.spliceOwner, "added", tpe)
  val defined = DefDef(sym, {
    case List(List(t: Term)) =>
      val ret = Return('{ ${ t.asExprOf[Int] } + 1 }.asTerm, sym).asExprOf[Nothing]
      Some('{ if ${ t.asExprOf[Int] } > 0 then $ret else 0 }.asTerm.changeOwner(sym))
    case other => report.errorAndAbort("the parameters the body was given: " + other)
  })
  Block(List(defined), Closure(Ref(sym), None)).asExprOf[Int => Int]
