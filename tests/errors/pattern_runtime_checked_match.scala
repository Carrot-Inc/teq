// teq: --werror
// A match on `.runtimeChecked` is exempt from the exhaustivity check, not from reachability
// (`SpaceEngine.exhaustivityCheckable` reads `RuntimeCheckedAnnot`, `reachabilityCheckable` only
// `UncheckedAnnot`): an unreachable case is still reported, a missing one is not.
def f(xs: Option[Int]): Int =
  xs.runtimeChecked match
    case is: Some[t] => is.get
    case is: Some[t] => 0
def g(xs: Option[Int]): Int =
  xs.runtimeChecked match
    case Some(x) => x
// expect: 8:10: warning: unreachable case
// absent: match may not be exhaustive
