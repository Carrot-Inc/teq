import teq.fix.*

/** Reports each annotation with its text, a type's among them (tests/fix/Driver/TypeAnnotations). */
final class Annotations extends SyntacticRule("Annotations"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case t: Mod.Annot => Patch.lint(Diagnostic("annot", t.syntax, t.pos, severity = LintSeverity.Warning)) }.asPatch
