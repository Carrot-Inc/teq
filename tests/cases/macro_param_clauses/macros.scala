// The parameter clauses of a `DefDef` as scalac's reflection API holds them at run time: a term
// clause is the list of its `ValDef`s, so a lambda matches `DefDef(_, List(List(ValDef(..))), ..)`
// and `(c: List[?]) :: Nil`, and the clause extractors and type tests take the same list.
import scala.quoted.*

object Macros:
  def shapeImpl(f: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def go(t: Tree): String = t match
      case Inlined(_, _, b) => go(b)
      case Block(List(DefDef(_, List(List(ValDef(in, tpt, _))), _, Some(body))), _) =>
        s"one param $in: ${tpt.tpe.show}, body ${body.show}"
      case Block(List(DefDef(_, (c: List[?]) :: Nil, _, _)), _) => s"one clause of ${c.size}"
      case other => "other"
    def clauses(t: Tree): String = t match
      case Inlined(_, _, b) => clauses(b)
      case Block(List(d: DefDef), _) =>
        d.paramss.map {
          case tc: TermParamClause =>
            val TermParamClause(ps) = tc: @unchecked
            s"term(${ps.map(_.name).mkString(", ")}) given=${tc.isGiven}"
          case _: TypeParamClause => "type"
        }.mkString("; ")
      case _ => "none"
    Expr(go(f.asTerm) + " | " + clauses(f.asTerm))

  inline def describe[A](inline f: A => Any): String = ${ shapeImpl('f) }
  inline def describe2[A, B](inline f: (A, B) => Any): String = ${ shapeImpl('f) }
