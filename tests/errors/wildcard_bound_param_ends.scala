// A bounded wildcard argument overlaps its parameter's bounds by its ends (`TypeOps.boundsViolations`'
// `checkOverlapsBounds`, TypeOps.scala 700-701): the written lower end within the parameter's upper bound and the
// parameter's lower bound within the written upper end. scalac 3.8.4 reports each at the argument (E057):
//   9:12  Type argument  >: String does not overlap with upper bound Int
//   11:12 Type argument  <: Int does not overlap with lower bound String
//   13:12 Type argument  >: Boolean does not overlap with upper bound Int | String
// and accepts `Up[? <: String]` (the ends `Nothing..String` overlap `Nothing..Int`).
class Up[T <: Int]
type A = Up[? >: String]
class Lo[T >: String]
type B = Lo[? <: Int]
class Un[T <: Int | String]
type C = Un[? >: Boolean]
type D = Up[? <: String]
// expect: 9:13: error: type argument ? >: String does not overlap with upper bound Int
// expect: 11:13: error: type argument ? <: Int does not overlap with lower bound String
// expect: 13:13: error: type argument ? >: Boolean does not overlap with upper bound Int | String
// expect: 3 errors found
