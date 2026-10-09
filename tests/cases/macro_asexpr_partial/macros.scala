import scala.quoted.*

// A `TreeMap` that asks for every term as an expression under `Try`, as utest's `Tests` does
// to find the `assert`s: a method selected without its arguments is no expression, and
// `asExpr` throws for it as scalac's does; `isExpr` says so beforehand.
object Macros:
  inline def tried[A](inline body: A): A = ${ triedImpl('body) }
  def triedImpl[A: Type](body: Expr[A])(using Quotes): Expr[A] =
    import quotes.reflect.*
    var failed = 0
    var partial = ""
    var notExpr = 0
    val map = new TreeMap:
      override def transformTerm(t: Term)(owner: Symbol): Term =
        if !t.isExpr then notExpr += 1
        scala.util.Try(t.asExpr) match
          case scala.util.Success(_) => super.transformTerm(t)(owner)
          case scala.util.Failure(e) =>
            failed += 1
            partial = e.getMessage
            super.transformTerm(t)(owner)
    val out = map.transformTerm(body.asTerm)(Symbol.spliceOwner).asExprOf[A]
    val seen = Expr(s"${failed > 0} ${failed == notExpr} $partial ${body.asTerm.isExpr}")
    '{ println($seen); $out }
