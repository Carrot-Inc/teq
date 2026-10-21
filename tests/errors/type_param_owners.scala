// dotty's `typeParam` by owner: a type lambda's binder may be `_` (`acceptsWildcard`) and takes no context bound
// ("context bounds are not allowed in type lambdas", `stripContextBounds`), a polymorphic function's takes them
// (`acceptsCtxBounds` from 3.6), a higher-kinded parameter's own one none, its `as` name read with the bound (E214).
// expect: 9:14: error: context bounds are not allowed in type lambdas
// expect: 10:20: error: Context bounds are not allowed in this position
trait Ord[X]
type W = [_] =>> Int
val w: W[String] = 1; val less = [X: Ord] => (x: X, y: X) => summon[Ord[X]] != null
type F = [X: Ordering] =>> List[X]
trait Higher[G[A : Ordering as ord]]
