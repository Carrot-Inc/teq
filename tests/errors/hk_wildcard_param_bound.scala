// expect: 13:43: error: Found:    (c : C[List, ?])
// expect: 1 error found
// The wildcard argument of the higher-kinded parameter `F` is a range in `B`'s bounds as the comparer reads them
// (`TypeComparer.isSubArgs`' `paramBounds`: `tparam.info.substApprox(tparams2, args2)`, a `TypeBounds` argument
// `range(lo, hi)`), so `B <: F[Int]` is bounded above by the top of `F`'s kind applied (`TypeApplications.appliedTo`),
// `Any`. `C[List, ?]`'s `?` met with those bounds is then no `? <: String`. scalac 3.8.4 rejects it:
//   -- [E007] Type Mismatch Error: hk_wildcard_param_bound.scala:13:42
//   Found:    (c : C[List, ?])
//   Required: C[?[_$1], ? <: String]
// With the first argument `List` the same program is rejected for the same `?` (scalac's E007 at the same place,
// `Required: C[List, ? <: String]`).
class C[F[_], B <: F[Int]]
def h(c: C[List, ?]): C[?, ? <: String] = c
// expect: Required: C[?, ? <: String]
