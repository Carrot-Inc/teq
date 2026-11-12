// A lower-bounded wildcard of an F-bounded parameter whose lower end is no instance of the F-bound at all: scalac
// rejects it, and teq is to reject it. scalac shows the bound seen from the application, the wildcard's own
// parameter a skolem (`paramInfoAsSeenFrom`, `argForParam`'s `TypeRef(pre, tparam)`, Types.scala 2794);
// the expectation shows it so, and the wildcard as `? >: String`. scalac's messages (E057):
//   10:11 Type argument  >: String does not overlap with upper bound D[D[? >: String]#T]
//   12:11 Type argument  >: Int does not overlap with upper bound Comparable[C[? >: Int]#T]
//   13:24 Type argument  >: String does not overlap with upper bound Enum[Enum[? >: String]#E]
// (A lower end that is an instance, `D[? >: E]` for `E extends D[E]`: wildcard_bound_fbound_lower_instance.scala.)
class D[T <: D[T]]
type X = D[? >: String]
class C[T <: Comparable[T]]
type Y = C[? >: Int]
type Z = java.lang.Enum[? >: String]
// expect: 10:12: error: type argument ? >: String does not overlap with upper bound D[D[? >: String]#T]
// expect: 12:12: error: type argument ? >: Int does not overlap with upper bound Comparable[C[? >: Int]#T]
// expect: 13:25: error: type argument ? >: String does not overlap with upper bound Enum[Enum[? >: String]#E]
// expect: 3 errors found
