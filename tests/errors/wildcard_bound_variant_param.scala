// A bounded wildcard of a covariant or contravariant parameter is checked by its written ends like any other:
// `Co[? >: String]` for `+T <: AnyVal`, `Ct[? <: Int]` for `-T >: Null`. Both reject and name the wildcard, its
// bounds kept whatever the parameter's variance (`typedAppliedTypeTree`'s `typedArg`), scalac 3.8.4 (E057):
//   9:12  Type argument  >: String does not overlap with upper bound AnyVal
//   11:12 Type argument  <: Int does not overlap with lower bound Null
// and both accept `Co[? <: String]` and `Ct[? >: Int]`.
class Co[+T <: AnyVal]
class Ct[-T >: Null]
type A = Co[? >: String]
type B = Co[? <: String]
type C = Ct[? <: Int]
type D = Ct[? >: Int]
// expect: 9:13: error: type argument ? >: String does not overlap with upper bound AnyVal
// expect: 11:13: error: type argument ? <: Int does not overlap with lower bound Null
// expect: 2 errors found
