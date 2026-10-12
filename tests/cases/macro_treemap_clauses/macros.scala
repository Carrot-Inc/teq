// A TreeMap rebuilds every Apply of a quoted call with `Apply.copy`: a curried call's inner application
// keeps its method type and the outer one completes it, a varargs clause's one `Repeated` argument taking
// its one parameter (`TypeAssigner.assignType`), as dotty's typed trees hold them.
import scala.quoted.*
object M:
  def f(xs: Int*): Int = xs.sum
  def g(a: Int)(xs: Int*): Int = a + xs.sum
  inline def mapped[T](inline x: T): T = ${ impl('x) }
  def impl[T: Type](x: Expr[T])(using Quotes): Expr[T] =
    import quotes.reflect.*
    val m = new TreeMap:
      override def transformTerm(tree: Term)(owner: Symbol): Term = tree match
        case Literal(IntConstant(n)) => Literal(IntConstant(n * 10))
        case _ => super.transformTerm(tree)(owner)
    m.transformTerm(x.asTerm)(Symbol.spliceOwner).asExprOf[T]
