// `typedAppliedTypeTree`'s `wrongNumberOfTypeArgs`
// (E023, Typer.scala 2733-2735) for a constructor that
// is a higher-kinded type parameter (`G[_]` applied to
// two, `F <: [X, Y] =>> K[Y, X]` applied to one): its
// `typeParams` are its bound's. scalac 3.8.4 reports both.
class K[X, Y]
def f[G[_]](x: G[Int, List]): Unit = ()
def g[F <: [X, Y] =>> K[Y, X]](x: F[Int]): Int = 1
