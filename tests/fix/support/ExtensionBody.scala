import teq.fix.*

/** Reports each extension group's body by its kind (tests/fix/Driver/ExtensionSingleBody). */
final class ExtensionBody extends SyntacticRule("ExtensionBody"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect { case g: Defn.ExtensionGroup => Patch.lint(Diagnostic("body", g.body.kind, g.pos)) }.asPatch
