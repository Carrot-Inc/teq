import teq.fix.*

/** Adds a comment left of each string literal, once: the token beside it is copied as it is, a multiline string's
  * line breaks with it (tests/fix/Driver/RetainedToken). */
final class MarkStrings extends SyntacticRule("MarkStrings"):
  override def fix(using doc: SyntacticDocument): Patch =
    if doc.input.text.contains("/*fixed*/") then Patch.empty
    else doc.tree.collect { case t: Lit.String => Patch.addLeft(t, "/*fixed*/") }.asPatch
