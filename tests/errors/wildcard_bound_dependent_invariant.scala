// A parameter's bound naming another parameter whose argument is a wildcard sees that argument as the skolem
// `C[? <: Int, Set[Int]]#A` (`Checking.checkAppliedType`'s `paramInfoAsSeenFrom(tree.tpe)`, Checking.scala 118, and
// `argForParam`'s `case _: TypeBounds if !widenAbstract => TypeRef(pre, tparam)`, Types.scala 2794): one unknown
// type, so `Set[Int] <: Set[<app>#A]` needs `Int =:= <app>#A`, and `Int <: <app>#A` fails (its lower end is
// `Nothing`). scalac 3.8.4 rejects it (E057):
//   10:21 Type argument Set[Int] does not conform to upper bound Set[C[? <: Int, Set[Int]]#A]
// teq rejects it as scalac does since round 23 (`skolemize_wildcard_args`, `open_skolems`); until then it substituted
// the wildcard itself into `B`'s bound, and `Set[Int] <: Set[? <: Int]` held by containment.
class C[A, B <: Set[A]]
type X = C[? <: Int, Set[Int]]
// expect: 10:22: error: type argument Set[Int] does not conform to upper bound Set[C[? <: Int, Set[Int]]#A]
// expect: 1 error found
