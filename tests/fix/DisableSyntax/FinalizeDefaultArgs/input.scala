/*
rules = DisableSyntax
DisableSyntax.noDefaultArgs = true
*/
// `FinalizeMatcher` applies only where `DefaultMatcher` is undefined: under `noDefaultArgs` its `DefaultArgs` case is
// defined at every `Defn.Def` (its `Some(Nil)` for a definition with no default), so a `finalize` is claimed by it and
// reported by neither (DisableSyntax.scala at 41cf7983, `checkTree`). A case of teq's own, from that source.
package finalizes

class Resource:
  def finalize(): Unit = ()
  def g(a: Int = 1) = a // assert: DisableSyntax.defaultArgs
