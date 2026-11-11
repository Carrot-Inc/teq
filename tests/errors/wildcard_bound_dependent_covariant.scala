// The dependent bound over a wildcard as a skolem (Checking.scala 118, Types.scala 2794) also under a covariant
// constructor: `List[Int] <: List[C[? <: Int, List[Int]]#A]` needs `Int <: <app>#A`, whose lower end is `Nothing`.
// scalac 3.8.4 rejects it (E057):
//   8:21 Type argument List[Int] does not conform to upper bound List[C[? <: Int, List[Int]]#A]
// teq rejects it as scalac does since round 23 (`skolemize_wildcard_args`, `open_skolems`); until then it substituted
// the wildcard as written and accepted it (`List[Int] <: List[? <: Int]`). The review's E057 program.
class C[A, B <: List[A]]
type X = C[? <: Int, List[Int]]
// expect: 8:22: error: type argument List[Int] does not conform to upper bound List[C[? <: Int, List[Int]]#A]
// expect: 1 error found
