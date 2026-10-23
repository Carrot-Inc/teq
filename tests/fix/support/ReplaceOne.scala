import teq.fix.*

/** Replaces each `1` token with `2` (tests/fix/Merge/EqualAdditions). */
final class ReplaceOne extends SyntacticRule("ReplaceOne"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tokens.filter(_.text == "1").map(t => Patch.replaceToken(t, "2")).asPatch
