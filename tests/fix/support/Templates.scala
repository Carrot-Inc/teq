import teq.fix.*

/** Reports each template with its text and replaces an empty body `{}` through `replaceTree`: a template spans its
  * braces, an empty body's too, and what follows `extends` (tests/fix/Driver/TemplateSpans). */
final class Templates extends SyntacticRule("Templates"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case t: Template => Patch.lint(Diagnostic("span", t.syntax, t.pos, severity = LintSeverity.Warning)) }.asPatch +
      doc.tree.collect { case t: Template if t.syntax == "{}" => Patch.replaceTree(t, "{ val inserted = 1 }") }.asPatch
