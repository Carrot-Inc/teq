import teq.fix.*

/** Replaces `1L`, `1.5f` and `2.0d`, each found by its text, through `replaceTree`, and reports each value definition
  * with its text: a literal's tree spans its suffix, and so do its ancestors
  * (tests/fix/Driver/LiteralSuffixes). */
final class Suffixes extends SyntacticRule("Suffixes"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case t: Lit.Long if t.syntax == "1L" => Patch.replaceTree(t, "2L")
      case t: Lit.Float if t.syntax == "1.5f" => Patch.replaceTree(t, "2.5f")
      case t: Lit.Double if t.syntax == "2.0d" => Patch.replaceTree(t, "3.0d")
      case t: Defn.Val => Patch.lint(Diagnostic("val", t.syntax, t.pos, severity = LintSeverity.Warning))
    }.asPatch
