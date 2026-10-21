/*
rules = DisableSyntax
*/
// scalafix's DisableSyntax applies `DefaultMatcher.orElse(FinalizeMatcher)` to every node, and `FinalizeMatcher` reads
// no setting (DisableSyntax.scala at 41cf7983, `checkTree`, `FinalizeMatcher`): a `finalize` of no parameters, or one
// empty clause, is reported with every setting off. A case of teq's own, from that source.
package finalizes

class Resource:
  def finalize(): Unit = () // assert: DisableSyntax.noFinalize
  def finalize(x: Int): Int = x
