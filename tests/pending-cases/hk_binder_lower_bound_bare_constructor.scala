// A bare class constructor as a higher-kinded binder's lower bound (`F[_] >: List`) is the lambda `[_] =>> List`
// over the unapplied constructor (`typedTypeBoundsTree`), which no argument conforms to: scalac 3.8.4 rejects the
// three applications with E057 ("does not conform to lower bound [_] =>> List"). teq resolves the bound to
// `[_] =>> List[<error>]` and accepts them (master the same; lane P's summary, `pi-scratch/r24p/own/h01`). The
// closure adversary of the pass on 97f0487ac (round 24).
trait C[F[_] >: List]
type Ok = C[Seq]
type Ok2 = C[List]
def f[G[F[_] >: List]](x: G[Seq]): Unit = ()
