// The dependent bound over a wildcard in a contravariant position: `B <: (A => Int)` with `A := ? >: Int` is
// `C[? >: Int, Int => Int]#A => Int` (Checking.scala 118, Types.scala 2794), and `(Int => Int) <: (<app>#A => Int)`
// needs `<app>#A <: Int`, whose upper end is `Any`. scalac 3.8.4 rejects it (E057):
//   9:25 Type argument Int => Int does not conform to upper bound C[? >: Int, Int => Int]#A => Int
// teq rejects it as scalac does since round 23 (`skolemize_wildcard_args`, the skolem opened as one unknown type
// with the argument's denotation, `open_skolems`); until then it substituted the wildcard and accepted it. With
// `? <: Int` both accept (scalac's position is the function arrow's, teq's the type's start).
class C[A, B <: (A => Int)]
type X = C[? >: Int, Int => Int]
// expect: 9:22: error: type argument Int => Int does not conform to upper bound C[? >: Int, Int => Int]#A => Int
// expect: 1 error found
