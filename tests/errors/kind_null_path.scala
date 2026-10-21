// `preCheckKind`'s exemption is a widened reference to `Nothing` alone: `n.type` for `n: Null` and a member bounded by
// `Nothing` (no alias of it) are compared by their kinds and refused, as scalac 3.8.4 refuses them (the review of
// `6c316c160`, finding 2's boundary).
// expect: 7:21: error: type argument (n : Null) does not have the same kind as its bound [_]
// expect: 10:14: error: type argument T.this.N does not have the same kind as its bound [_]
class Box[F[_]]
def f(n: Null): Box[n.type] = ???
trait T:
  type N <: Nothing
  def g: Box[N] = ???
