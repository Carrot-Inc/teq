// A quote pattern's hole whose type the pattern's inference fixes (`Handler.instance($v)` under `Box.build[V](..)`, Airstream's
// split macros): bound at the solved type, `V`, as dotty types a `SplicePattern` against its prototype, where teq took
// `Any` while the type argument of `instance` was open.
import scala.quoted.*
final class Handler[V](val v: V)
object Handler:
  def instance[V](v: V): Handler[V] = new Handler(v)
final class Box[V](val h: Handler[V])
object Box:
  def build[V](h: Handler[V]): Box[V] = new Box(h)
object M:
  inline def twice[V](inline b: Box[V]): List[V] = ${ twiceImpl('b) }
  def twiceImpl[V: Type](b: Expr[Box[V]])(using Quotes): Expr[List[V]] =
    b match
      case '{ Box.build[V](Handler.instance($v)) } =>
        val typed: Expr[V] = v
        '{ List($typed, $typed) }
      case _ => '{ Nil }
