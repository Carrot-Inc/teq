// A higher-kinded parameter's own parameter takes no context bound (dotty's `Parsers.typeParam` under
// `ParamOwner.Hk`, whose `acceptsCtxBounds` is false): the bound is parsed and reported, as scalac 3.8.4 reports
// `IllegalContextBounds`; an annotation there is read (`annotsAsMods`).
// expect: 7:20: error: Context bounds are not allowed in this position
class ann extends scala.annotation.StaticAnnotation
trait Annotated[F[@ann A]]
trait Bounded[F[A: Ordering]]
