// A match type's scrutinee is checked below its top: `PostTyper` transforms the `MatchTypeTree`'s selector with
// `super.transform(selector)` (PostTyper.scala 691-699), which checks the selector's arguments but not the selector's
// own application. So scalac 3.8.4 rejects the wildcard of `C` nested in a tuple scrutinee, as teq is to (E057):
//   7:12  Type argument  >: String does not overlap with upper bound Int
// (A scrutinee that is itself `C[? >: String]` is accepted by scalac.)
class C[T <: Int]
type N = (C[? >: String], Int) match
  case (Any, Int) => Int
// expect: 7:13: error: type argument ? >: String does not overlap with upper bound Int
// expect: 1 error found
