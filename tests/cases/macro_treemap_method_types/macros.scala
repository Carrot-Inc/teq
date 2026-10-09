import scala.quoted.*

// A `TreeMap` over a body as utest's `Tests` walks it: it looks at a term's type before it
// asks for the term as an expression (a selection of a method awaiting its arguments has a
// method type, a generic one's a polymorphic type), counts what it met and gives the body
// back rebuilt: operators, equality, closures and constructors among it.
object Macros:
  inline def typed[A](inline body: A): A = ${ typedImpl('body) }
  def typedImpl[A: Type](body: Expr[A])(using Quotes): Expr[A] =
    import quotes.reflect.*
    var methods = 0
    var polys = 0
    var values = 0
    val map = new TreeMap:
      override def transformTerm(t: Term)(owner: Symbol): Term =
        t.tpe.widen match
          case _: MethodType =>
            methods += 1
            super.transformTerm(t)(owner)
          case _: PolyType =>
            polys += 1
            super.transformTerm(t)(owner)
          case _ =>
            t.asExpr match
              case '{ "never" } => t
              case _ =>
                values += 1
                super.transformTerm(t)(owner)
    val out = map.transformTerm(body.asTerm)(Symbol.spliceOwner).asExprOf[A]
    val seen = Expr(s"${methods > 0} ${polys > 0} ${values > 0}")
    '{ println($seen); $out }
