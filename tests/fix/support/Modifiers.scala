import teq.fix.*

/** Reports each modifier with its kind and text (tests/fix/Driver/AnnotatedModifiers). */
final class Modifiers extends SyntacticRule("Modifiers"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case t: Mod => Patch.lint(Diagnostic("mod", s"${t.kind} ${t.syntax}", t.pos, severity = LintSeverity.Warning)) }.asPatch
