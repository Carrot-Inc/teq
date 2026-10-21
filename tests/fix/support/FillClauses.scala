import teq.fix.*

/** Replaces each empty parameter clause `()` through `replaceTree`: a clause spans its parentheses, an empty one too
  * (tests/fix/Driver/ParamClauseSpans). */
final class FillClauses extends SyntacticRule("FillClauses"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case t: Term.ParamClause if t.syntax == "()" => Patch.replaceTree(t, "(x: Int)") }.asPatch
