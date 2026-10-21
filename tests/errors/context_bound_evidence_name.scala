// A context bound's fresh evidence is `ContextBoundParamName`, a derived name no written identifier equals: `evidence$1`
// written is not found, of a polymorphic function literal, a method, a class (the review of `50e3a1b38`); a bound's own
// `as` name is the parameter's.
// expect: 8:42: error: not found: evidence$1
// expect: 9:33: error: not found: evidence$1
// expect: 10:39: error: not found: evidence$1
// absent: context_bound_evidence_name.scala:11:
val literal = [X: Ordering] => (x: X) => evidence$1
def method[X: Ordering](x: X) = evidence$1
class Holder[X: Ordering] { def get = evidence$1 }
val named = [X: Ordering as ord] => (x: X) => ord
