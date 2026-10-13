package oea

// A case class nested in a class tests its enclosing instance in `equals` and in a pattern through
// another instance (`SyntheticMembers.equalsBody`, `PatternMatcher.addOuterTest`), by the outer
// accessor scalac names `oea$O$I$$$outer` (`ExplicitOuter.outerAccName`), which a downstream of
// either compiler calls.
class O:
  case class I(n: Int)
  class Plain(val n: Int)
