import teq.fix.*

/** Reports the names under each `private` modifier (tests/fix/Driver/PrivateCtorQualifier). */
final class AccessNames extends SyntacticRule("AccessNames"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case m: Mod.Private => Patch.lint(Diagnostic("names", m.collect { case n: Name.Indeterminate => n.value }.mkString(","), m.pos)) }.asPatch
