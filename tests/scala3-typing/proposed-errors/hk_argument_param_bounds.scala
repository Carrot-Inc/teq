// A constructor argument whose parameter is bounded more narrowly than the formal's (`C[G <: P#F]` for `H[G[_]]`):
// the kinds agree (`preCheckKind`), and PostTyper's `checkBounds` refuses it, E057 `Type argument C does not conform to
// upper bound [G[_$1]] =>> Any` (the parameters' bounds compared contravariantly, `TypeComparer.compareTypeLambda`'s
// `variancesConform` and its bounds). teq accepts it (master's): a part of its own (found by the fold of the review of
// `b8eb8842a`, beside `tests/modules/loadedindirectkindok`). scalac 3.8.4's line, at its tree's point.
// expect: 13:17: error: Type argument C does not conform to upper bound [G[_$1]] =>> Any
trait P:
  type F[X]
class C[G <: P#F]

class Box[H[G[_]]]
object Use:
  type Bad = Box[C]
