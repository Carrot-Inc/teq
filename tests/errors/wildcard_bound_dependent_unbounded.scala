// The unbounded wildcard as well: `C[?, Option[Int]]`'s `B <: Option[A]` is `Option[C[?, Option[Int]]#A]`
// (Checking.scala 118, Types.scala 2794), which `Option[Int]` is not below. scalac 3.8.4 rejects it (E057):
//   7:14 Type argument Option[Int] does not conform to upper bound Option[C[?, Option[Int]]#A]
// teq rejects it as scalac does since round 23; until then the `?` of the invariant parameter, `WILD`, was substituted
// into the bound as `Option[?]`.
class C[A, B <: Option[A]]
type X = C[?, Option[Int]]
// expect: 7:15: error: type argument Option[Int] does not conform to upper bound Option[C[?, Option[Int]]#A]
// expect: 1 error found
