// A macro that maps its argument's tree unchanged (utest's `assert` rebuilds the expression it
// checks so): an enum case's construction stays the case's, not its enum's.
import scala.quoted.*

inline def same[T](inline x: T): T = ${ sameImpl('x) }

def sameImpl[T: Type](x: Expr[T])(using Quotes): Expr[T] =
  import quotes.reflect.*
  new TreeMap {}.transformTerm(x.asTerm)(Symbol.spliceOwner).asExprOf[T]
