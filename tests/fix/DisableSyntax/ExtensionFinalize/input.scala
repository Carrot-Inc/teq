/*
rules = DisableSyntax
*/
// An extension method's clauses are its own, the receiver's its `Defn.ExtensionGroup`'s, so `Defn.Def.Initial`'s first
// group of `extension (s: String) def finalize()` is `()` and `FinalizeMatcher` reports it (DisableSyntax.scala at
// 41cf7983), as it reports a `finalize()` of a class.
package extfinalize

object Ext:
  extension (s: String) def finalize(): Unit = () // assert: DisableSyntax.noFinalize
