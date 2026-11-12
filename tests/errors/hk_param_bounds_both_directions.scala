// expect: 18:14: error: type argument Option does not conform to lower bound [X] =>> List[X]
// expect: 19:43: error: type argument Option does not conform to lower bound [X] =>> List[X]
// expect: 21:20: error: type argument Option does not conform to lower bound [X] =>> List[X]
// expect: 23:15: error: type argument Option does not conform to lower bound [X] =>> List[X]
// expect: 26:15: error: type argument ? <: [X] =>> Option[X] does not overlap with lower bound [X] =>> [Y] =>> List[Y]
// expect: 27:16: error: type argument ? <: [X] =>> Iterable[X] does not overlap with lower bound [X] =>> [Y] =>> List[Y]
// expect: 30:15: error: type argument Option does not conform to upper bound [X] =>> Seq[X]
// expect: 7 errors found
// `TypeOps.boundsViolations`' loop (TypeOps.scala 704-739) and `check` (688-701) hold a higher-kinded parameter's
// argument to both bounds as a proper one's: its lower bound below the argument (`check(loBound, hi)`), the
// constructors compared, and a bounded wildcard's ends overlapping both
// bounds. scalac 3.8.4 rejects each `Bad` line (E057) and accepts the
// others: a lambda written as the bound of `F[X]` is the result of `F`'s own
// lambda (`[X] =>> [Y] =>> List[Y]`), below no constructor of `F`'s kind.
trait C[F[X] >: List[X]]
type Ok = C[Seq]
type Ok2 = C[List]
type Bad = C[Option]
def f[G[F[X] >: List[X]]](x: G[Seq], y: G[Option]): Unit = ()
trait H { type M[F[X] >: List[X]] }
def g(h: H)(x: h.M[Option], y: h.M[Seq]): Unit = ()
type A[F[X] >: List[X]] = Int
type BadA = A[Option]
type OkA = A[Seq]
trait D[F[X] >: [Y] =>> List[Y]]
type BadW = D[? <: ([X] =>> Option[X])]
type BadW2 = D[? <: ([X] =>> Iterable[X])]
trait E[F[X] <: Seq[X]]
type OkE = E[List]
type BadE = E[Option]
