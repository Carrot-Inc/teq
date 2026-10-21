// A type parameter's `_` is `WildcardParamName.fresh().toTypeName`, a derived name, not the wildcard's; a backquoted
// wildcard or an empty backquoted name is refused by the scanner (`Scanners.getBackquotedIdent`, the review of
// `4f8b8bb39`'s WildcardTypeName). scalac 3.8.4 gives these four errors.
// expect: 9:9: error: wildcard invalid as backquoted identifier
// expect: 10:10: error: wildcard invalid as backquoted identifier
// expect: 11:13: error: wildcard invalid as backquoted identifier
// expect: 12:7: error: empty quoted identifier
object W {
  def f[`_`](x: Int) = x
  type T[`_`] = Int
  class C[F[`_`]]
  val `` = 1
}
