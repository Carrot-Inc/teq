package rules
import teq.fix.*
final class ObjectHash extends SyntacticRule("ObjectHash"):
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", ((new Object).hashCode).toString, doc.tree.pos))
