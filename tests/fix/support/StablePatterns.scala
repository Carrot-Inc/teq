import teq.fix.*

/** Each case's pattern by its kind and whether it is a `Pat`, as scalameta's: a stable identifier's `Term.Name` or
  * `Term.Select` among them (tests/fix/Driver/StablePatterns). */
final class StablePatterns extends SyntacticRule("StablePatterns"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case c: Case =>
      Patch.lint(Diagnostic("pattern", c.pat.kind + " " + c.pat.isInstanceOf[Pat], c.pat.pos))
    }.asPatch
