import teq.fix.*

/** A rule that keeps a token of the first document it reads and replaces it while it reads the next one: a patch made
  * from another document, which the run refuses as the token's accessors refuse it (tests/fix/Driver/StalePatch). */
final class Stale extends SyntacticRule("Stale"):
  private var saved: Option[Token] = None
  private var done = false
  override def fix(using doc: SyntacticDocument): Patch =
    if saved.isEmpty then
      saved = doc.tokens.toList.find(t => t.text == "old" || t.text == "now")
      Patch.empty
    else if !done then
      done = true
      Patch.replaceToken(saved.get, "changed")
    else Patch.empty
