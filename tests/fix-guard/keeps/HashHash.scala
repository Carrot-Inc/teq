package rules
import teq.fix.*
final class HashHash extends SyntacticRule("HashHash"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", ((new Object).##).toString, doc.tree.pos))
