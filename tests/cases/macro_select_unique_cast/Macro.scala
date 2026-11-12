// `TypeApply(Select.unique(x, "asInstanceOf"), List(T))`: `asInstanceOf` is a member of `Any` with one type
// parameter (`Definitions.Any_asInstanceOf`), and the tree is the cast `x.asInstanceOf[T]`.
import scala.quoted.*

inline def castTo[T](inline x: Any): T = ${ castToImpl[T]('x) }

def castToImpl[T: Type](x: Expr[Any])(using q: Quotes): Expr[T] =
  import q.reflect.*
  TypeApply(Select.unique(x.asTerm, "asInstanceOf"), List(Inferred(TypeRepr.of[T]))).asExprOf[T]
