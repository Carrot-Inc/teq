// `narrowBound`'s `massage` (TypeOps.scala 664-670) of an abstract type member's bound, as of a type parameter's
// (fbound_through_sibling_member_alias.scala): `type S <: L[S]` with `L` an alias is dotty's `AppliedType` of the
// alias's `TypeRef`, no class's (`skolemizeWildcardArgs`' `tycon.typeSymbol.isClass`), kept, so no GADT bound is
// installed for `S` and the check of `BS[?, S]` fails, with `L` a member of the same trait (`H`) or a top-level alias
// (`M`, `LT`) alike: scalac 3.8.4's E057 at 12:17 and 15:17 (0-based columns). Written expanded, `type S <:
// BS[?, S]`, the bound is massaged and the check holds (`K`; wildcard_bound_fbound_narrow_accepts.scala's `H`).
trait BS[T, S <: BS[T, S]]
type LT[X <: BS[?, X]] = BS[?, X]
trait H:
  type L[X <: BS[?, X]] = BS[?, X]
  type S <: L[S]
  def g(x: BS[?, S]): Int = 1
trait M:
  type S <: LT[S]
  def g(x: BS[?, S]): Int = 1
trait K:
  type S <: BS[?, S]
  def g(x: BS[?, S]): Int = 1
// expect: fbound_through_sibling_member_alias_member.scala:12:18: error: type argument H.this.S does not conform to upper bound BS[BS[?, H.this.S]#T, H.this.S]
// expect: fbound_through_sibling_member_alias_member.scala:15:18: error: type argument M.this.S does not conform to upper bound BS[BS[?, M.this.S]#T, M.this.S]
// expect: 2 errors found
