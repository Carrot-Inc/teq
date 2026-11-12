// The dependent bound over a wildcard in a contravariant position: `B <: (A => Int)` with `A := ? >: Int` is
// `C[? >: Int, Int => Int]#A => Int` (Checking.scala 118, Types.scala 2794), and `(Int => Int) <: (<app>#A => Int)`
// needs `<app>#A <: Int`, whose upper end is `Any`. scalac 3.8.4 rejects it (E057):
//   9:25 Type argument Int => Int does not conform to upper bound C[? >: Int, Int => Int]#A => Int
// teq is to reject it as scalac does; substituting the wildcard accepts it.
// With `? <: Int` scalac accepts it (the expectation's position
// is the type's start, scalac's the function arrow's).
class C[A, B <: (A => Int)]
type X = C[? >: Int, Int => Int]
// expect: 9:22: error: type argument Int => Int does not conform to upper bound C[? >: Int, Int => Int]#A => Int
// expect: 1 error found
