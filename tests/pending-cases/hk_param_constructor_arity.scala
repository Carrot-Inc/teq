// `typedAppliedTypeTree`'s `wrongNumberOfTypeArgs` (E023, Typer.scala 2733-2735) for a constructor that is a
// higher-kinded type parameter (`G[_]` applied to two, `F <: [X, Y] =>> K[Y, X]` applied to one): its `typeParams`
// are its bound's. scalac 3.8.4 reports both. teq reports the arity of a class's and a lambda's application only and
// accepts these (master the same; lane P's K6 row, `pi-scratch/r24p/own/k15`). The closure adversary of the pass on
// 97f0487ac (round 24).
class K[X, Y]
def f[G[_]](x: G[Int, List]): Unit = ()
def g[F <: [X, Y] =>> K[Y, X]](x: F[Int]): Int = 1
