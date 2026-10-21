/*
rules = DisableSyntax
DisableSyntax.noAsInstanceOf = true
DisableSyntax.noIsInstanceOf = true
*/
// An identifier is compared by its name, its backquotes off (scalameta's `Token.Ident.value`; the review of
// `86884ea36`): each backquoted cast and test is reported, `Matchable` among them.
object BackquotedCasts {
  val x: Any = 1
  val y = x.`asInstanceOf`[Int] // assert: DisableSyntax.asInstanceOf
  val z = x.`isInstanceOf`[Int] // assert: DisableSyntax.isInstanceOf
  val m = x.asInstanceOf[`Matchable`] // assert: DisableSyntax.asInstanceOfMatchable
}
