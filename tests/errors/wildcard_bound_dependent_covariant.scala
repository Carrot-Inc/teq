// The dependent bound over a wildcard as a skolem (Checking.scala 118, Types.scala 2794) also under a covariant
// constructor: `List[Int] <: List[C[? <: Int, List[Int]]#A]` needs `Int <: <app>#A`, whose lower end is `Nothing`.
// scalac 3.8.4 rejects it (E057):
//   8:21 Type argument List[Int] does not conform to upper bound List[C[? <: Int, List[Int]]#A]
// teq is to reject it as scalac does; substituting
// the wildcard as written accepts it (`List[Int] <: List[? <: Int]`).
class C[A, B <: List[A]]
type X = C[? <: Int, List[Int]]
// expect: 8:22: error: type argument List[Int] does not conform to upper bound List[C[? <: Int, List[Int]]#A]
// expect: 1 error found
