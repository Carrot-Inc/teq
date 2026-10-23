import teq.fix.*

/** Adds `2` right of each `1` token (tests/fix/Merge/EqualAdditions). */
final class AppendTwo extends SyntacticRule("AppendTwo"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tokens.filter(_.text == "1").map(t => Patch.addRight(t, "2")).asPatch
