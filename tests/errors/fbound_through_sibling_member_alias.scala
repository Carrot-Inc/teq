// `narrowBound`'s `massage` (TypeOps.scala 664-670) skolemizes the wildcards of a class's application only
// (`skolemizeWildcardArgs`: `tycon.typeSymbol.isClass`): `S <: L[S]` with `L` an alias is kept, no GADT bound is
// installed for `S`, and the check of `BS[?, S]`
// (`S <: BS[BS[?, S]#T, S]`) fails: scalac 3.8.4's E057.
// teq is to report it for an alias that is a member of
// the method's own trait (`H`). scalac's columns are 0-based.
trait BS[T, S <: BS[T, S]]
trait H:
  type L[X <: BS[?, X]] = BS[?, X]
  def g[S <: L[S]](x: BS[?, S]): Int = 1
// expect: fbound_through_sibling_member_alias.scala:10:29: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 1 error found
