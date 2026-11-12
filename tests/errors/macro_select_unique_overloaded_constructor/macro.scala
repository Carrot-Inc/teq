import scala.quoted.*

inline def make[T](inline x: String): T = ${ makeImpl[T]('x) }

def makeImpl[T: Type](x: Expr[String])(using q: Quotes): Expr[T] =
  import q.reflect.*
  Apply(Select.unique(New(Inferred(TypeRepr.of[T])), "<init>"), List(x.asTerm)).asExprOf[T]
