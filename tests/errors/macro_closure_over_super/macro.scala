import scala.quoted.*
inline def close(inline x: Int): () => Int = ${ closeImpl('x) }
def closeImpl(x: Expr[Int])(using q: Quotes): Expr[() => Int] =
  import q.reflect.*
  def strip(t: Term): Term = t match
    case Inlined(_, _, v) => strip(v)
    case t => t
  val Apply(m, Nil) = strip(x.asTerm): @unchecked
  Closure(m, None).asExprOf[() => Int]
