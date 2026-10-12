// A macro rebuilding calls `Apply` by `Apply` (a `TreeMap`): the selection of an extension is the
// member of its prefix, its receiver the first argument, its clauses its method type's
// (`TypeAssigner.assignType` on `fun.tpe.widen`); `a -> b`, which scalac's trees hold as
// `ArrowAssoc(a).->(b)`, rebuilt the same way.
import scala.quoted.*
object E:
  extension (x: Int) def plus(y: Int)(z: Int): Int = x + y + z
object Mac:
  inline def rebuilt[T](inline x: T): T = ${ impl('x) }
  def impl[T: Type](x: Expr[T])(using Quotes): Expr[T] =
    import quotes.reflect.*
    val mapper = new TreeMap:
      override def transformTerm(t: Term)(owner: Symbol): Term = t match
        case Apply(f, as) => Apply(transformTerm(f)(owner), as.map(transformTerm(_)(owner)))
        case _ => super.transformTerm(t)(owner)
    mapper.transformTerm(x.asTerm)(Symbol.spliceOwner).asExprOf[T]
  inline def copied[T](inline x: T): T = ${ copiedImpl('x) }
  def copiedImpl[T: Type](x: Expr[T])(using Quotes): Expr[T] =
    import quotes.reflect.*
    (new TreeMap {}).transformTerm(x.asTerm)(Symbol.spliceOwner).asExprOf[T]
