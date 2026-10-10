// expect: 9:28: warning: forward reference in refinement is deprecated
// expect: 10:48: warning: forward reference in refinement is deprecated
// expect: 2 warnings found, errors under --werror
// absent: 11:
// teq: --werror
// scalac warns of a member a refinement names before declaring it, the members being one scope
// (`Checking.checkRefinementNonCyclic`); a member named after it is declared, or one of the
// parent, is no forward reference.
type R = AnyRef { type U = T; type T }
type S = AnyRef { def f(x: Int): Int; type V = W; type W = Int }
type Q = AnyRef { type A; type B = A }
