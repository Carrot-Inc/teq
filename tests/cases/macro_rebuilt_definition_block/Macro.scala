// A block a program wrote, of a method and a lambda that applies it, put together again by a `TreeMap`: it stays
// that block, its method the one a `return` in its body leaves.
import scala.quoted.*

inline def same[T](inline x: T): T = ${ sameImpl[T]('x) }

def sameImpl[T: Type](x: Expr[T])(using q: Quotes): Expr[T] =
  import q.reflect.*
  val m = new TreeMap {}
  m.transformTerm(x.asTerm)(Symbol.spliceOwner).asExprOf[T]
