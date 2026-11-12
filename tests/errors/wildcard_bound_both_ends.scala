// A bounded wildcard out of its parameter's bounds at both ends: `? >: Int <: Boolean` for `T >: Null <: AnyRef`
// fails both `Int <: AnyRef` and `Null <: Boolean` (TypeOps.scala 700-701), two violations at one position. scalac
// 3.8.4 shows only the first, its reporter hiding a second error at the same position (`UniqueMessagePositions`):
//   8:11  Type argument  >: Int <: Boolean does not overlap with upper bound AnyRef
// scalac does not show `does not overlap with lower bound Null` at the same position;
// the line it shows is expected below.
class C[T >: Null <: AnyRef]
type A = C[? >: Int <: Boolean]
// expect: 8:12: error: type argument ? >: Int <: Boolean does not overlap with upper bound AnyRef
